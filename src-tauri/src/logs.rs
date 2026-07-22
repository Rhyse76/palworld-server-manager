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

/// `YYYY-MM-DD HH:MM:SS UTC` clock derived from the system time (no chrono
/// dependency). Previously omitted the date, which made a multi-day log
/// genuinely ambiguous about which day an entry happened on.
fn clock() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    let (y, mo, d) = civil_from_days((secs / 86400) as i64);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

/// Days-since-epoch to (year, month, day), Howard Hinnant's `civil_from_days`
/// algorithm — pure arithmetic, no chrono dependency for one date calculation.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn civil_from_days_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(30), (1970, 1, 31));
        assert_eq!(civil_from_days(31), (1970, 2, 1));
        assert_eq!(civil_from_days(365), (1971, 1, 1)); // 1970 not a leap year
        assert_eq!(civil_from_days(789), (1972, 2, 29)); // 1972 leap year
        assert_eq!(civil_from_days(790), (1972, 3, 1));
    }
}
