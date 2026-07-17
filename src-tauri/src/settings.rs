//! Persisted app settings: multiple server profiles (each an install location),
//! the active profile, and automation settings. Also resolves the active install
//! dir and the shared SteamCMD dir.

use std::collections::HashMap;
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
    /// Which game this profile manages (`game::by_id`). Defaults to `palworld`
    /// so existing configs (written before multi-game) load unchanged.
    #[serde(default = "default_game")]
    pub game: String,
    /// Extra command-line arguments appended after the game's own auto-generated
    /// launch args (e.g. ARK's map/port/RCON flags), space-separated. Free text —
    /// this app doesn't validate or interpret them.
    #[serde(default)]
    pub extra_launch_args: String,
    /// Auto-restart/auto-backup/crash-watchdog settings for this profile specifically
    /// (each game server has its own schedule). Migrated once from the old global
    /// `AppConfig.automation` on first load after upgrading — see `load()`.
    #[serde(default)]
    pub automation: Automation,
}

fn default_game() -> String {
    "palworld".into()
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
    /// For scheduled restarts: wait until 0 players are online before restarting.
    pub smart_restart: bool,
    /// Check for and apply new Palworld server versions automatically.
    pub auto_update_enabled: bool,
    pub auto_update_interval_hours: f64,
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
            smart_restart: false,
            auto_update_enabled: false,
            auto_update_interval_hours: 6.0,
        }
    }
}

/// Discord webhook notification settings.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Discord {
    pub enabled: bool,
    /// Legacy single webhook URL, shared by every game before per-game webhooks.
    /// No longer written to by the UI — kept so old configs still deserialize, and
    /// as the seed value for the one-time migration into `webhooks` (see
    /// `AppConfig.discord_migrated`).
    pub webhook_url: String,
    pub notify_server: bool,
    pub notify_players: bool,
    pub notify_backups: bool,
    /// Per-game webhook URLs, keyed by game id (`game::by_id`), so e.g. Palworld and
    /// ARK: SA can post to different Discord channels. The event-type toggles above
    /// stay shared across all games.
    pub webhooks: HashMap<String, String>,
}

impl Default for Discord {
    fn default() -> Self {
        Self {
            enabled: false,
            webhook_url: String::new(),
            notify_server: true,
            notify_players: true,
            notify_backups: true,
            webhooks: HashMap::new(),
        }
    }
}

/// A recurring in-game broadcast (MOTD / rules / reminders).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Announcement {
    pub id: String,
    pub message: String,
    pub interval_minutes: f64,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub active_profile: Option<String>,
    pub profiles: Vec<ServerProfile>,
    /// Legacy global automation settings — replaced by `ServerProfile.automation`
    /// (each game server needs its own schedule). Kept only so `load()` can migrate
    /// it onto profiles once; nothing reads it after that.
    pub automation: Automation,
    /// Set once `automation` above has been copied onto every profile, so the
    /// migration in `load()` runs exactly once even though the legacy field stays
    /// in the file.
    pub automation_migrated: bool,
    pub discord: Discord,
    /// Set once the legacy single `discord.webhook_url` has been copied into
    /// `discord.webhooks` for every game, so the migration in `load()` runs exactly
    /// once even though the legacy field stays in the file.
    #[serde(default)]
    pub discord_migrated: bool,
    pub announcements: Vec<Announcement>,
    /// Extra folder each backup is also copied to (e.g. a cloud-synced folder).
    pub backup_mirror_dir: String,
    /// Hide the server's console window when launching (default: show it).
    pub hide_server_console: bool,
    /// CurseForge API key (console.curseforge.com), used to search mods for games
    /// with `ModsKind::CurseForgeIds` (e.g. ARK: SA). Stored plaintext in this local
    /// config.json — same trust boundary as the Discord webhook URL above.
    pub curseforge_api_key: String,
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
            game: default_game(),
            extra_launch_args: String::new(),
            automation: Automation::default(),
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

    // One-time migration: automation used to be one global setting; copy it onto
    // every existing profile so nobody's current schedule silently vanishes.
    if !cfg.automation_migrated {
        for p in &mut cfg.profiles {
            p.automation = cfg.automation.clone();
        }
        cfg.automation_migrated = true;
        changed = true;
    }

    // One-time migration: Discord used to be one webhook URL shared by every game;
    // seed the per-game map with it so nobody's existing notifications silently stop,
    // and they can split channels out per game from Settings afterward.
    if !cfg.discord_migrated {
        let legacy_url = cfg.discord.webhook_url.trim().to_string();
        if !legacy_url.is_empty() {
            for spec in crate::game::all() {
                cfg.discord.webhooks.entry(spec.id.to_string()).or_insert_with(|| legacy_url.clone());
            }
        }
        cfg.discord_migrated = true;
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

/// The active profile's game id (default `palworld`).
pub fn active_game_id(app: &AppHandle) -> String {
    active_profile(app).map(|p| p.game).unwrap_or_else(default_game)
}

/// The install dir of the active profile.
pub fn install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    active_profile(app)
        .map(|p| PathBuf::from(p.install_dir))
        .ok_or_else(|| "No active server profile.".into())
}

pub fn default_install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    default_install_dir_for(app, "palworld")
}

/// Default install dir for `game`, distinct per game so picking a different game (e.g.
/// in the first-run wizard) never collides with an existing profile's install dir.
/// Palworld keeps the original bare "server" path for backward compatibility with
/// installs that predate multi-game support.
pub fn default_install_dir_for(app: &AppHandle, game: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(if game.is_empty() || game == "palworld" {
        dir.join("server")
    } else {
        dir.join(format!("server-{game}"))
    })
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
pub fn add_profile(app: &AppHandle, name: &str, install_dir: &str, game: &str) -> Result<String, String> {
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
    let game = if game.trim().is_empty() { default_game() } else { game.trim().to_string() };
    let id = new_id();
    cfg.profiles.push(ServerProfile {
        id: id.clone(),
        name: if name.trim().is_empty() { "Server".into() } else { name.trim().to_string() },
        install_dir: install_dir.to_string(),
        game,
        extra_launch_args: String::new(),
        automation: Automation::default(),
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

pub fn set_launch_args(app: &AppHandle, id: &str, args: &str) -> Result<(), String> {
    let mut cfg = load(app);
    let p = cfg
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found.")?;
    p.extra_launch_args = args.trim().to_string();
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
    let active = cfg.active_profile.clone();
    let p = cfg
        .profiles
        .iter_mut()
        .find(|p| Some(&p.id) == active.as_ref())
        .ok_or("No active server profile.")?;
    p.automation = automation;
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

pub fn set_curseforge_key(app: &AppHandle, key: String) -> Result<(), String> {
    let mut cfg = load(app);
    cfg.curseforge_api_key = key.trim().to_string();
    save(app, &cfg)
}

pub fn set_announcements(app: &AppHandle, announcements: Vec<Announcement>) -> Result<(), String> {
    let mut cfg = load(app);
    cfg.announcements = announcements;
    save(app, &cfg)
}

pub fn set_backup_mirror(app: &AppHandle, dir: String) -> Result<(), String> {
    let mut cfg = load(app);
    cfg.backup_mirror_dir = dir;
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
