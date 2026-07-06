//! Detect and apply new Palworld dedicated-server versions.
//!
//! We compare the **installed build id** (from `steamapps/appmanifest_2394010.acf`)
//! against the **latest public build id** (from the community steamcmd.net API) so
//! we can tell an update is available *without* stopping the server.

use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;

use crate::{rest, server, settings, steamcmd};

const APP_ID: &str = "2394010";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub installed_build: String,
    pub latest_build: String,
    pub update_available: bool,
    /// True only if both build ids were determined.
    pub checked: bool,
}

/// Read the installed build id out of the Steam app manifest.
fn installed_build(install_dir: &Path) -> Option<String> {
    let acf = install_dir
        .join("steamapps")
        .join(format!("appmanifest_{APP_ID}.acf"));
    let text = fs::read_to_string(acf).ok()?;
    let line = text.lines().find(|l| l.contains("\"buildid\""))?;
    line.split('"').nth(3).map(|s| s.to_string())
}

/// Latest public build id from the steamcmd.net info API.
fn latest_build() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .ok()?;
    let json: serde_json::Value = client
        .get(format!("https://api.steamcmd.net/v1/info/{APP_ID}"))
        .send()
        .ok()?
        .json()
        .ok()?;
    json["data"][APP_ID]["depots"]["branches"]["public"]["buildid"]
        .as_str()
        .map(|s| s.to_string())
}

pub fn check(app: &AppHandle) -> UpdateStatus {
    let installed = settings::install_dir(app)
        .ok()
        .and_then(|d| installed_build(&d))
        .unwrap_or_default();
    let latest = latest_build().unwrap_or_default();
    let checked = !installed.is_empty() && !latest.is_empty();
    UpdateStatus {
        update_available: checked && installed != latest,
        installed_build: installed,
        latest_build: latest,
        checked,
    }
}

/// Gracefully update the server: warn + save + stop (if running), run the
/// SteamCMD update, then restart it if it had been running. Blocks; call off the
/// UI thread.
pub fn apply(app: &AppHandle) -> Result<(), String> {
    let dir = settings::install_dir(app)?;
    let was_running = server::is_running();

    if was_running {
        let _ = tauri::async_runtime::block_on(rest::announce(
            &dir,
            "Server is updating and will restart shortly.",
        ));
        let _ = tauri::async_runtime::block_on(rest::save(&dir));
        let _ = server::stop();
        for _ in 0..30 {
            if !server::is_running() {
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    let steamcmd = tauri::async_runtime::block_on(steamcmd::ensure_steamcmd(app))?;
    steamcmd::run_update(app, &steamcmd, &dir)?;

    if was_running {
        let hide = settings::load(app).hide_server_console;
        server::start(&dir, hide)?;
    }
    Ok(())
}
