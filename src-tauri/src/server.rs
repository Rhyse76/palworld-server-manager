//! Dedicated server process lifecycle.
//!
//! The launcher exe is often only a launcher — it spawns the real shipping process
//! and (on some versions, e.g. Palworld) exits, so we track/stop the server by
//! image name rather than by the launcher PID. Every function has a `_for` variant
//! taking an explicit game/spec (used by the automation scheduler, which supervises
//! every profile's server regardless of which one is active in the UI) and a
//! zero-arg convenience wrapper for the active game (used by UI-triggered commands,
//! which always act on whatever profile the user currently has selected).
//!
//! Known limitation: process detection matches by image name (`process_match`/
//! `process_marker`), not by PID or install dir, so it can't tell apart two
//! profiles running the *same* game (e.g. two Palworld servers) — each would see
//! the other's process as its own. Fine for the intended one-server-per-game setup
//! (automation now supervises Palworld/ARK/Enshrouded concurrently); a same-game
//! multi-profile setup would need PID tracking to fix properly.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::game;
use crate::util::CommandExt;

pub fn palserver_exe_for(game: &dyn game::Game, install_dir: &Path) -> PathBuf {
    install_dir.join(game.spec().server_launcher)
}

pub fn palserver_exe(install_dir: &Path) -> PathBuf {
    palserver_exe_for(game::active(), install_dir)
}

pub fn is_installed(install_dir: &Path) -> bool {
    palserver_exe(install_dir).exists()
}

/// Whether the shipping server process for `spec` is currently running. We require
/// the marker substring specifically, so a lingering launcher alone doesn't count.
pub fn is_running_for(spec: &game::GameSpec) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", spec.process_match, "/NH"])
        .hidden()
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains(spec.process_marker),
        Err(_) => false,
    }
}

pub fn is_running() -> bool {
    is_running_for(game::active().spec())
}

/// Launch the dedicated server via its launcher exe. By default it gets its own
/// visible console (the stable, standard method — equivalent to double-clicking the
/// launcher). With `hide_console`, it runs with a console but no visible window
/// (`CREATE_NO_WINDOW`), which still gives console builds the real console handle
/// they need to avoid crashing. `extra_args` is the profile's freeform "Extra
/// launch arguments", appended after the game's own auto-generated args — split on
/// whitespace (no quoting support).
pub fn start_for(
    game: &dyn game::Game,
    install_dir: &Path,
    hide_console: bool,
    extra_args: &str,
) -> Result<(), String> {
    let exe = palserver_exe_for(game, install_dir);
    if !exe.exists() {
        return Err("Server is not installed yet.".into());
    }
    if is_running_for(game.spec()) {
        return Err("Server is already running.".into());
    }

    let mut command = Command::new(&exe);
    command.current_dir(install_dir);
    command.args(game.launch_args(install_dir)); // empty for Palworld
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

pub fn start(install_dir: &Path, hide_console: bool, extra_args: &str) -> Result<(), String> {
    start_for(game::active(), install_dir, hide_console, extra_args)
}

/// Force-stop every server process matching `spec` (launcher + shipping variant).
pub fn stop_for(spec: &game::GameSpec) -> Result<(), String> {
    Command::new("taskkill")
        .args(["/F", "/T", "/FI", spec.process_match])
        .hidden()
        .output()
        .map_err(|e| format!("failed to stop server: {e}"))?;
    Ok(())
}

pub fn stop() -> Result<(), String> {
    stop_for(game::active().spec())
}
