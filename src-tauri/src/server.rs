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

use crate::game;
use crate::util::CommandExt;

pub fn palserver_exe(install_dir: &Path) -> PathBuf {
    install_dir.join(game::active().spec().server_launcher)
}

pub fn is_installed(install_dir: &Path) -> bool {
    palserver_exe(install_dir).exists()
}

/// Whether the shipping server process is currently running. We require the
/// "Shipping" process specifically, so a lingering launcher alone doesn't count.
pub fn is_running() -> bool {
    let spec = game::active().spec();
    let output = Command::new("tasklist")
        .args(["/FI", spec.process_match, "/NH"])
        .hidden()
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains(spec.process_marker),
        Err(_) => false,
    }
}

/// Launch the dedicated server via the `PalServer.exe` launcher. By default it
/// gets its own visible console (the stable, standard method — equivalent to
/// double-clicking the launcher). With `hide_console`, it runs with a console
/// but no visible window (`CREATE_NO_WINDOW`), which still gives the console
/// build the real console handle it needs to avoid crashing. `extra_args` is the
/// user's freeform "Extra launch arguments" from the profile, appended after the
/// game's own auto-generated args — split on whitespace (no quoting support).
pub fn start(install_dir: &Path, hide_console: bool, extra_args: &str) -> Result<(), String> {
    let exe = palserver_exe(install_dir);
    if !exe.exists() {
        return Err("Server is not installed yet.".into());
    }
    if is_running() {
        return Err("Server is already running.".into());
    }

    let mut command = Command::new(&exe);
    command.current_dir(install_dir);
    command.args(game::active().launch_args(install_dir)); // empty for Palworld
    command.args(extra_args.split_whitespace());
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
        .args(["/F", "/T", "/FI", game::active().spec().process_match])
        .hidden()
        .output()
        .map_err(|e| format!("failed to stop server: {e}"))?;
    Ok(())
}
