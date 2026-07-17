//! Detect and apply new dedicated-server versions.
//!
//! We compare the **installed build id** (from `steamapps/appmanifest_<app_id>.acf`)
//! against the **latest public build id** (from the community steamcmd.net API) so
//! we can tell an update is available *without* stopping the server.
//!
//! Every function has a `_for` variant taking an explicit profile (used by the
//! automation scheduler, which checks every profile's server regardless of which
//! one is active in the UI) and a zero-arg convenience wrapper for the active
//! profile (used by the UI's "Check now" button).

use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;

use crate::{game, server, settings, steamcmd};

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
fn installed_build(install_dir: &Path, app_id: &str) -> Option<String> {
    let acf = install_dir
        .join("steamapps")
        .join(format!("appmanifest_{app_id}.acf"));
    let text = fs::read_to_string(acf).ok()?;
    let line = text.lines().find(|l| l.contains("\"buildid\""))?;
    line.split('"').nth(3).map(|s| s.to_string())
}

/// Latest public build id from the steamcmd.net info API.
fn latest_build(app_id: &str) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .ok()?;
    let json: serde_json::Value = client
        .get(format!("https://api.steamcmd.net/v1/info/{app_id}"))
        .send()
        .ok()?
        .json()
        .ok()?;
    json["data"][app_id]["depots"]["branches"]["public"]["buildid"]
        .as_str()
        .map(|s| s.to_string())
}

fn check_impl(install_dir: Option<&Path>, app_id: &str) -> UpdateStatus {
    let installed = install_dir.and_then(|d| installed_build(d, app_id)).unwrap_or_default();
    let latest = latest_build(app_id).unwrap_or_default();
    let checked = !installed.is_empty() && !latest.is_empty();
    UpdateStatus {
        update_available: checked && installed != latest,
        installed_build: installed,
        latest_build: latest,
        checked,
    }
}

pub fn check(app: &AppHandle) -> UpdateStatus {
    let install_dir = settings::install_dir(app).ok();
    check_impl(install_dir.as_deref(), game::active().spec().steam_app_id)
}

pub fn check_for(profile: &settings::ServerProfile) -> UpdateStatus {
    let spec = game::by_id_or_default(&profile.game).spec();
    check_impl(Some(Path::new(&profile.install_dir)), spec.steam_app_id)
}

/// Gracefully update the server: warn + save + stop (if running), run the
/// SteamCMD update, then restart it if it had been running. Blocks; call off the
/// UI thread.
fn apply_impl(
    app: &AppHandle,
    game: &dyn game::Game,
    install_dir: &Path,
    hide_console: bool,
    extra_args: &str,
) -> Result<(), String> {
    let spec = game.spec();
    let was_running = server::is_running_for(spec);

    if was_running {
        let _ = tauri::async_runtime::block_on(crate::game::live::announce_for(
            game,
            install_dir,
            "Server is updating and will restart shortly.",
        ));
        let _ = tauri::async_runtime::block_on(crate::game::live::save_for(game, install_dir));
        let _ = server::stop_for(spec);
        for _ in 0..30 {
            if !server::is_running_for(spec) {
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    let steamcmd = tauri::async_runtime::block_on(steamcmd::ensure_steamcmd(app))?;
    steamcmd::run_update_for(app, &steamcmd, &install_dir.to_path_buf(), spec.steam_app_id)?;

    if was_running {
        server::start_for(game, install_dir, hide_console, extra_args)?;
    }
    Ok(())
}

pub fn apply_for(app: &AppHandle, profile: &settings::ServerProfile, hide_console: bool) -> Result<(), String> {
    let game = game::by_id_or_default(&profile.game);
    let dir = Path::new(&profile.install_dir);
    apply_impl(app, game, dir, hide_console, &profile.extra_launch_args)
}
