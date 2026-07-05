//! Server log capture location + tail reader.
//!
//! Palworld's dedicated server doesn't write a usable log file — the game output
//! only goes to the console shipping build's stdout. So `server::start` redirects
//! that stdout to the file returned by [`log_path`], and the UI tails it here.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// Only read the last chunk — the log can grow large over a long session.
const MAX_BYTES: u64 = 256 * 1024;

/// Where the active server's captured stdout is written.
pub fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("logs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("server.log"))
}

pub fn read_tail(app: &AppHandle) -> Result<String, String> {
    let path = log_path(app)?;
    if !path.exists() {
        return Err("No server log yet — start the server from this app to capture its output.".into());
    }

    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    let start = len.saturating_sub(MAX_BYTES);
    file.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
