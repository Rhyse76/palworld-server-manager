//! Host machine performance (CPU/RAM) for the Dashboard's "how's this machine
//! doing" tiles. Independent of any per-game live-control protocol (REST/RCON) —
//! this reads the OS directly, so it works the same for every game.

use std::sync::Mutex;

use serde::Serialize;
use sysinfo::System;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HostStats {
    pub cpu_percent: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub mem_percent: f32,
    /// CPU/RAM used by the active game's server process specifically, if it's
    /// running (summed across matching processes, e.g. launcher + shipping exe).
    pub server_cpu_percent: Option<f32>,
    pub server_mem_mb: Option<u64>,
}

// A `System` needs to persist between calls: CPU usage is computed from the delta
// since the previous refresh, not a point-in-time read.
static SYS: Mutex<Option<System>> = Mutex::new(None);

pub fn sample() -> HostStats {
    let mut guard = SYS.lock().unwrap();
    let sys = guard.get_or_insert_with(System::new_all);
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mem_total_mb = sys.total_memory() / 1024 / 1024;
    let mem_used_mb = sys.used_memory() / 1024 / 1024;
    let mem_percent = if mem_total_mb > 0 {
        (mem_used_mb as f32 / mem_total_mb as f32) * 100.0
    } else {
        0.0
    };

    // Match the same process(es) `server::is_running()` looks for: image name
    // contains the active game's marker substring (e.g. "Shipping" for Palworld).
    let marker = crate::game::active().spec().process_marker;
    let mut server_cpu = 0.0f32;
    let mut server_mem_bytes = 0u64;
    let mut found = false;
    for proc in sys.processes().values() {
        if proc.name().to_string_lossy().contains(marker) {
            found = true;
            server_cpu += proc.cpu_usage();
            server_mem_bytes += proc.memory(); // bytes, per sysinfo::Process::memory
        }
    }

    HostStats {
        cpu_percent: sys.global_cpu_usage(),
        mem_used_mb,
        mem_total_mb,
        mem_percent,
        server_cpu_percent: found.then_some(server_cpu),
        server_mem_mb: found.then_some(server_mem_bytes / 1024 / 1024),
    }
}
