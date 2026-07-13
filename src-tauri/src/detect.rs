//! Auto-detect an existing dedicated server install for the active game, so the user
//! can connect to a server they already have instead of downloading a fresh copy.
//!
//! We look in:
//!   - the app-managed default install dir,
//!   - every Steam library (found via the registry + `libraryfolders.vdf`), matched
//!     to the active game's `steam_app_id` via the `appmanifest_<id>.acf` Steam
//!     itself writes (which records the exact install folder name — more robust than
//!     guessing a naming convention),
//!   - a curated set of common manual (non-Steam-tracked) install locations,
//! and keep any folder that actually contains the active game's launcher exe.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::Serialize;
use tauri::AppHandle;

use crate::{server, settings};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstall {
    pub path: String,
    pub source: String,
    pub has_config: bool,
}

pub fn detect(app: &AppHandle) -> Vec<DetectedInstall> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    if let Ok(dir) = settings::default_install_dir(app) {
        consider(&mut out, &mut seen, dir, "App-managed");
    }

    let app_id = crate::game::active().spec().steam_app_id;
    for dir in steam_installs_for(app_id) {
        consider(&mut out, &mut seen, dir, "Steam library");
    }

    for dir in common_locations() {
        consider(&mut out, &mut seen, dir, "Found on disk");
    }

    out
}

/// Folder-name guesses for the active game's manual (non-Steam-tracked) installs.
/// Palworld's and Enshrouded's first entries are confirmed against real installs;
/// the rest are reasonable but unverified conventions — harmless if wrong, since
/// `consider` still requires the launcher exe to actually exist there.
fn folder_name_guesses() -> Vec<&'static str> {
    match crate::game::active().spec().id {
        "ark-sa" => vec!["ArkSurvivalAscendedServer", "ARK Survival Ascended Dedicated Server", "ArkAscended"],
        "enshrouded" => vec!["EnshroudedServer", "Enshrouded"],
        _ => vec!["PalworldServer", "Palworld", "PalServer", "PalworldDedicatedServer"],
    }
}

/// Fast, curated set of places a server is commonly installed manually, across every
/// fixed drive plus the user's profile. Each candidate is just a file-existence check,
/// so this stays near-instant (no recursive/full-disk scan).
fn common_locations() -> Vec<PathBuf> {
    let names = folder_name_guesses();
    let mut rel: Vec<String> = Vec::new();
    for name in &names {
        rel.push((*name).to_string());
        rel.push(format!("Games/{name}"));
        rel.push(format!("Servers/{name}"));
        rel.push(format!("SteamCMD/steamapps/common/{name}"));
        rel.push(format!("steamcmd/steamapps/common/{name}"));
        rel.push(format!("SteamLibrary/steamapps/common/{name}"));
        rel.push(format!("Steam/steamapps/common/{name}"));
        rel.push(format!("Program Files (x86)/Steam/steamapps/common/{name}"));
    }

    let mut out = Vec::new();

    // Every drive root that exists (C: through Z:).
    for letter in b'C'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if !root.exists() {
            continue;
        }
        for r in &rel {
            out.push(root.join(r));
        }
    }

    // User-profile spots.
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(profile);
        for name in &names {
            out.push(p.join(name));
            out.push(p.join("Desktop").join(name));
            out.push(p.join("Documents").join(name));
        }
    }

    out
}

fn consider(out: &mut Vec<DetectedInstall>, seen: &mut HashSet<String>, path: PathBuf, source: &str) {
    if !server::is_installed(&path) {
        return;
    }
    let key = path.to_string_lossy().to_lowercase();
    if !seen.insert(key) {
        return;
    }
    let has_config = path.join(crate::game::active().spec().config_rel).exists();
    out.push(DetectedInstall {
        path: path.to_string_lossy().to_string(),
        source: source.to_string(),
        has_config,
    });
}

/// Every install of `app_id` found across all Steam libraries, via the manifest Steam
/// itself writes (`appmanifest_<id>.acf`, which records the exact install folder name
/// it chose) rather than guessing a folder-name convention.
fn steam_installs_for(app_id: &str) -> Vec<PathBuf> {
    let manifest_name = format!("appmanifest_{app_id}.acf");
    steam_libraries()
        .into_iter()
        .filter_map(|lib| {
            let steamapps = lib.join("steamapps");
            let text = std::fs::read_to_string(steamapps.join(&manifest_name)).ok()?;
            let installdir = vdf_values(&text, "installdir").into_iter().next()?;
            Some(steamapps.join("common").join(installdir))
        })
        .collect()
}

/// Every occurrence of a `"key"    "value"` VDF line's value in `text` (used for both
/// `libraryfolders.vdf`'s repeated `"path"` entries and an appmanifest's `"installdir"`).
fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix(&needle)?;
            let val = rest.trim().trim_matches('"').replace("\\\\", "\\");
            (!val.is_empty()).then_some(val)
        })
        .collect()
}

/// All Steam library roots, including the main Steam install.
#[cfg(windows)]
fn steam_libraries() -> Vec<PathBuf> {
    let mut libs = Vec::new();
    let Some(root) = steam_root() else {
        return libs;
    };

    let vdf = root.join("steamapps").join("libraryfolders.vdf");
    libs.push(root);

    if let Ok(text) = std::fs::read_to_string(&vdf) {
        libs.extend(vdf_values(&text, "path").into_iter().map(PathBuf::from));
    }
    libs
}

#[cfg(windows)]
fn steam_root() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam")
        .ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(not(windows))]
fn steam_libraries() -> Vec<PathBuf> {
    Vec::new()
}
