//! SteamCMD bootstrap + Palworld dedicated server install/update.
//!
//! The Palworld dedicated server is a free anonymous SteamCMD download,
//! Steam App ID `2394010`. We download SteamCMD if missing, then run
//! `+force_install_dir <dir> +login anonymous +app_update 2394010 validate +quit`,
//! streaming progress back to the UI via Tauri events:
//!   - `install-log`      : raw SteamCMD output lines
//!   - `install-progress` : f64 download percentage (0-100)

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use tauri::{AppHandle, Emitter};

use crate::server;
use crate::settings;
use crate::util::CommandExt;

const STEAMCMD_URL: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
const PALWORLD_APP_ID: &str = "2394010";

pub fn steamcmd_exe(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(settings::steamcmd_dir(app)?.join("steamcmd.exe"))
}

pub fn is_steamcmd_ready(app: &AppHandle) -> bool {
    steamcmd_exe(app).map(|p| p.exists()).unwrap_or(false)
}

/// Download + extract SteamCMD if it isn't already present. Returns the exe path.
pub async fn ensure_steamcmd(app: &AppHandle) -> Result<PathBuf, String> {
    let exe = steamcmd_exe(app)?;
    if exe.exists() {
        return Ok(exe);
    }

    let dir = settings::steamcmd_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let _ = app.emit("install-log", "Downloading SteamCMD...");
    let bytes = reqwest::get(STEAMCMD_URL)
        .await
        .map_err(|e| format!("failed to download SteamCMD: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("failed to read SteamCMD download: {e}"))?;

    let zip_path = dir.join("steamcmd.zip");
    fs::write(&zip_path, &bytes).map_err(|e| e.to_string())?;

    let _ = app.emit("install-log", "Extracting SteamCMD...");
    let file = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(&dir).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&zip_path);

    if !exe.exists() {
        return Err("SteamCMD extracted but steamcmd.exe was not found".into());
    }
    Ok(exe)
}

/// Install/update the server, retrying once to absorb SteamCMD's first-run
/// self-update: on a fresh SteamCMD the first invocation only updates the Steam
/// client, relaunches, and exits (code 7) without running `app_update`. Running it
/// again then downloads the server. We also treat "server ended up installed" as
/// success even if SteamCMD reports a non-zero exit.
pub fn run_update(app: &AppHandle, steamcmd: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 1..=2 {
        if attempt == 2 {
            let _ = app.emit(
                "install-log",
                "SteamCMD updated itself — running the install again...",
            );
        }
        match run_once(app, steamcmd, install_dir) {
            Ok(()) => return Ok(()),
            Err(e) => last_err = e,
        }
        // If the server binary is present, the install actually succeeded.
        if server::is_installed(install_dir) {
            return Ok(());
        }
    }
    Err(last_err)
}

/// A single SteamCMD `app_update` run, streaming output as events. Blocks until
/// SteamCMD exits, so callers run it off the async runtime.
fn run_once(app: &AppHandle, steamcmd: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(install_dir).map_err(|e| e.to_string())?;

    let mut child = Command::new(steamcmd)
        .arg("+force_install_dir")
        .arg(install_dir)
        .arg("+login")
        .arg("anonymous")
        .arg("+app_update")
        .arg(PALWORLD_APP_ID)
        .arg("validate")
        .arg("+quit")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .hidden()
        .spawn()
        .map_err(|e| format!("failed to start SteamCMD: {e}"))?;

    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(pct) = parse_progress(&line) {
                let _ = app.emit("install-progress", pct);
            }
            let _ = app.emit("install-log", line);
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        let _ = app.emit("install-log", "SteamCMD finished.");
        Ok(())
    } else {
        Err(format!(
            "SteamCMD exited with code {}",
            status.code().unwrap_or(-1)
        ))
    }
}

/// Parse the download percentage out of a SteamCMD progress line, e.g.
/// `Update state (0x61) downloading, progress: 42.13 (123 / 456)`.
fn parse_progress(line: &str) -> Option<f64> {
    let idx = line.find("progress: ")? + "progress: ".len();
    let rest = &line[idx..];
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse::<f64>().ok()
}
