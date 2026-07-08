//! Palworld save reading (M4 foundation).
//!
//! `.sav` files are a custom-compressed wrapper around Unreal **GVAS** data:
//!   [u32 uncompressed_len][u32 compressed_len]["PlZ" magic][u8 save_type][zlib…]
//! save_type `0x31` = single zlib, `0x32` = double zlib. Decompressed data starts
//! with the `GVAS` magic. This module decompresses and inspects it; full property
//! parsing (players/pals/inventory) builds on top.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::ZlibDecoder;
use serde::Serialize;

use crate::game;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInfo {
    pub path: String,
    pub compressed_size: u64,
    pub decompressed_size: u64,
    pub is_gvas: bool,
    pub save_type: u8,
}

/// Locate the world's `Level.sav` (`Pal/Saved/SaveGames/0/<worldid>/Level.sav`).
pub fn find_level_sav(install_dir: &Path) -> Option<PathBuf> {
    let base = install_dir.join(game::active().spec().saves_rel).join("0");
    for entry in fs::read_dir(base).ok()?.flatten() {
        let candidate = entry.path().join("Level.sav");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| format!("zlib decompress failed: {e}"))?;
    Ok(out)
}

/// Extract raw GVAS bytes from a `.sav`. Handles the older zlib-wrapped format
/// ("PlZ") and the newer "PlM" format (GVAS stored after a short prefix; small
/// saves are uncompressed). Returns (gvas, save_type, was_compressed).
pub fn read_gvas(path: &Path) -> Result<(Vec<u8>, u8, bool), String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    if data.len() < 12 || &data[8..10] != b"Pl" {
        return Err("Not a Palworld save (missing Pl magic).".into());
    }
    let save_type = data[11];
    // Newer saves embed GVAS uncompressed shortly after the header.
    if let Some(pos) = data.windows(4).take(128).position(|w| w == b"GVAS") {
        return Ok((data[pos..].to_vec(), save_type, false));
    }
    // Older zlib-wrapped format.
    let first = zlib_decompress(&data[12..])?;
    let gvas = if save_type == 0x32 {
        zlib_decompress(&first)? // double-compressed
    } else {
        first
    };
    Ok((gvas, save_type, true))
}

pub fn inspect(install_dir: &Path) -> Result<SaveInfo, String> {
    let path = find_level_sav(install_dir)
        .ok_or("No Level.sav found yet — run the server once to create a world.")?;
    let compressed_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let (gvas, save_type, _) = read_gvas(&path)?;
    Ok(SaveInfo {
        path: path.to_string_lossy().to_string(),
        compressed_size,
        decompressed_size: gvas.len() as u64,
        is_gvas: gvas.len() >= 4 && &gvas[0..4] == b"GVAS",
        save_type,
    })
}
