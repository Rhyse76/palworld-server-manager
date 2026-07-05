//! Palworld server process lifecycle.
//!
//! `PalServer.exe` is only a launcher — it spawns the real shipping process and
//! (on some versions) exits, so we track/stop the server by image name rather than
//! by the launcher PID. The shipping process name varies between game versions:
//! it may be `PalServer-Win64-Shipping.exe` or `PalServer-Win64-Shipping-Cmd.exe`.
//! To stay robust we match the `PalServer*` image prefix and look for "Shipping".
//! (Single-server assumption for now; multi-server profiles come later.)

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::util::CommandExt;

const LAUNCHER_IMAGE: &str = "PalServer.exe";
/// The console ("-Cmd") shipping build writes the game log to stdout, unlike the
/// launcher. We launch it directly so we can capture logs.
const SHIPPING_CMD_REL: &str = "Pal/Binaries/Win64/PalServer-Win64-Shipping-Cmd.exe";
/// tasklist/taskkill filter matching every Palworld server process.
const IMAGE_FILTER: &str = "IMAGENAME eq PalServer*";

pub fn palserver_exe(install_dir: &Path) -> PathBuf {
    install_dir.join(LAUNCHER_IMAGE)
}

fn shipping_cmd_exe(install_dir: &Path) -> PathBuf {
    install_dir.join(SHIPPING_CMD_REL)
}

pub fn is_installed(install_dir: &Path) -> bool {
    palserver_exe(install_dir).exists()
}

/// Whether the shipping server process is currently running. We require the
/// "Shipping" process specifically, so a lingering launcher alone doesn't count.
pub fn is_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", IMAGE_FILTER, "/NH"])
        .hidden()
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains("Shipping"),
        Err(_) => false,
    }
}

/// Launch the dedicated server, redirecting its output to `log_path` (truncated
/// on each start). Prefers the console shipping build so logs are captured;
/// falls back to the launcher on older installs (no captured log in that case).
pub fn start(install_dir: &Path, log_path: &Path) -> Result<(), String> {
    if !is_installed(install_dir) {
        return Err("Server is not installed yet.".into());
    }
    if is_running() {
        return Err("Server is already running.".into());
    }

    let cmd_exe = shipping_cmd_exe(install_dir);
    let exe = if cmd_exe.exists() {
        cmd_exe
    } else {
        palserver_exe(install_dir)
    };

    let mut command = Command::new(&exe);
    command.current_dir(install_dir);

    // Capture stdout/stderr to the log file when we can create it.
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(file) = File::create(log_path) {
        if let Ok(err_clone) = file.try_clone() {
            command.stdout(Stdio::from(file)).stderr(Stdio::from(err_clone));
        }
    }

    command
        .spawn()
        .map_err(|e| format!("failed to start server: {e}"))?;
    Ok(())
}

/// Force-stop every Palworld server process (launcher + shipping variant).
pub fn stop() -> Result<(), String> {
    Command::new("taskkill")
        .args(["/F", "/T", "/FI", IMAGE_FILTER])
        .hidden()
        .output()
        .map_err(|e| format!("failed to stop server: {e}"))?;
    Ok(())
}
