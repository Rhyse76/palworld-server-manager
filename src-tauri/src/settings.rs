//! Persisted app settings (where the server is installed) plus path resolution
//! for the SteamCMD and server install directories.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// User-facing app configuration, stored as JSON in the app config dir.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Where the Palworld dedicated server is installed. `None` = use the default.
    pub install_dir: Option<String>,
}

fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

pub fn load(app: &AppHandle) -> AppConfig {
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

/// Default location for the server install (under the app's data dir).
pub fn default_install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("server"))
}

/// The install dir the app should actually use: the user's choice, or the default.
pub fn install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    match load(app).install_dir {
        Some(s) if !s.trim().is_empty() => Ok(PathBuf::from(s)),
        _ => default_install_dir(app),
    }
}

/// Directory where SteamCMD lives (under the app's data dir).
pub fn steamcmd_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("steamcmd"))
}
