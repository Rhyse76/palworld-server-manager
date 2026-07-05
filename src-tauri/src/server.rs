//! Palworld server process lifecycle.
//!
//! `PalServer.exe` is only a launcher — it spawns `PalServer-Win64-Shipping.exe`
//! and exits, so we track/stop the server by image name rather than by the PID of
//! the launcher. (Single-server assumption for now; multi-server profiles come later.)

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::CommandExt;

const SHIPPING_IMAGE: &str = "PalServer-Win64-Shipping.exe";
const LAUNCHER_IMAGE: &str = "PalServer.exe";

pub fn palserver_exe(install_dir: &Path) -> PathBuf {
    install_dir.join(LAUNCHER_IMAGE)
}

pub fn is_installed(install_dir: &Path) -> bool {
    palserver_exe(install_dir).exists()
}

/// Whether the shipping server process is currently running.
pub fn is_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {SHIPPING_IMAGE}"), "/NH"])
        .hidden()
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains(SHIPPING_IMAGE),
        Err(_) => false,
    }
}

/// Launch the dedicated server. Returns an error if it isn't installed or is
/// already running.
pub fn start(install_dir: &Path) -> Result<(), String> {
    let exe = palserver_exe(install_dir);
    if !exe.exists() {
        return Err("Server is not installed yet.".into());
    }
    if is_running() {
        return Err("Server is already running.".into());
    }

    Command::new(&exe)
        .current_dir(install_dir)
        .spawn()
        .map_err(|e| format!("failed to start server: {e}"))?;
    Ok(())
}

/// Force-stop the server process tree by image name.
pub fn stop() -> Result<(), String> {
    for image in [SHIPPING_IMAGE, LAUNCHER_IMAGE] {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/IM", image])
            .hidden()
            .output();
    }
    Ok(())
}
