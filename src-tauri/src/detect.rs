//! Auto-detect existing Palworld dedicated server installations so the user can
//! connect to a server they already have instead of downloading a fresh copy.
//!
//! We look in:
//!   - the app-managed default install dir,
//!   - every Steam library (found via the registry + `libraryfolders.vdf`),
//! and keep any folder that actually contains `PalServer.exe`.

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

    for lib in steam_libraries() {
        consider(
            &mut out,
            &mut seen,
            lib.join("steamapps").join("common").join("PalServer"),
            "Steam library",
        );
    }

    for dir in common_locations() {
        consider(&mut out, &mut seen, dir, "Found on disk");
    }

    out
}

/// Fast, curated set of places a Palworld server is commonly installed manually,
/// across every fixed drive plus the user's profile. Each candidate is just a
/// file-existence check, so this stays near-instant (no recursive/full-disk scan).
fn common_locations() -> Vec<PathBuf> {
    // Folder layouts people commonly use, relative to a drive root.
    const REL: &[&str] = &[
        "PalworldServer",
        "Palworld",
        "PalServer",
        "PalworldDedicatedServer",
        "Games/PalworldServer",
        "Games/Palworld",
        "Servers/Palworld",
        "SteamCMD/steamapps/common/PalServer",
        "steamcmd/steamapps/common/PalServer",
        "SteamLibrary/steamapps/common/PalServer",
        "Steam/steamapps/common/PalServer",
        "Program Files (x86)/Steam/steamapps/common/PalServer",
    ];

    let mut out = Vec::new();

    // Every drive root that exists (C: through Z:).
    for letter in b'C'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if !root.exists() {
            continue;
        }
        for rel in REL {
            out.push(root.join(rel));
        }
    }

    // User-profile spots.
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(profile);
        for rel in ["PalworldServer", "Desktop/PalworldServer", "Documents/PalworldServer"] {
            out.push(p.join(rel));
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
    let has_config = path
        .join("Pal")
        .join("Saved")
        .join("Config")
        .join("WindowsServer")
        .join("PalWorldSettings.ini")
        .exists();
    out.push(DetectedInstall {
        path: path.to_string_lossy().to_string(),
        source: source.to_string(),
        has_config,
    });
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
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("\"path\"") {
                let path = rest.trim().trim_matches('"').replace("\\\\", "\\");
                if !path.is_empty() {
                    libs.push(PathBuf::from(path));
                }
            }
        }
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
