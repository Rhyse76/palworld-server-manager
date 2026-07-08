//! Read the Palworld server ban list. Palworld writes banned users to
//! `Pal/Saved/SaveGames/0/<worldid>/banlist.txt` (one `steam_…` userid per line).
//! The REST API has no "list bans" endpoint, so we read the file; unbanning uses
//! the existing REST `/unban` (server must be running).

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::game;

fn find_banlist(install_dir: &Path) -> Option<PathBuf> {
    // Expected location: <saves>/0/<worldid>/banlist.txt.
    let base = install_dir.join(game::active().spec().saves_rel).join("0");
    if let Ok(rd) = fs::read_dir(&base) {
        for entry in rd.flatten() {
            let candidate = entry.path().join("banlist.txt");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    // Fallback: search under Pal/Saved.
    WalkDir::new(install_dir.join("Pal").join("Saved"))
        .into_iter()
        .flatten()
        .find(|e| e.file_name() == "banlist.txt")
        .map(|e| e.path().to_path_buf())
}

pub fn list(install_dir: &Path) -> Result<Vec<String>, String> {
    let path = match find_banlist(install_dir) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect())
}
