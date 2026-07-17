//! Background scheduler: scheduled backups, scheduled restarts, a crash watchdog,
//! and scheduled auto-updates.
//!
//! A single OS thread wakes every 60s and runs the checks below for **every
//! profile whose server is actually running**, not just whichever one is active
//! in the UI — so e.g. a Palworld server can keep getting scheduled backups in the
//! background while the app is focused on ARK. Each profile gets its own timers/
//! supervise flag (`ProfileTimers`, keyed by profile id in `SchedulerState`), and
//! only servers this app started (tracked by that profile's `supervise` flag) are
//! auto-restarted, so it won't fight a server the user deliberately stopped.
//! Activity is written to the manager activity log, prefixed with the profile name
//! since several profiles can now log concurrently.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

use crate::{backups, discord, game, logs, server, settings, updates};
use settings::{AppConfig, ServerProfile};

const TICK: Duration = Duration::from_secs(60);

/// One profile's scheduler bookkeeping: when actions last ran, and whether we
/// expect its server to be up (so the watchdog knows a crash from an intentional
/// stop).
#[derive(Default, Clone)]
struct ProfileTimers {
    last_backup: u64,
    last_restart: u64,
    supervise: bool,
    announce_last: HashMap<String, u64>,
    last_update_check: u64,
}

#[derive(Default)]
pub struct SchedulerState {
    profiles: Mutex<HashMap<String, ProfileTimers>>,
}

impl SchedulerState {
    fn with_timers<T>(&self, id: &str, f: impl FnOnce(&mut ProfileTimers) -> T) -> T {
        let mut profiles = self.profiles.lock().unwrap();
        f(profiles.entry(id.to_string()).or_default())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Record whether a given profile's server *should* be running. Set true after the
/// app starts it, false when the app stops/graceful-shuts it down.
pub fn set_supervise(app: &AppHandle, profile_id: &str, value: bool) {
    if let Some(state) = app.try_state::<SchedulerState>() {
        state.with_timers(profile_id, |t| t.supervise = value);
    }
}

/// Start the scheduler thread. Timestamps start at "now" so scheduled actions
/// don't fire immediately; each profile's supervise flag starts true if its
/// server is already running.
pub fn start(app: AppHandle) {
    {
        let state = app.state::<SchedulerState>();
        let cfg = settings::load(&app);
        let t = now();
        for p in &cfg.profiles {
            let spec = game::by_id_or_default(&p.game).spec();
            state.with_timers(&p.id, |timers| {
                timers.last_backup = t;
                timers.last_restart = t;
                timers.last_update_check = t;
                timers.supervise = server::is_running_for(spec);
            });
        }
    }

    std::thread::spawn(move || loop {
        std::thread::sleep(TICK);
        tick(&app);
    });
}

fn tick(app: &AppHandle) {
    let cfg = settings::load(app);
    let t = now();
    for profile in &cfg.profiles {
        tick_profile(app, &cfg, profile, t);
    }
}

fn tick_profile(app: &AppHandle, cfg: &AppConfig, profile: &ServerProfile, t: u64) {
    let game = game::by_id_or_default(&profile.game);
    let spec = game.spec();
    let install_dir = Path::new(&profile.install_dir);
    let a = &profile.automation;
    let state = app.state::<SchedulerState>();
    let log = |msg: &str| logs::record(app, &format!("[{}] {msg}", profile.name));

    // Scheduled backups.
    if a.auto_backup_enabled && a.backup_interval_hours > 0.0 {
        let due = state.with_timers(&profile.id, |timers| {
            (t.saturating_sub(timers.last_backup)) as f64 >= a.backup_interval_hours * 3600.0
        });
        if due {
            state.with_timers(&profile.id, |timers| timers.last_backup = t);
            run_backup(app, profile, a.keep_backups);
        }
    }

    // Scheduled restarts. With smart_restart, wait until the server is empty.
    if a.auto_restart_enabled && a.restart_interval_hours > 0.0 {
        let due = state.with_timers(&profile.id, |timers| {
            (t.saturating_sub(timers.last_restart)) as f64 >= a.restart_interval_hours * 3600.0
        });
        if due {
            if server::is_running_for(spec) {
                let players = if a.smart_restart {
                    tauri::async_runtime::block_on(crate::game::live::players_for(game, install_dir))
                        .map(|p| p.len())
                        .unwrap_or(0)
                } else {
                    0
                };
                if a.smart_restart && players > 0 {
                    log(&format!("Scheduled restart waiting — {players} player(s) online."));
                    // Leave the timer 'due' so we retry once the server empties.
                } else {
                    state.with_timers(&profile.id, |timers| timers.last_restart = t);
                    run_restart_for(app, profile, cfg.hide_server_console, 30, "Scheduled restart");
                    return; // skip the watchdog this tick for this profile (server is cycling)
                }
            } else {
                // Not running — reset so it doesn't fire the instant it starts.
                state.with_timers(&profile.id, |timers| timers.last_restart = t);
            }
        }
    }

    // Scheduled announcements (MOTD) — only while the server is up. Announcements
    // are a single global list (`cfg.announcements`, not per-profile), so every
    // running server broadcasts the same messages on its own schedule.
    if !cfg.announcements.is_empty() && server::is_running_for(spec) {
        for ann in &cfg.announcements {
            if !ann.enabled || ann.interval_minutes <= 0.0 || ann.message.trim().is_empty() {
                continue;
            }
            let due = state.with_timers(&profile.id, |timers| {
                match timers.announce_last.get(&ann.id).copied() {
                    None => {
                        timers.announce_last.insert(ann.id.clone(), t); // prime; first send after one interval
                        false
                    }
                    Some(prev) if (t.saturating_sub(prev)) as f64 >= ann.interval_minutes * 60.0 => {
                        timers.announce_last.insert(ann.id.clone(), t);
                        true
                    }
                    _ => false,
                }
            });
            if due {
                let _ = tauri::async_runtime::block_on(crate::game::live::announce_for(
                    game,
                    install_dir,
                    &ann.message,
                ));
            }
        }
    }

    // Scheduled auto-update: check for a new server build and apply it.
    if a.auto_update_enabled && a.auto_update_interval_hours > 0.0 {
        let due = state.with_timers(&profile.id, |timers| {
            (t.saturating_sub(timers.last_update_check)) as f64 >= a.auto_update_interval_hours * 3600.0
        });
        if due {
            state.with_timers(&profile.id, |timers| timers.last_update_check = t);
            let status = updates::check_for(profile);
            if status.update_available {
                log(&format!(
                    "Server update available (build {} → {}). Applying…",
                    status.installed_build, status.latest_build
                ));
                discord::notify_for(
                    app,
                    &profile.game,
                    discord::Event::Restarting("A server update is available — applying now.".into()),
                );
                match updates::apply_for(app, profile, cfg.hide_server_console) {
                    Ok(()) => log("Server updated to the latest build."),
                    Err(e) => log(&format!("Auto-update failed: {e}")),
                }
            }
        }
    }

    // Crash watchdog: we expected it up, but it isn't.
    if a.auto_restart_on_crash {
        let supervise = state.with_timers(&profile.id, |timers| timers.supervise);
        if supervise && !server::is_running_for(spec) {
            state.with_timers(&profile.id, |timers| timers.last_restart = t);
            log("Server stopped unexpectedly — auto-restarting…");
            discord::notify_for(app, &profile.game, discord::Event::Crashed);
            match server::start_for(game, install_dir, cfg.hide_server_console, &profile.extra_launch_args) {
                Ok(()) => log("Crash watchdog: server restarted."),
                Err(e) => log(&format!("Crash watchdog: restart failed: {e}")),
            }
        }
    }
}

fn run_backup(app: &AppHandle, profile: &ServerProfile, keep: u32) {
    match backups::create_for(app, profile) {
        Ok(name) => {
            discord::notify_for(app, &profile.game, discord::Event::BackupCreated(name.clone()));
            logs::record(app, &format!("[{}] Auto-backup created: {name}", profile.name));
            prune_for(app, profile, keep);
        }
        Err(e) => logs::record(app, &format!("[{}] Auto-backup failed: {e}", profile.name)),
    }
}

fn prune_for(app: &AppHandle, profile: &ServerProfile, keep: u32) {
    if keep == 0 {
        return;
    }
    if let Ok(list) = backups::list_for(app, &profile.id) {
        // `list` is newest-first; delete everything past the keep count.
        for b in list.iter().skip(keep as usize) {
            let _ = backups::delete_for(app, &profile.id, &b.name);
        }
    }
}

/// Gracefully restart a profile's server: warn players, save + shut down via
/// REST/RCON (waiting `countdown` seconds), wait for it to exit, then start it
/// back up. Falls back to a force stop when the graceful shutdown isn't available.
/// Blocking — call off the UI thread. `label` prefixes the activity-log lines
/// (e.g. "Scheduled restart").
pub fn run_restart_for(app: &AppHandle, profile: &ServerProfile, hide_console: bool, countdown: i64, label: &str) {
    let game = game::by_id_or_default(&profile.game);
    let spec = game.spec();
    let install_dir = Path::new(&profile.install_dir);
    let log = |msg: &str| logs::record(app, &format!("[{}] {label}: {msg}", profile.name));

    let secs = countdown.max(0);
    let notice = format!("Server will restart in {secs} seconds.");
    log(&format!("warning players and shutting down ({secs}s)…"));
    discord::notify_for(app, &profile.game, discord::Event::Restarting(notice.clone()));
    if secs > 0 {
        let _ = tauri::async_runtime::block_on(crate::game::live::announce_for(game, install_dir, &notice));
    }
    let shutdown_ok =
        tauri::async_runtime::block_on(crate::game::live::shutdown_for(game, install_dir, secs, &notice)).is_ok();

    // If the graceful shutdown was accepted, wait for it to go down (up to ~2 min).
    // Otherwise (REST/RCON off) skip the wait and force-stop straight away.
    if shutdown_ok {
        for _ in 0..40 {
            if !server::is_running_for(spec) {
                break;
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    }
    let _ = server::stop_for(spec); // force-stop as a safety net / primary when live control is off
    std::thread::sleep(Duration::from_secs(2));

    match server::start_for(game, install_dir, hide_console, &profile.extra_launch_args) {
        Ok(()) => log("server started."),
        Err(e) => log(&format!("failed to start: {e}")),
    }
}
