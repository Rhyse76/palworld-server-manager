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

use crate::{backups, discord, logs, rest, server, settings, updates};

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
    let a = &cfg.automation;
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

    // Scheduled restarts (only while running). Skip the watchdog this tick.
    if a.auto_restart_enabled && a.restart_interval_hours > 0.0 {
        let due = (t.saturating_sub(*state.last_restart.lock().unwrap())) as f64
            >= a.restart_interval_hours * 3600.0;
        if due {
            *state.last_restart.lock().unwrap() = t;
            if server::is_running() {
                run_restart(app, cfg.hide_server_console);
            }
            return;
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
                        let _ = tauri::async_runtime::block_on(rest::announce(d, &ann.message));
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
                match server::start(&dir, cfg.hide_server_console) {
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

fn run_restart(app: &AppHandle, hide_console: bool) {
    let dir = match settings::install_dir(app) {
        Ok(d) => d,
        Err(_) => return,
    };
    logs::record(app, "Scheduled restart: warning players and shutting down (30s)…");
    discord::notify(app, discord::Event::Restarting("Scheduled restart in 30 seconds.".into()));
    let _ = tauri::async_runtime::block_on(rest::announce(
        &dir,
        "Server will restart in 30 seconds.",
    ));
    let _ = tauri::async_runtime::block_on(rest::shutdown(
        &dir,
        30,
        "Server will restart in 30 seconds.",
    ));

    // Wait for it to go down (up to ~2 min), then force-stop as a safety net.
    for _ in 0..40 {
        if !server::is_running() {
            break;
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    let _ = server::stop();
    std::thread::sleep(Duration::from_secs(2));

    match server::start(&dir, hide_console) {
        Ok(()) => logs::record(app, "Scheduled restart: server started."),
        Err(e) => logs::record(app, &format!("Scheduled restart: failed to start: {e}")),
    }
}
