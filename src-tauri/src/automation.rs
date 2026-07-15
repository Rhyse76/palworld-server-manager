//! Background scheduler: scheduled backups, scheduled restarts, and a crash
//! watchdog that auto-restarts the server when it dies unexpectedly.
//!
//! A single OS thread wakes every 60s. It only supervises servers this app
//! started (tracked by the `supervise` flag), so it won't fight a server the
//! user deliberately stopped. Activity is written to the manager activity log.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

use crate::{backups, discord, logs, server, settings, updates};

const TICK: Duration = Duration::from_secs(60);

/// Scheduler bookkeeping: when actions last ran, and whether we expect the
/// server to be up (so the watchdog knows a crash from an intentional stop).
#[derive(Default)]
pub struct SchedulerState {
    last_backup: Mutex<u64>,
    last_restart: Mutex<u64>,
    supervise: Mutex<bool>,
    announce_last: Mutex<HashMap<String, u64>>,
    last_update_check: Mutex<u64>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Record whether the server *should* be running. Set true after the app starts
/// it, false when the app stops/graceful-shuts it down.
pub fn set_supervise(app: &AppHandle, value: bool) {
    if let Some(state) = app.try_state::<SchedulerState>() {
        *state.supervise.lock().unwrap() = value;
    }
}

/// Start the scheduler thread. Timestamps start at "now" so scheduled actions
/// don't fire immediately; supervise starts true if a server is already running.
pub fn start(app: AppHandle) {
    {
        let state = app.state::<SchedulerState>();
        let t = now();
        *state.last_backup.lock().unwrap() = t;
        *state.last_restart.lock().unwrap() = t;
        *state.last_update_check.lock().unwrap() = t;
        *state.supervise.lock().unwrap() = server::is_running();
    }

    std::thread::spawn(move || loop {
        std::thread::sleep(TICK);
        tick(&app);
    });
}

fn tick(app: &AppHandle) {
    let cfg = settings::load(app);
    let a = settings::active_automation(app);
    let state = app.state::<SchedulerState>();
    let t = now();

    // Scheduled backups.
    if a.auto_backup_enabled && a.backup_interval_hours > 0.0 {
        let due = (t.saturating_sub(*state.last_backup.lock().unwrap())) as f64
            >= a.backup_interval_hours * 3600.0;
        if due {
            *state.last_backup.lock().unwrap() = t;
            run_backup(app, a.keep_backups);
        }
    }

    // Scheduled restarts. With smart_restart, wait until the server is empty.
    if a.auto_restart_enabled && a.restart_interval_hours > 0.0 {
        let due = (t.saturating_sub(*state.last_restart.lock().unwrap())) as f64
            >= a.restart_interval_hours * 3600.0;
        if due {
            if server::is_running() {
                let players = if a.smart_restart {
                    settings::install_dir(app)
                        .ok()
                        .and_then(|d| tauri::async_runtime::block_on(crate::game::live::players(&d)).ok())
                        .map(|p| p.len())
                        .unwrap_or(0)
                } else {
                    0
                };
                if a.smart_restart && players > 0 {
                    logs::record(
                        app,
                        &format!("Scheduled restart waiting — {players} player(s) online."),
                    );
                    // Leave the timer 'due' so we retry once the server empties.
                } else {
                    *state.last_restart.lock().unwrap() = t;
                    run_restart(app, cfg.hide_server_console, 30, "Scheduled restart");
                    return; // skip watchdog this tick (server is cycling)
                }
            } else {
                // Not running — reset so it doesn't fire the instant it starts.
                *state.last_restart.lock().unwrap() = t;
            }
        }
    }

    // Scheduled announcements (MOTD) — only while the server is up.
    if !cfg.announcements.is_empty() && server::is_running() {
        let dir = settings::install_dir(app).ok();
        let mut last = state.announce_last.lock().unwrap();
        for ann in &cfg.announcements {
            if !ann.enabled || ann.interval_minutes <= 0.0 || ann.message.trim().is_empty() {
                continue;
            }
            match last.get(&ann.id).copied() {
                None => {
                    last.insert(ann.id.clone(), t); // prime; first send after one interval
                }
                Some(prev) if (t.saturating_sub(prev)) as f64 >= ann.interval_minutes * 60.0 => {
                    if let Some(d) = &dir {
                        let _ = tauri::async_runtime::block_on(crate::game::live::announce(d, &ann.message));
                    }
                    last.insert(ann.id.clone(), t);
                }
                _ => {}
            }
        }
    }

    // Scheduled auto-update: check for a new server build and apply it.
    if a.auto_update_enabled && a.auto_update_interval_hours > 0.0 {
        let due = (t.saturating_sub(*state.last_update_check.lock().unwrap())) as f64
            >= a.auto_update_interval_hours * 3600.0;
        if due {
            *state.last_update_check.lock().unwrap() = t;
            let status = updates::check(app);
            if status.update_available {
                logs::record(
                    app,
                    &format!(
                        "Server update available (build {} → {}). Applying…",
                        status.installed_build, status.latest_build
                    ),
                );
                discord::notify(
                    app,
                    discord::Event::Restarting("A server update is available — applying now.".into()),
                );
                match updates::apply(app) {
                    Ok(()) => logs::record(app, "Server updated to the latest build."),
                    Err(e) => logs::record(app, &format!("Auto-update failed: {e}")),
                }
            }
        }
    }

    // Crash watchdog: we expected it up, but it isn't.
    if a.auto_restart_on_crash {
        let supervise = *state.supervise.lock().unwrap();
        if supervise && !server::is_running() {
            *state.last_restart.lock().unwrap() = t;
            logs::record(app, "Server stopped unexpectedly — auto-restarting…");
            discord::notify(app, discord::Event::Crashed);
            if let Ok(dir) = settings::install_dir(app) {
                let extra_args = settings::active_profile(app).map(|p| p.extra_launch_args).unwrap_or_default();
                match server::start(&dir, cfg.hide_server_console, &extra_args) {
                    Ok(()) => logs::record(app, "Crash watchdog: server restarted."),
                    Err(e) => logs::record(app, &format!("Crash watchdog: restart failed: {e}")),
                }
            }
        }
    }
}

fn run_backup(app: &AppHandle, keep: u32) {
    let dir = match settings::install_dir(app) {
        Ok(d) => d,
        Err(_) => return,
    };
    match backups::create(app, &dir) {
        Ok(name) => {
            discord::notify(app, discord::Event::BackupCreated(name.clone()));
            logs::record(app, &format!("Auto-backup created: {name}"));
            prune(app, keep);
        }
        Err(e) => logs::record(app, &format!("Auto-backup failed: {e}")),
    }
}

fn prune(app: &AppHandle, keep: u32) {
    if keep == 0 {
        return;
    }
    if let Ok(list) = backups::list(app) {
        // `list` is newest-first; delete everything past the keep count.
        for b in list.iter().skip(keep as usize) {
            let _ = backups::delete(app, &b.name);
        }
    }
}

/// Gracefully restart the server: warn players, save + shut down via REST (waiting
/// `countdown` seconds), wait for it to exit, then start it back up. Falls back to
/// a force stop when the REST shutdown isn't available. Blocking — call off the UI
/// thread. `label` prefixes the activity-log lines (e.g. "Scheduled restart").
pub fn run_restart(app: &AppHandle, hide_console: bool, countdown: i64, label: &str) {
    let dir = match settings::install_dir(app) {
        Ok(d) => d,
        Err(_) => return,
    };
    let secs = countdown.max(0);
    let notice = format!("Server will restart in {secs} seconds.");
    logs::record(app, &format!("{label}: warning players and shutting down ({secs}s)…"));
    discord::notify(app, discord::Event::Restarting(notice.clone()));
    if secs > 0 {
        let _ = tauri::async_runtime::block_on(crate::game::live::announce(&dir, &notice));
    }
    let shutdown_ok = tauri::async_runtime::block_on(crate::game::live::shutdown(&dir, secs, &notice)).is_ok();

    // If the graceful shutdown was accepted, wait for it to go down (up to ~2 min).
    // Otherwise (REST off) skip the wait and force-stop straight away.
    if shutdown_ok {
        for _ in 0..40 {
            if !server::is_running() {
                break;
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    }
    let _ = server::stop(); // force-stop as a safety net / primary when REST is off
    std::thread::sleep(Duration::from_secs(2));

    let extra_args = settings::active_profile(app).map(|p| p.extra_launch_args).unwrap_or_default();
    match server::start(&dir, hide_console, &extra_args) {
        Ok(()) => logs::record(app, &format!("{label}: server started.")),
        Err(e) => logs::record(app, &format!("{label}: failed to start: {e}")),
    }
}
