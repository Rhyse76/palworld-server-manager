//! Read the server's ban list, so the Dashboard can show currently-banned players.
//! Palworld has no "list bans" REST endpoint, so we read `banlist.txt` directly from
//! the save folder. ARK: SA has no such RCON command either, so we read `BanList.txt`
//! next to the server binaries instead (confirmed accessed by the server — see
//! `docs/ark-reference.md`'s Procmon finding). Unbanning itself goes through
//! `game::live::unban` (REST for Palworld, RCON's `UnbanPlayer` for ARK) — not this
//! module, which only covers listing who's currently banned.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::game;

/// One banned player: `id` is what `game::live::unban` needs; `label` is a
/// human-readable name if the game's ban list includes one (empty otherwise, e.g.
/// Palworld's `banlist.txt` is bare ids with no name).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BanEntry {
    pub id: String,
    pub label: String,
}

fn read_lines(path: &Path) -> Vec<String> {
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

pub fn list(install_dir: &Path) -> Result<Vec<BanEntry>, String> {
    if game::active().spec().id == "ark-sa" {
        // Confirmed live (2026-07): each line is `<id>,<name>,<flag>` (comma-
        // separated, e.g. "0002860bec...,Rhyse,0"), not a bare id like the
        // exclusive-join/admin lists — split out the id (for unban) and name
        // (so the list stays readable) separately.
        let entries = read_lines(&install_dir.join(ARK_BANLIST_REL))
            .into_iter()
            .filter_map(|l| {
                let mut parts = l.split(',').map(str::trim);
                let id = parts.next()?.to_string();
                if id.is_empty() {
                    return None;
                }
                let label = parts.next().unwrap_or("").to_string();
                Some(BanEntry { id, label })
            })
            .collect();
        return Ok(entries);
    }
    let path = match find_palworld_banlist(install_dir) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    Ok(read_lines(&path)
        .into_iter()
        .map(|id| BanEntry { id, label: String::new() })
        .collect())
}
