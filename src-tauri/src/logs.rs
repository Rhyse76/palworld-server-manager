//! Manager activity log.
//!
//! Palworld's dedicated server doesn't expose a usable log file, and its console
//! build can't have its output safely redirected. So instead of the game's log,
//! we keep our own activity log of everything the manager does — starts, stops,
//! installs, REST actions, and automation/crash events — persisted to disk and
//! streamed live to the UI via `activity-log` events.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

/// Only read the last chunk when tailing.
const MAX_BYTES: u64 = 128 * 1024;

fn activity_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("logs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("activity.log"))
}

/// Append a timestamped entry to the activity log and emit it live.
pub fn record(app: &AppHandle, msg: &str) {
    let entry = format!("{}  {msg}", clock());
    if let Ok(path) = activity_path(app) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{entry}");
        }
    }
    let _ = app.emit("activity-log", entry);
}

/// Read the tail of the activity log (empty string if none yet).
pub fn read_tail(app: &AppHandle) -> Result<String, String> {
    let path = activity_path(app)?;
    if !path.exists() {
        return Ok(String::new());
    }
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    let start = len.saturating_sub(MAX_BYTES);
    file.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// `HH:MM:SS UTC` clock derived from the system time (no chrono dependency).
fn clock() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02} UTC")
}
