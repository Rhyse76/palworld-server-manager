//! Read the server's ban list, so the Dashboard can show currently-banned players.
//! Palworld has no "list bans" REST endpoint, so we read `banlist.txt` directly from
//! the save folder. ARK: SA has no such RCON command either, so we read `BanList.txt`
//! next to the server binaries instead (confirmed accessed by the server — see
//! `docs/ark-reference.md`'s Procmon finding). Unbanning itself goes through
//! `game::live::unban` (REST for Palworld, RCON's `UnbanPlayer` for ARK) — not this
//! module, which only covers listing who's currently banned.

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::game;

fn read_list(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

fn find_palworld_banlist(install_dir: &Path) -> Option<PathBuf> {
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

const ARK_BANLIST_REL: &str = "ShooterGame/Binaries/Win64/BanList.txt";

pub fn list(install_dir: &Path) -> Result<Vec<String>, String> {
    if game::active().spec().id == "ark-sa" {
        return Ok(read_list(&install_dir.join(ARK_BANLIST_REL)));
    }
    match find_palworld_banlist(install_dir) {
        Some(path) => Ok(read_list(&path)),
        None => Ok(Vec::new()),
    }
}
