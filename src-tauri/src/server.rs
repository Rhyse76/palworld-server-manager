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

/// PID of the running shipping server process for `spec`, if any.
fn running_pid(spec: &game::GameSpec) -> Option<u32> {
    let out = Command::new("tasklist")
        .args(["/FI", spec.process_match, "/NH", "/FO", "CSV"])
        .hidden()
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.contains(spec.process_marker))
        .and_then(|l| l.split(',').nth(1))
        .and_then(|pid| pid.trim_matches('"').parse().ok())
}

/// Ask the server to shut itself down by sending Ctrl+Break to its console, then
/// wait (up to ~20s) for it to exit. The signal goes to every process attached to
/// that console — including whoever sends it — so it's sent from a throwaway
/// PowerShell helper that attaches to the server's console, never from this app.
fn request_console_exit(spec: &game::GameSpec) {
    let Some(pid) = running_pid(spec) else { return };
    let script = format!(
        "$k = Add-Type -Name K -Namespace W -PassThru -MemberDefinition '         [DllImport(\"kernel32.dll\")]public static extern bool FreeConsole();         [DllImport(\"kernel32.dll\")]public static extern bool AttachConsole(uint p);         [DllImport(\"kernel32.dll\")]public static extern bool GenerateConsoleCtrlEvent(uint e,uint g);';          [void]$k::FreeConsole();          if ($k::AttachConsole({pid})) {{ [void]$k::GenerateConsoleCtrlEvent(1, 0) }}"
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .hidden()
        .output();
    for _ in 0..20 {
        if !is_running_for(spec) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// Stop every server process matching `spec` (launcher + shipping variant). Games
/// that exit on a console Ctrl+Break are asked to shut down first; the force-kill
/// then only cleans up whatever is left.
pub fn stop_for(spec: &game::GameSpec) -> Result<(), String> {
    if spec.exits_on_console_break {
        request_console_exit(spec);
    }
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
