mod automation;
mod backups;
mod bans;
mod config;
mod detect;
mod discord;
mod logs;
mod mods;
mod network;
mod rest;
mod saves;
mod server;
mod settings;
mod steamcmd;
mod updates;
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

/// Point the active profile at a different install folder.
#[tauri::command]
fn set_install_dir(app: AppHandle, path: String) -> Result<(), String> {
    let profile = settings::active_profile(&app).ok_or("No active profile.")?;
    settings::set_profile_dir(&app, &profile.id, &path)
}

/// Ensure SteamCMD exists, then install/update the Palworld server. Progress is
/// streamed via the `install-log` / `install-progress` events.
#[tauri::command]
async fn install_server(app: AppHandle) -> Result<(), String> {
    let steamcmd = steamcmd::ensure_steamcmd(&app).await?;
    let install_dir = settings::install_dir(&app)?;
    let app_for_task = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        steamcmd::run_update(&app_for_task, &steamcmd, &install_dir)
    })
    .await
    .map_err(|e| e.to_string())?;
    if result.is_ok() {
        logs::record(&app, "Server install/update finished.");
    }
    result
}

#[tauri::command]
fn start_server(app: AppHandle) -> Result<(), String> {
    let install_dir = settings::install_dir(&app)?;
    server::start(&install_dir, settings::hide_console(&app))?;
    automation::set_supervise(&app, true);
    logs::record(&app, "Server started.");
    discord::notify(&app, discord::Event::ServerStarted);
    Ok(())
}

#[tauri::command]
fn stop_server(app: AppHandle) -> Result<(), String> {
    // Mark intent first so the crash watchdog doesn't fight the stop.
    automation::set_supervise(&app, false);
    server::stop()?;
    logs::record(&app, "Server stopped by user.");
    discord::notify(&app, discord::Event::ServerStopped);
    Ok(())
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
    rest::announce(&dir, &message).await?;
    logs::record(&app, &format!("Broadcast: {message}"));
    Ok(())
}

#[tauri::command]
async fn rest_kick(app: AppHandle, userid: String, message: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::kick(&dir, &userid, &message).await?;
    logs::record(&app, &format!("Kicked {userid}."));
    Ok(())
}

#[tauri::command]
async fn rest_ban(app: AppHandle, userid: String, message: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::ban(&dir, &userid, &message).await?;
    logs::record(&app, &format!("Banned {userid}."));
    Ok(())
}

#[tauri::command]
async fn rest_unban(app: AppHandle, userid: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::unban(&dir, &userid).await?;
    logs::record(&app, &format!("Unbanned {userid}."));
    Ok(())
}

#[tauri::command]
fn bans_list(app: AppHandle) -> Result<Vec<String>, String> {
    bans::list(&settings::install_dir(&app)?)
}

#[tauri::command]
async fn rest_save(app: AppHandle) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    rest::save(&dir).await?;
    logs::record(&app, "World saved.");
    Ok(())
}

#[tauri::command]
async fn rest_shutdown(app: AppHandle, seconds: i64, message: String) -> Result<(), String> {
    let dir = settings::install_dir(&app)?;
    // A graceful shutdown from the UI is an intentional stop.
    automation::set_supervise(&app, false);
    rest::shutdown(&dir, seconds, &message).await?;
    logs::record(&app, &format!("Graceful shutdown requested ({seconds}s)."));
    Ok(())
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

#[tauri::command]
fn set_backup_mirror(app: AppHandle, dir: String) -> Result<(), String> {
    settings::set_backup_mirror(&app, dir)
}

// ---- Profiles ----

#[tauri::command]
fn add_profile(app: AppHandle, name: String, path: String) -> Result<String, String> {
    settings::add_profile(&app, &name, &path)
}

#[tauri::command]
fn set_active_profile(app: AppHandle, id: String) -> Result<(), String> {
    settings::set_active(&app, &id)
}

#[tauri::command]
fn rename_profile(app: AppHandle, id: String, name: String) -> Result<(), String> {
    settings::rename_profile(&app, &id, &name)
}

#[tauri::command]
fn delete_profile(app: AppHandle, id: String) -> Result<(), String> {
    settings::delete_profile(&app, &id)
}

// ---- Automation ----

#[tauri::command]
fn set_automation(app: AppHandle, automation: settings::Automation) -> Result<(), String> {
    settings::set_automation(&app, automation)
}

#[tauri::command]
fn set_hide_console(app: AppHandle, hide: bool) -> Result<(), String> {
    settings::set_hide_console(&app, hide)
}

#[tauri::command]
fn set_discord(app: AppHandle, discord: settings::Discord) -> Result<(), String> {
    settings::set_discord(&app, discord)
}

#[tauri::command]
fn discord_test(app: AppHandle) -> Result<(), String> {
    let cfg = settings::load(&app).discord;
    if cfg.webhook_url.trim().is_empty() {
        return Err("Enter a webhook URL first.".into());
    }
    discord::notify(&app, discord::Event::Test);
    Ok(())
}

// ---- Activity log ----

#[tauri::command]
fn read_activity_log(app: AppHandle) -> Result<String, String> {
    logs::read_tail(&app)
}

// ---- Connectivity ----

#[tauri::command]
fn set_announcements(app: AppHandle, announcements: Vec<settings::Announcement>) -> Result<(), String> {
    settings::set_announcements(&app, announcements)
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<updates::UpdateStatus, String> {
    tauri::async_runtime::spawn_blocking(move || updates::check(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn mods_list(app: AppHandle) -> Result<Vec<mods::ModInfo>, String> {
    mods::list(&settings::install_dir(&app)?)
}

#[tauri::command]
fn mod_set_enabled(app: AppHandle, name: String, enabled: bool) -> Result<(), String> {
    mods::set_enabled(&settings::install_dir(&app)?, &name, enabled)
}

#[tauri::command]
fn mod_install(app: AppHandle, path: String) -> Result<String, String> {
    mods::install(&settings::install_dir(&app)?, std::path::Path::new(&path))
}

#[tauri::command]
fn mod_remove(app: AppHandle, name: String) -> Result<(), String> {
    mods::remove(&settings::install_dir(&app)?, &name)
}

#[tauri::command]
async fn inspect_save(app: AppHandle) -> Result<saves::SaveInfo, String> {
    let dir = settings::install_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || saves::inspect(&dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn network_info(app: AppHandle) -> network::NetworkInfo {
    network::info(&app)
}

#[tauri::command]
fn network_forward(app: AppHandle) -> Result<String, String> {
    network::forward(&app)
}

#[tauri::command]
fn network_unforward(app: AppHandle) -> Result<String, String> {
    network::unforward(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(automation::SchedulerState::default())
        .setup(|app| {
            automation::start(app.handle().clone());
            discord::start_player_watch(app.handle().clone());
            Ok(())
        })
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
            bans_list,
            rest_save,
            rest_shutdown,
            enable_rest_api,
            backup_create,
            backup_list,
            backup_restore,
            backup_delete,
            backup_open_folder,
            set_backup_mirror,
            add_profile,
            set_active_profile,
            rename_profile,
            delete_profile,
            set_automation,
            set_hide_console,
            set_discord,
            discord_test,
            read_activity_log,
            set_announcements,
            check_update,
            network_info,
            network_forward,
            network_unforward,
            inspect_save,
            mods_list,
            mod_set_enabled,
            mod_install,
            mod_remove,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
