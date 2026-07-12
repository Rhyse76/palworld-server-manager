//! Live-control dispatch: routes player/moderation/announce actions to the active
//! game's mechanism — Palworld's REST API (`rest`) or ARK's RCON (`ark::live`) —
//! based on the game's `live_control` capability. RCON is synchronous, so those
//! calls run on a blocking task.

use std::path::Path;
use std::path::PathBuf;

use crate::rest::{self, Player};

use super::{ark, active, LiveControl};

fn no_live() -> String {
    "This game doesn't support live control while the server is running.".into()
}

/// Run a blocking (RCON) closure off the async runtime.
async fn blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// Enable the active game's live control (Palworld REST or ARK RCON), setting up
/// config as needed. Synchronous (config writes only).
pub fn enable(dir: &Path) -> Result<rest::EnableResult, String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::enable(dir),
        LiveControl::Rcon => {
            if crate::server::is_running() {
                return Err("Stop the ARK server first — it rewrites its config on shutdown. Enable RCON while stopped, then start.".into());
            }
            ark::config::enable_rcon(dir)
        }
        LiveControl::None => Err("This game has no live control to enable.".into()),
    }
}

pub async fn players(dir: &Path) -> Result<Vec<Player>, String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::players(dir).await,
        LiveControl::Rcon => {
            let d: PathBuf = dir.into();
            blocking(move || ark::live::players(&d)).await
        }
        LiveControl::None => Err(no_live()),
    }
}

pub async fn announce(dir: &Path, message: &str) -> Result<(), String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::announce(dir, message).await,
        LiveControl::Rcon => {
            let (d, m): (PathBuf, String) = (dir.into(), message.into());
            blocking(move || ark::live::announce(&d, &m)).await
        }
        LiveControl::None => Err(no_live()),
    }
}

pub async fn kick(dir: &Path, user_id: &str, message: &str) -> Result<(), String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::kick(dir, user_id, message).await,
        LiveControl::Rcon => {
            let (d, u): (PathBuf, String) = (dir.into(), user_id.into());
            blocking(move || ark::live::kick(&d, &u)).await
        }
        LiveControl::None => Err(no_live()),
    }
}

pub async fn ban(dir: &Path, user_id: &str, message: &str) -> Result<(), String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::ban(dir, user_id, message).await,
        LiveControl::Rcon => {
            let (d, u): (PathBuf, String) = (dir.into(), user_id.into());
            blocking(move || ark::live::ban(&d, &u)).await
        }
        LiveControl::None => Err(no_live()),
    }
}

pub async fn unban(dir: &Path, user_id: &str) -> Result<(), String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::unban(dir, user_id).await,
        LiveControl::Rcon => {
            let (d, u): (PathBuf, String) = (dir.into(), user_id.into());
            blocking(move || ark::live::unban(&d, &u)).await
        }
        LiveControl::None => Err(no_live()),
    }
}

pub async fn save(dir: &Path) -> Result<(), String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::save(dir).await,
        LiveControl::Rcon => {
            let d: PathBuf = dir.into();
            blocking(move || ark::live::save(&d)).await
        }
        LiveControl::None => Err(no_live()),
    }
}

pub async fn shutdown(dir: &Path, seconds: i64, message: &str) -> Result<(), String> {
    match active().spec().live_control {
        LiveControl::RestApi => rest::shutdown(dir, seconds, message).await,
        LiveControl::Rcon => {
            let (d, m): (PathBuf, String) = (dir.into(), message.into());
            blocking(move || ark::live::shutdown(&d, seconds, &m)).await
        }
        LiveControl::None => Err(no_live()),
    }
}
