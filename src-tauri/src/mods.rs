//! Manage mods for the active game.
//!
//! Two shapes, per `game::ModsKind`:
//!   * **Local files** (Palworld `.pak` in `Pal/Content/Paks/~mods/`) — we own the
//!     files; enabling/disabling renames `.pak` ⇄ `.pak.disabled` (Unreal only loads
//!     `.pak`). Whether a given mod works on a dedicated server is up to the mod.
//!   * **CurseForge id list** (ARK: SA's `ActiveMods`) — we only manage which
//!     numeric project ids are active, stored as one comma-separated config field;
//!     the game's own launcher downloads/updates the actual mod content from that
//!     list on next start (`-mods=`/`-allowcfcore`).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config;
use crate::curseforge;
use crate::game::{self, ModsKind};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    /// Base file name, e.g. "MyMod.pak" (without the .disabled suffix).
    pub name: String,
    pub enabled: bool,
    pub size_bytes: u64,
}

fn mods_dir(install_dir: &Path) -> PathBuf {
    // Games not in LocalFiles mode never call these functions (the UI branches on
    // `modsKind`), so this sentinel path simply never exists / lists nothing.
    match game::active().spec().mods {
        ModsKind::LocalFiles(rel) => install_dir.join(rel),
        _ => install_dir.join(".no-mods"),
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

// ---- CurseForge id-list mode (e.g. ARK: SA's ActiveMods) ----

fn active_ids_key() -> Result<&'static str, String> {
    match game::active().spec().mods {
        ModsKind::CurseForgeIds { active_key, .. } => Ok(active_key),
        _ => Err("This game doesn't use a CurseForge mod id list.".into()),
    }
}

fn cache_dir(install_dir: &Path) -> Result<PathBuf, String> {
    match game::active().spec().mods {
        ModsKind::CurseForgeIds { cache_dir_rel, .. } => Ok(install_dir.join(cache_dir_rel)),
        _ => Err("This game doesn't use a CurseForge mod id list.".into()),
    }
}

fn parse_ids(raw: &str) -> Vec<String> {
    raw.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()
}

pub fn list_ids(install_dir: &Path) -> Result<Vec<String>, String> {
    let key = active_ids_key()?;
    let fields = config::read(install_dir)?;
    Ok(parse_ids(&config::find(&fields, key).unwrap_or_default()))
}

pub fn add_id(install_dir: &Path, id: &str) -> Result<(), String> {
    let id = id.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err("Mod id must be numeric — copy it from the mod's CurseForge project page.".into());
    }
    let key = active_ids_key()?;
    let mut fields = config::read(install_dir)?;
    let mut ids = parse_ids(&config::find(&fields, key).unwrap_or_default());
    if ids.iter().any(|i| i == id) {
        return Ok(()); // already active
    }
    ids.push(id.to_string());
    config::upsert(&mut fields, key, &ids.join(","), "string");
    config::write(install_dir, &fields)
}

pub fn remove_id(install_dir: &Path, id: &str) -> Result<(), String> {
    let key = active_ids_key()?;
    let mut fields = config::read(install_dir)?;
    let mut ids = parse_ids(&config::find(&fields, key).unwrap_or_default());
    let before = ids.len();
    ids.retain(|i| i != id);
    if ids.len() == before {
        return Ok(()); // wasn't active
    }
    config::upsert(&mut fields, key, &ids.join(","), "string");
    config::write(install_dir, &fields)
}

/// Whether a `cache_dir_rel` entry name is this mod id's downloaded content
/// (`<mod-id>_<file-id>`, e.g. `940975_8362419`).
fn is_mod_cache_entry(entry_name: &str, id: &str) -> bool {
    entry_name.strip_prefix(id).and_then(|rest| rest.strip_prefix('_')).is_some()
}

/// Delete a mod's downloaded content from the CurseForge cache, wherever it landed
/// under `cache_dir_rel` — the leading folder is an opaque session/list-hash dir we
/// don't need to understand, so this just scans one level down from it. Does not
/// touch the active id list; pair with `remove_id` for a full uninstall.
fn delete_cached_files(install_dir: &Path, id: &str) -> Result<(), String> {
    let dir = cache_dir(install_dir)?;
    if !dir.exists() {
        return Ok(());
    }
    for session in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        if !session.path().is_dir() {
            continue;
        }
        for entry in fs::read_dir(session.path()).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().is_dir() && is_mod_cache_entry(&name, id) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(())
}

/// Full removal: drop the id from the active list and delete its cached files.
pub fn uninstall_id(install_dir: &Path, id: &str) -> Result<(), String> {
    remove_id(install_dir, id)?;
    delete_cached_files(install_dir, id)
}

/// Search CurseForge for mods of the active game, scoped to its catalog id.
pub async fn search_curseforge(api_key: &str, query: &str) -> Result<Vec<curseforge::ModResult>, String> {
    let game_id = match game::active().spec().mods {
        ModsKind::CurseForgeIds { curseforge_game_id, .. } => curseforge_game_id,
        _ => return Err("This game doesn't use a CurseForge mod id list.".into()),
    };
    curseforge::search(api_key, game_id, query).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ids_trims_and_drops_empties() {
        assert_eq!(parse_ids(""), Vec::<String>::new());
        assert_eq!(parse_ids("940975"), vec!["940975"]);
        assert_eq!(parse_ids(" 940975 , 927090,,"), vec!["940975", "927090"]);
    }

    #[test]
    fn cache_entry_matches_mod_id_by_prefix() {
        // Real observed shape: "<mod-id>_<file-id>", e.g. 940975_8362419.
        assert!(is_mod_cache_entry("940975_8362419", "940975"));
        // Must not match a different id that happens to share a numeric prefix.
        assert!(!is_mod_cache_entry("9409751_2345", "940975"));
        assert!(!is_mod_cache_entry("927090_1111", "940975"));
        // No underscore separator at all.
        assert!(!is_mod_cache_entry("940975", "940975"));
    }
}
