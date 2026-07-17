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

/// Root folder all profiles' backups live under — `backups_dir` (per-profile) is
/// what everything actually uses; this is only for the legacy migration below.
fn backups_root(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("backups");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// A given profile's own backup folder — each profile gets its own subfolder
/// (keyed by profile id, stable across renames) so different games' backups never
/// mix and a restore can never land in the wrong game's save folder.
pub fn backups_dir_for(app: &AppHandle, profile_id: &str) -> Result<PathBuf, String> {
    let dir = backups_root(app)?.join(profile_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// The active profile's own backup folder — see `backups_dir_for`.
pub fn backups_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let id = settings::active_profile(app)
        .map(|p| p.id)
        .ok_or("No active server profile.")?;
    backups_dir_for(app, &id)
}

/// One-time migration: backups used to live loose directly under `backups/` shared
/// across every profile/game. Move any still sitting there into the first Palworld
/// profile's folder (that's what pre-multi-game backups always were) so they don't
/// silently vanish from the list. No-op once the root has no more loose zips.
pub fn migrate_legacy(app: &AppHandle) {
    let Ok(root) = backups_root(app) else { return };
    let Ok(entries) = fs::read_dir(&root) else { return };
    let loose: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("zip"))
        .collect();
    if loose.is_empty() {
        return;
    }
    let cfg = settings::load(app);
    let Some(palworld_id) = cfg.profiles.iter().find(|p| p.game == "palworld").map(|p| p.id.clone()) else {
        return; // no Palworld profile to migrate into — leave them, nothing lost
    };
    let dest_dir = root.join(&palworld_id);
    if fs::create_dir_all(&dest_dir).is_err() {
        return;
    }
    for src in loose {
        if let Some(name) = src.file_name() {
            let _ = fs::rename(&src, dest_dir.join(name));
        }
    }
}

/// Subfolder name for the off-site mirror, so different profiles' backups land in
/// clearly separate folders under one shared mirror root. Human-readable (the
/// profile's current name) since this is what the user actually browses in e.g.
/// OneDrive — unlike the local `backups_dir`, it's fine if renaming a profile
/// starts a new folder here.
fn mirror_subfolder_name(profile_name: &str) -> String {
    let cleaned: String = profile_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() { "Server".into() } else { cleaned.to_string() }
}

fn savegames_dir_for(spec: &game::GameSpec, install_dir: &Path) -> PathBuf {
    install_dir.join(spec.saves_rel)
}

fn savegames_dir(install_dir: &Path) -> PathBuf {
    savegames_dir_for(game::active().spec(), install_dir)
}

/// Create a backup for a specific profile, regardless of which one is active in the
/// UI — used by the automation scheduler, which supervises every profile's server.
/// Everything comes from `profile` directly; doesn't touch `game::active()`.
pub fn create_for(app: &AppHandle, profile: &settings::ServerProfile) -> Result<String, String> {
    let install_dir = Path::new(&profile.install_dir);
    let spec = game::by_id_or_default(&profile.game).spec();
    let src = savegames_dir_for(spec, install_dir);
    if !src.exists() {
        return Err("No SaveGames folder found yet — run the server once to create a world.".into());
    }

    let name = format!("save-{}.zip", timestamp());
    let dest = backups_dir_for(app, &profile.id)?.join(&name);
    zip_dir(&src, &dest).map_err(|e| format!("backup failed: {e}"))?;

    let mirror = settings::load(app).backup_mirror_dir;
    let mirror = mirror.trim();
    if !mirror.is_empty() {
        let mdir = std::path::Path::new(mirror).join(mirror_subfolder_name(&profile.name));
        let _ = fs::create_dir_all(&mdir);
        if mdir.is_dir() {
            let _ = fs::copy(&dest, mdir.join(&name));
        }
    }
    Ok(name)
}

pub fn create(app: &AppHandle, install_dir: &Path) -> Result<String, String> {
    let src = savegames_dir(install_dir);
    if !src.exists() {
        return Err("No SaveGames folder found yet — run the server once to create a world.".into());
    }

    let name = format!("save-{}.zip", timestamp());
    let dest = backups_dir(app)?.join(&name);
    zip_dir(&src, &dest).map_err(|e| format!("backup failed: {e}"))?;

    // Also copy to the off-site mirror folder, if configured — in this profile's
    // own subfolder so different games' backups don't mix in the mirror either.
    let mirror = settings::load(app).backup_mirror_dir;
    let mirror = mirror.trim();
    if !mirror.is_empty() {
        let profile_name = settings::active_profile(app).map(|p| p.name).unwrap_or_default();
        let mdir = std::path::Path::new(mirror).join(mirror_subfolder_name(&profile_name));
        let _ = fs::create_dir_all(&mdir);
        if mdir.is_dir() {
            let _ = fs::copy(&dest, mdir.join(&name));
        }
    }
    Ok(name)
}

pub fn list_for(app: &AppHandle, profile_id: &str) -> Result<Vec<BackupInfo>, String> {
    let dir = backups_dir_for(app, profile_id)?;
    list_dir(&dir)
}

pub fn list(app: &AppHandle) -> Result<Vec<BackupInfo>, String> {
    let dir = backups_dir(app)?;
    list_dir(&dir)
}

fn list_dir(dir: &Path) -> Result<Vec<BackupInfo>, String> {
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

pub fn delete_for(app: &AppHandle, profile_id: &str, name: &str) -> Result<(), String> {
    let safe = sanitize(name)?;
    let path = backups_dir_for(app, profile_id)?.join(&safe);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
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
