mod config;
mod server;
mod settings;
mod steamcmd;
mod util;

use serde::Serialize;
use tauri::AppHandle;

/// Snapshot of the current install/run state for the dashboard.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusInfo {
    install_dir: String,
    installed: bool,
    running: bool,
    steamcmd_ready: bool,
}

#[tauri::command]
fn get_status(app: AppHandle) -> Result<StatusInfo, String> {
    let install_dir = settings::install_dir(&app)?;
    Ok(StatusInfo {
        installed: server::is_installed(&install_dir),
        running: server::is_running(),
        steamcmd_ready: steamcmd::is_steamcmd_ready(&app),
        install_dir: install_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn get_app_config(app: AppHandle) -> settings::AppConfig {
    settings::load(&app)
}

#[tauri::command]
fn set_install_dir(app: AppHandle, path: Option<String>) -> Result<(), String> {
    let mut cfg = settings::load(&app);
    cfg.install_dir = path.filter(|s| !s.trim().is_empty());
    settings::save(&app, &cfg)
}

/// Ensure SteamCMD exists, then install/update the Palworld server. Progress is
/// streamed via the `install-log` / `install-progress` events.
#[tauri::command]
async fn install_server(app: AppHandle) -> Result<(), String> {
    let steamcmd = steamcmd::ensure_steamcmd(&app).await?;
    let install_dir = settings::install_dir(&app)?;
    let app_for_task = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        steamcmd::run_update(&app_for_task, &steamcmd, &install_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn start_server(app: AppHandle) -> Result<(), String> {
    let install_dir = settings::install_dir(&app)?;
    server::start(&install_dir)
}

#[tauri::command]
fn stop_server() -> Result<(), String> {
    server::stop()
}

#[tauri::command]
fn read_config(app: AppHandle) -> Result<Vec<config::ConfigField>, String> {
    let install_dir = settings::install_dir(&app)?;
    config::read(&install_dir)
}

#[tauri::command]
fn write_config(app: AppHandle, fields: Vec<config::ConfigField>) -> Result<(), String> {
    let install_dir = settings::install_dir(&app)?;
    config::write(&install_dir, &fields)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_app_config,
            set_install_dir,
            install_server,
            start_server,
            stop_server,
            read_config,
            write_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
