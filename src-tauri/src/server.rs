//! Palworld server process lifecycle.
//!
//! `PalServer.exe` is only a launcher — it spawns the real shipping process and
//! (on some versions) exits, so we track/stop the server by image name rather than
//! by the launcher PID. The shipping process name varies between game versions:
//! it may be `PalServer-Win64-Shipping.exe` or `PalServer-Win64-Shipping-Cmd.exe`.
//! To stay robust we match the `PalServer*` image prefix and look for "Shipping".
//! (Single-server assumption for now; multi-server profiles come later.)

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::CommandExt;

const LAUNCHER_IMAGE: &str = "PalServer.exe";
/// tasklist/taskkill filter matching every Palworld server process.
const IMAGE_FILTER: &str = "IMAGENAME eq PalServer*";

pub fn palserver_exe(install_dir: &Path) -> PathBuf {
    install_dir.join(LAUNCHER_IMAGE)
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

/// Launch the dedicated server via the `PalServer.exe` launcher. By default it
/// gets its own visible console (the stable, standard method — equivalent to
/// double-clicking the launcher). With `hide_console`, it runs with a console
/// but no visible window (`CREATE_NO_WINDOW`), which still gives the console
/// build the real console handle it needs to avoid crashing.
pub fn start(install_dir: &Path, hide_console: bool) -> Result<(), String> {
    let exe = palserver_exe(install_dir);
    if !exe.exists() {
        return Err("Server is not installed yet.".into());
    }
    if is_running() {
        return Err("Server is already running.".into());
    }

    let mut command = Command::new(&exe);
    command.current_dir(install_dir);
    if hide_console {
        command.hidden();
    } else {
        command.new_console();
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
