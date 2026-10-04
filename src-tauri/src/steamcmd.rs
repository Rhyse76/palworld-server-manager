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
/// success even if SteamCMD reports a non-zero exit — unless SteamCMD said outright
/// that the update job failed.
pub fn run_update(app: &AppHandle, steamcmd: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    run_update_for(app, steamcmd, install_dir, crate::game::active().spec().steam_app_id)
}

/// Same as `run_update`, but for an explicit Steam app id — used by the automation
/// scheduler's auto-update check, which acts on a specific profile regardless of
/// which game is active in the UI.
pub fn run_update_for(
    app: &AppHandle,
    steamcmd: &PathBuf,
    install_dir: &PathBuf,
    app_id: &str,
) -> Result<(), String> {
    let mut last_err = String::new();
    let mut attempts_left = 2;
    let mut reset_install_state = false;
    while attempts_left > 0 {
        attempts_left -= 1;
        let log_mark = content_log_len(steamcmd);
        match run_once(app, steamcmd, install_dir, app_id) {
            Ok(()) => return Ok(()),
            Err(RunError::UpdateJob(line)) => {
                // An existing install that SteamCMD could not update. If the cause is
                // the stale-manifest case below, forget the old version and go again
                // (once); otherwise it's a real failure, not masked by "is installed".
                if !reset_install_state
                    && old_manifest_denied(&content_log_since(steamcmd, log_mark))
                    && forget_installed_version(install_dir, app_id)
                {
                    reset_install_state = true;
                    attempts_left += 1;
                    let _ = app.emit(
                        "install-log",
                        "This server was installed by a different SteamCMD, so a normal update isn't possible — re-verifying all files against the new version instead...",
                    );
                    continue;
                }
                return Err(format!(
                    "SteamCMD could not update the server ({line}). Make sure the server is stopped and try again."
                ));
            }
            Err(RunError::Exit(e)) => last_err = e,
        }
        // If the server binary is present, the install actually succeeded.
        if server::is_installed(install_dir) {
            return Ok(());
        }
        if attempts_left > 0 {
            let _ = app.emit(
                "install-log",
                "SteamCMD updated itself — running the install again...",
            );
        }
    }
    Err(last_err)
}

fn content_log_path(steamcmd: &PathBuf) -> PathBuf {
    steamcmd.with_file_name("logs").join("content_log.txt")
}

fn content_log_len(steamcmd: &PathBuf) -> u64 {
    fs::metadata(content_log_path(steamcmd)).map(|m| m.len()).unwrap_or(0)
}

/// What SteamCMD appended to its content log since `mark` (i.e. during one run).
fn content_log_since(steamcmd: &PathBuf, mark: u64) -> String {
    let bytes = fs::read(content_log_path(steamcmd)).unwrap_or_default();
    let start = (mark as usize).min(bytes.len());
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

/// Whether an update failed because Steam refused the *installed* version's manifest.
/// SteamCMD needs that old manifest to work out what changed, normally has it cached
/// from doing the install itself, and can't fetch it anonymously otherwise — so this
/// is what updating a server installed by some other SteamCMD looks like (seen live).
fn old_manifest_denied(content_log: &str) -> bool {
    content_log
        .lines()
        .any(|l| l.contains("Failed to get manifest request code") && l.contains("Access Denied"))
}

/// Set aside SteamCMD's record of which version is installed in `install_dir`, so the
/// next run verifies the files on disk against the current version instead of needing
/// the old manifest. Game files, saves and config are untouched. Returns whether there
/// was a record to set aside.
fn forget_installed_version(install_dir: &PathBuf, app_id: &str) -> bool {
    let manifest = install_dir.join("steamapps").join(format!("appmanifest_{app_id}.acf"));
    let stale = manifest.with_extension("acf.stale");
    let _ = fs::remove_file(&stale);
    manifest.exists() && fs::rename(&manifest, &stale).is_ok()
}

enum RunError {
    /// SteamCMD ran the update job and reported that it failed (its own error line).
    UpdateJob(String),
    /// SteamCMD exited non-zero without saying the update job failed.
    Exit(String),
}

/// A single SteamCMD `app_update` run, streaming output as events. Blocks until
/// SteamCMD exits, so callers run it off the async runtime.
fn run_once(app: &AppHandle, steamcmd: &PathBuf, install_dir: &PathBuf, app_id: &str) -> Result<(), RunError> {
    fs::create_dir_all(install_dir).map_err(|e| RunError::Exit(e.to_string()))?;

    let mut child = Command::new(steamcmd)
        .arg("+force_install_dir")
        .arg(install_dir)
        .arg("+login")
        .arg("anonymous")
        .arg("+app_update")
        .arg(app_id)
        .arg("validate")
        .arg("+quit")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .hidden()
        .spawn()
        .map_err(|e| RunError::Exit(format!("failed to start SteamCMD: {e}")))?;

    let mut update_job_error = None;
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(pct) = parse_progress(&line) {
                let _ = app.emit("install-progress", pct);
            }
            if is_update_job_error(&line) {
                update_job_error = Some(line.trim().to_string());
            }
            let _ = app.emit("install-log", line);
        }
    }

    let status = child.wait().map_err(|e| RunError::Exit(e.to_string()))?;
    if let Some(line) = update_job_error {
        return Err(RunError::UpdateJob(line));
    }
    if status.success() {
        let _ = app.emit("install-log", "SteamCMD finished.");
        Ok(())
    } else {
        Err(RunError::Exit(format!(
            "SteamCMD exited with code {}",
            status.code().unwrap_or(-1)
        )))
    }
}

/// SteamCMD's own "the update job failed" line, e.g.
/// `Error! App '4019830' state is 0x6 after update job.`
fn is_update_job_error(line: &str) -> bool {
    line.contains("Error! App") && line.contains("after update job")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_the_update_job_error_line() {
        assert!(is_update_job_error("Error! App '4019830' state is 0x6 after update job."));
        assert!(!is_update_job_error(" Update state (0x61) downloading, progress: 42.13 (123 / 456)"));
        assert!(!is_update_job_error("Success! App '4019830' fully installed."));
    }

    #[test]
    fn recognizes_a_denied_old_manifest_in_the_content_log() {
        // Lines from a real failed update of a server installed by another SteamCMD.
        let log = "\
[2026-10-04 13:50:11] cache1.steamcontent.com/depot/4019831/manifest/5222142871788076515/5/1302 - manifest request received 200 (OK) HTTP response
[2026-10-04 13:50:11] CDepotDownloadMgr::BYldRequestDepotManifest(App: 4019830, Depot: 4019831, Manifest: 8698681730846329652, branch: ): Failed to get manifest request code, 'Access Denied'
[2026-10-04 13:50:11] AppID 4019830 update canceled : Failed downloading 1 manifests (No connection)
";
        assert!(old_manifest_denied(log));
        // A plain offline failure must not trigger the recovery.
        assert!(!old_manifest_denied(
            "[2026-10-04 13:50:11] AppID 1 update canceled : Failed downloading 1 manifests (No connection)\n"
        ));
    }

    #[test]
    fn forgetting_the_installed_version_sets_the_manifest_aside() {
        let dir = std::env::temp_dir().join(format!("pwsm-steamcmd-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("steamapps")).unwrap();
        assert!(!forget_installed_version(&dir, "4019830")); // nothing to set aside

        let manifest = dir.join("steamapps/appmanifest_4019830.acf");
        fs::write(&manifest, "x").unwrap();
        assert!(forget_installed_version(&dir, "4019830"));
        assert!(!manifest.exists());
        assert!(dir.join("steamapps/appmanifest_4019830.acf.stale").exists());

        fs::remove_dir_all(&dir).ok();
    }
}
