//! Backup/restore of the world save folder (`Pal/Saved/SaveGames`).
//!
//! Backups are timestamped zip archives stored under the app data dir. Each archive
//! contains a top-level `SaveGames/` directory, so restoring extracts straight back
//! into `Pal/Saved/`.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

use crate::game;
use crate::server;
use crate::settings;
use crate::util::CommandExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified: u64,
}

pub fn backups_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn savegames_dir(install_dir: &Path) -> PathBuf {
    install_dir.join(game::active().spec().saves_rel)
}

pub fn create(app: &AppHandle, install_dir: &Path) -> Result<String, String> {
    let src = savegames_dir(install_dir);
    if !src.exists() {
        return Err("No SaveGames folder found yet — run the server once to create a world.".into());
    }

    let name = format!("save-{}.zip", timestamp());
    let dest = backups_dir(app)?.join(&name);
    zip_dir(&src, &dest).map_err(|e| format!("backup failed: {e}"))?;

    // Also copy to the off-site mirror folder, if configured.
    let mirror = settings::load(app).backup_mirror_dir;
    let mirror = mirror.trim();
    if !mirror.is_empty() {
        let mdir = std::path::Path::new(mirror);
        let _ = fs::create_dir_all(mdir);
        if mdir.is_dir() {
            let _ = fs::copy(&dest, mdir.join(&name));
        }
    }
    Ok(name)
}

pub fn list(app: &AppHandle) -> Result<Vec<BackupInfo>, String> {
    let dir = backups_dir(app)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push(BackupInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            size_bytes: meta.len(),
            modified,
        });
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(out)
}

pub fn restore(app: &AppHandle, install_dir: &Path, name: &str) -> Result<(), String> {
    if server::is_running() {
        return Err("Stop the server before restoring a backup.".into());
    }
    let safe = sanitize(name)?;
    let zip_path = backups_dir(app)?.join(&safe);
    if !zip_path.exists() {
        return Err("Backup not found.".into());
    }

    // Archive stores paths as `SaveGames/...`, so extract into `Pal/Saved`.
    let target = savegames_dir(install_dir)
        .parent()
        .ok_or("invalid install path")?
        .to_path_buf();
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;

    let file = File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(&target).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete(app: &AppHandle, name: &str) -> Result<(), String> {
    let safe = sanitize(name)?;
    let path = backups_dir(app)?.join(&safe);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn open_folder(app: &AppHandle) -> Result<(), String> {
    let dir = backups_dir(app)?;
    std::process::Command::new("explorer")
        .arg(dir)
        .hidden()
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Reject path separators so a backup name can't escape the backups dir.
fn sanitize(name: &str) -> Result<String, String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Invalid backup name.".into());
    }
    Ok(name.to_string())
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn zip_dir(src: &Path, dest_zip: &Path) -> io::Result<()> {
    let file = File::create(dest_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    // `base` is the parent so entries are stored as `SaveGames/...`.
    let base = src.parent().unwrap_or(src);

    for entry in WalkDir::new(src).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let rel = match path.strip_prefix(base) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let name = rel.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }
        if path.is_dir() {
            zip.add_directory(format!("{name}/"), options)?;
        } else {
            zip.start_file(name, options)?;
            let mut f = File::open(path)?;
            io::copy(&mut f, &mut zip)?;
        }
    }
    zip.finish()?;
    Ok(())
}
