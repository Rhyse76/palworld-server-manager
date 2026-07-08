//! Manage Palworld `.pak` mods in `Pal/Content/Paks/~mods/`.
//!
//! Enabling/disabling is done by renaming `.pak` ⇄ `.pak.disabled` (Unreal only
//! loads `.pak`). We manage the files; whether a given mod works on a dedicated
//! server is up to the mod itself.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::game;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    /// Base file name, e.g. "MyMod.pak" (without the .disabled suffix).
    pub name: String,
    pub enabled: bool,
    pub size_bytes: u64,
}

pub fn mods_dir(install_dir: &Path) -> PathBuf {
    // Games without a mods dir advertise `mods_rel: None`; the UI hides the Mods
    // page for them, so this sentinel path simply never exists / lists nothing.
    match game::active().spec().mods_rel {
        Some(rel) => install_dir.join(rel),
        None => install_dir.join(".no-mods"),
    }
}

fn sanitize(name: &str) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Invalid mod name.".into());
    }
    Ok(())
}

pub fn list(install_dir: &Path) -> Result<Vec<ModInfo>, String> {
    let dir = mods_dir(install_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().to_string();
        let (name, enabled) = if let Some(base) = fname.strip_suffix(".pak.disabled") {
            (format!("{base}.pak"), false)
        } else if fname.ends_with(".pak") {
            (fname.clone(), true)
        } else {
            continue;
        };
        out.push(ModInfo {
            name,
            enabled,
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn set_enabled(install_dir: &Path, name: &str, enabled: bool) -> Result<(), String> {
    sanitize(name)?;
    let dir = mods_dir(install_dir);
    let on = dir.join(name);
    let off = dir.join(format!("{name}.disabled"));
    if enabled && off.exists() {
        fs::rename(&off, &on).map_err(|e| e.to_string())?;
    } else if !enabled && on.exists() {
        fs::rename(&on, &off).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn install(install_dir: &Path, src: &Path) -> Result<String, String> {
    let fname = src
        .file_name()
        .ok_or("Invalid file.")?
        .to_string_lossy()
        .to_string();
    if !fname.to_lowercase().ends_with(".pak") {
        return Err("Please choose a .pak mod file.".into());
    }
    let dir = mods_dir(install_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::copy(src, dir.join(&fname)).map_err(|e| e.to_string())?;
    Ok(fname)
}

pub fn remove(install_dir: &Path, name: &str) -> Result<(), String> {
    sanitize(name)?;
    let dir = mods_dir(install_dir);
    let _ = fs::remove_file(dir.join(name));
    let _ = fs::remove_file(dir.join(format!("{name}.disabled")));
    Ok(())
}
