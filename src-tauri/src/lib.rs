mod backups;
mod config;
mod detect;
mod rest;
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

/// Scan for existing Palworld server installations on this machine.
#[tauri::command]
fn detect_installs(app: AppHandle) -> Vec<detect::DetectedInstall> {
    detect::detect(&app)
}

/// Export the provided config fields to a portable `.json` preset.
#[tauri::command]
fn export_config(fields: Vec<config::ConfigField>, dest: String) -> Result<(), String> {
    config::export_json(&fields, std::path::Path::new(&dest))
}

/// Load config fields from a `.json` preset or a `PalWorldSettings.ini` file.
#[tauri::command]
fn import_config(path: String) -> Result<Vec<config::ConfigField>, String> {
    config::import_file(std::path::Path::new(&path))
}

// ---- REST API (live dashboard) ----

#[tauri::command]
async fn rest_overview(app: AppHandle) -> Result<rest::Overview, String> {
    let dir = settings::install_dir(&app)?;
    rest::overview(&dir).await
}

#[tauri::command]
async fn rest_players(app: AppHandle) -> Result<Vec<rest::Player>, String> {
    let dir = settings::install_dir(&app)?;
    rest::players(&dir).await
}

#[tauri::command]
async fn rest_announce(app: AppHandle, message: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::announce(&dir, &message).await
}

#[tauri::command]
async fn rest_kick(app: AppHandle, userid: String, message: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::kick(&dir, &userid, &message).await
}

#[tauri::command]
async fn rest_ban(app: AppHandle, userid: String, message: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::ban(&dir, &userid, &message).await
}

#[tauri::command]
async fn rest_unban(app: AppHandle, userid: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::unban(&dir, &userid).await
}

#[tauri::command]
async fn rest_save(app: AppHandle) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::save(&dir).await
}

#[tauri::command]
async fn rest_shutdown(app: AppHandle, seconds: i64, message: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::shutdown(&dir, seconds, &message).await
}

#[tauri::command]
fn enable_rest_api(app: AppHandle) -> Result<rest::EnableResult, String> {
    let dir = settings::install_dir(&app)?;
    rest::enable(&dir)
}

// ---- Backups ----

#[tauri::command]
fn backup_create(app: AppHandle) -> Result<String, String> {
    let dir = settings::install_dir(&app)?;
    backups::create(&app, &dir)
}

#[tauri::command]
fn backup_list(app: AppHandle) -> Result<Vec<backups::BackupInfo>, String> {
    backups::list(&app)
}

#[tauri::command]
fn backup_restore(app: AppHandle, name: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    backups::restore(&app, &dir, &name)
}

#[tauri::command]
fn backup_delete(app: AppHandle, name: String) -> Result<(), String> {
    backups::delete(&app, &name)
}

#[tauri::command]
fn backup_open_folder(app: AppHandle) -> Result<(), String> {
    backups::open_folder(&app)
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
            detect_installs,
            export_config,
            import_config,
            rest_overview,
            rest_players,
            rest_announce,
            rest_kick,
            rest_ban,
            rest_unban,
            rest_save,
            rest_shutdown,
            enable_rest_api,
            backup_create,
            backup_list,
            backup_restore,
            backup_delete,
            backup_open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
