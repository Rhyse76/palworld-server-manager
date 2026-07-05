//! Background scheduler for auto-restart and scheduled backups.
//!
//! A single OS thread wakes every 60s and, based on the active profile's
//! automation settings, creates backups (pruning old ones) and restarts the
//! server on the configured intervals. Activity is emitted as `automation-log`
//! events for the UI.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

use crate::{backups, logs, rest, server, settings};

const TICK: Duration = Duration::from_secs(60);

/// Tracks when each automated action last ran (unix seconds).
#[derive(Default)]
pub struct SchedulerState {
    last_backup: Mutex<u64>,
    last_restart: Mutex<u64>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Start the scheduler thread. Timestamps start at "now" so nothing fires
/// immediately on launch.
pub fn start(app: AppHandle) {
    {
        let state = app.state::<SchedulerState>();
        let t = now();
        *state.last_backup.lock().unwrap() = t;
        *state.last_restart.lock().unwrap() = t;
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

    if a.auto_backup_enabled && a.backup_interval_hours > 0.0 {
        let due = {
            let last = *state.last_backup.lock().unwrap();
            (t.saturating_sub(last)) as f64 >= a.backup_interval_hours * 3600.0
        };
        if due {
            *state.last_backup.lock().unwrap() = t;
            run_backup(app, a.keep_backups);
        }
    }

    if a.auto_restart_enabled && a.restart_interval_hours > 0.0 {
        let due = {
            let last = *state.last_restart.lock().unwrap();
            (t.saturating_sub(last)) as f64 >= a.restart_interval_hours * 3600.0
        };
        if due {
            *state.last_restart.lock().unwrap() = t;
            if server::is_running() {
                run_restart(app);
            }
        }
    }
}

fn log(app: &AppHandle, msg: impl Into<String>) {
    let _ = app.emit("automation-log", msg.into());
}

fn run_backup(app: &AppHandle, keep: u32) {
    let dir = match settings::install_dir(app) {
        Ok(d) => d,
        Err(_) => return,
    };
    match backups::create(app, &dir) {
        Ok(name) => {
            log(app, format!("Auto-backup created: {name}"));
            prune(app, keep);
        }
        Err(e) => log(app, format!("Auto-backup failed: {e}")),
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

fn run_restart(app: &AppHandle) {
    let dir = match settings::install_dir(app) {
        Ok(d) => d,
        Err(_) => return,
    };
    log(app, "Auto-restart: warning players and shutting down (30s)…");
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

    let log_path = match logs::log_path(app) {
        Ok(p) => p,
        Err(e) => {
            log(app, format!("Auto-restart: {e}"));
            return;
        }
    };
    match server::start(&dir, &log_path) {
        Ok(()) => log(app, "Auto-restart: server started."),
        Err(e) => log(app, format!("Auto-restart: failed to start server: {e}")),
    }
}
