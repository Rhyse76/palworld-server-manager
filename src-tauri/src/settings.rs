//! Persisted app settings: multiple server profiles (each an install location),
//! the active profile, and automation settings. Also resolves the active install
//! dir and the shared SteamCMD dir.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// A named server install location. Multiple profiles let one manager drive
/// several servers (one active at a time for now).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub install_dir: String,
}

/// Auto-restart and scheduled-backup settings, applied to the active profile.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Automation {
    pub auto_restart_enabled: bool,
    pub restart_interval_hours: f64,
    pub auto_backup_enabled: bool,
    pub backup_interval_hours: f64,
    pub keep_backups: u32,
    /// Restart the server automatically if it dies unexpectedly (crash watchdog).
    pub auto_restart_on_crash: bool,
}

impl Default for Automation {
    fn default() -> Self {
        Self {
            auto_restart_enabled: false,
            restart_interval_hours: 6.0,
            auto_backup_enabled: false,
            backup_interval_hours: 2.0,
            keep_backups: 10,
            auto_restart_on_crash: true,
        }
    }
}

/// Discord webhook notification settings.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Discord {
    pub enabled: bool,
    pub webhook_url: String,
    pub notify_server: bool,
    pub notify_players: bool,
    pub notify_backups: bool,
}

impl Default for Discord {
    fn default() -> Self {
        Self {
            enabled: false,
            webhook_url: String::new(),
            notify_server: true,
            notify_players: true,
            notify_backups: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub active_profile: Option<String>,
    pub profiles: Vec<ServerProfile>,
    pub automation: Automation,
    pub discord: Discord,
    /// Hide the server's console window when launching (default: show it).
    pub hide_server_console: bool,
    /// Legacy single-install field, migrated into a profile on first load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_dir: Option<String>,
}

fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

fn read_raw(app: &AppHandle) -> AppConfig {
    config_file(app)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let path = config_file(app)?;
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Load config, ensuring there's always at least one profile and an active one.
/// Migrates the legacy `install_dir` field and persists if anything changed.
pub fn load(app: &AppHandle) -> AppConfig {
    let mut cfg = read_raw(app);
    let mut changed = false;

    // Migrate legacy single-install setups into a profile.
    if cfg.profiles.is_empty() {
        let dir = cfg
            .install_dir
            .take()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                default_install_dir(app)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default()
            });
        cfg.profiles.push(ServerProfile {
            id: new_id(),
            name: "Default".into(),
            install_dir: dir,
        });
        changed = true;
    }

    // Ensure a valid active profile.
    let active_ok = cfg
        .active_profile
        .as_ref()
        .map(|id| cfg.profiles.iter().any(|p| &p.id == id))
        .unwrap_or(false);
    if !active_ok {
        cfg.active_profile = cfg.profiles.first().map(|p| p.id.clone());
        changed = true;
    }

    if changed {
        let _ = save(app, &cfg);
    }
    cfg
}

pub fn active_profile(app: &AppHandle) -> Option<ServerProfile> {
    let cfg = load(app);
    let id = cfg.active_profile.clone()?;
    cfg.profiles.into_iter().find(|p| p.id == id)
}

/// The install dir of the active profile.
pub fn install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    active_profile(app)
        .map(|p| PathBuf::from(p.install_dir))
        .ok_or_else(|| "No active server profile.".into())
}

pub fn default_install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("server"))
}

pub fn steamcmd_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("steamcmd"))
}

// ---- Mutations ----

pub fn set_active(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut cfg = load(app);
    if !cfg.profiles.iter().any(|p| p.id == id) {
        return Err("Profile not found.".into());
    }
    cfg.active_profile = Some(id.to_string());
    save(app, &cfg)
}

/// Add a profile. If a profile already points at the same dir, activate it
/// instead of duplicating. Returns the active profile id.
pub fn add_profile(app: &AppHandle, name: &str, install_dir: &str) -> Result<String, String> {
    let mut cfg = load(app);
    if let Some(existing) = cfg
        .profiles
        .iter()
        .find(|p| p.install_dir.eq_ignore_ascii_case(install_dir))
    {
        let id = existing.id.clone();
        cfg.active_profile = Some(id.clone());
        save(app, &cfg)?;
        return Ok(id);
    }
    let id = new_id();
    cfg.profiles.push(ServerProfile {
        id: id.clone(),
        name: if name.trim().is_empty() { "Server".into() } else { name.trim().to_string() },
        install_dir: install_dir.to_string(),
    });
    cfg.active_profile = Some(id.clone());
    save(app, &cfg)?;
    Ok(id)
}

pub fn rename_profile(app: &AppHandle, id: &str, name: &str) -> Result<(), String> {
    let mut cfg = load(app);
    let p = cfg
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found.")?;
    p.name = name.trim().to_string();
    save(app, &cfg)
}

pub fn set_profile_dir(app: &AppHandle, id: &str, install_dir: &str) -> Result<(), String> {
    let mut cfg = load(app);
    let p = cfg
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found.")?;
    p.install_dir = install_dir.to_string();
    save(app, &cfg)
}

pub fn delete_profile(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut cfg = load(app);
    if cfg.profiles.len() <= 1 {
        return Err("Can't delete the only profile.".into());
    }
    cfg.profiles.retain(|p| p.id != id);
    if cfg.active_profile.as_deref() == Some(id) {
        cfg.active_profile = cfg.profiles.first().map(|p| p.id.clone());
    }
    save(app, &cfg)
}

pub fn set_automation(app: &AppHandle, automation: Automation) -> Result<(), String> {
    let mut cfg = load(app);
    cfg.automation = automation;
    save(app, &cfg)
}

pub fn set_hide_console(app: &AppHandle, hide: bool) -> Result<(), String> {
    let mut cfg = load(app);
    cfg.hide_server_console = hide;
    save(app, &cfg)
}

pub fn set_discord(app: &AppHandle, discord: Discord) -> Result<(), String> {
    let mut cfg = load(app);
    cfg.discord = discord;
    save(app, &cfg)
}

pub fn hide_console(app: &AppHandle) -> bool {
    load(app).hide_server_console
}

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("p{nanos}")
}
