//! Client for Palworld's official REST admin API.
//!
//! The server exposes this on `http://127.0.0.1:<RESTAPIPort>/v1/api/...` (default
//! port 8212) using HTTP Basic auth with username `admin` and the server's
//! `AdminPassword`. It must be enabled in config (`RESTAPIEnabled=True`) and the
//! server restarted for changes to take effect.

use std::path::Path;
use std::time::Duration;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;

use crate::config;
use crate::game::palworld::config as pal_config;

const HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8212;
const TIMEOUT: Duration = Duration::from_secs(5);

struct Conn {
    port: u16,
    password: String,
}

/// Read REST connection details straight from *this* install dir's own Palworld
/// config, not the ambiguous `crate::config::read` (which dispatches by whatever
/// game is globally "active" in the UI, not by which game this directory actually
/// belongs to). This module is only ever used for Palworld, but it used to go
/// through that active-game dispatch anyway -- meaning REST calls for a
/// backgrounded Palworld profile (player-watch, scheduled restarts' graceful
/// shutdown, auto-update's pre-restart save) would silently read the *active*
/// profile's config instead of this one's, and fail, whenever some other game was
/// the one currently selected in the UI. Confirmed live: join/leave notifications
/// only worked while Palworld itself was the active profile.
fn conn(install_dir: &Path) -> Result<Conn, String> {
    let fields = pal_config::read(install_dir)?;

    let enabled = config::find(&fields, "RESTAPIEnabled").as_deref() == Some("true");
    if !enabled {
        return Err(
            "REST API is disabled. Click “Enable REST API”, then restart the server.".into(),
        );
    }

    let port = config::find(&fields, "RESTAPIPort")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let password = config::find(&fields, "AdminPassword").unwrap_or_default();
    if password.is_empty() {
        return Err("No Admin Password is set. Set one in Config, then restart the server.".into());
    }

    Ok(Conn { port, password })
}

fn base(conn: &Conn) -> String {
    format!("http://{HOST}:{}/v1/api", conn.port)
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())
}

fn friendly_err(e: reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() {
        "Could not reach the server. Is it running with the REST API enabled?".into()
    } else {
        e.to_string()
    }
}

async fn get_json<T: DeserializeOwned>(install_dir: &Path, path: &str) -> Result<T, String> {
    let c = conn(install_dir)?;
    let resp = client()?
        .get(format!("{}{path}", base(&c)))
        .basic_auth("admin", Some(&c.password))
        .send()
        .await
        .map_err(friendly_err)?;
    if !resp.status().is_success() {
        return Err(status_message(resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn post(install_dir: &Path, path: &str, body: serde_json::Value) -> Result<(), String> {
    let c = conn(install_dir)?;
    let resp = client()?
        .post(format!("{}{path}", base(&c)))
        .basic_auth("admin", Some(&c.password))
        .json(&body)
        .send()
        .await
        .map_err(friendly_err)?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(status_message(resp.status()))
    }
}

fn status_message(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 => "Authentication failed — the Admin Password doesn't match the running server. Restart the server after changing it.".into(),
        _ => format!("Server returned {status}"),
    }
}

// ---- Response models (Palworld uses mostly lowercase JSON keys) ----

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ServerInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub servername: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Metrics {
    #[serde(default)]
    pub serverfps: i64,
    #[serde(default)]
    pub currentplayernum: i64,
    #[serde(default)]
    pub maxplayernum: i64,
    #[serde(default)]
    pub serverframetime: f64,
    #[serde(default)]
    pub uptime: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Player {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "playerId")]
    pub player_id: String,
    #[serde(default, rename = "userId")]
    pub user_id: String,
    #[serde(default)]
    pub ping: f64,
    #[serde(default)]
    pub level: i64,
}

#[derive(Deserialize)]
struct PlayersResponse {
    #[serde(default)]
    players: Vec<Player>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub info: ServerInfo,
    pub metrics: Metrics,
}

// ---- Public operations ----

pub async fn overview(install_dir: &Path) -> Result<Overview, String> {
    let info = get_json::<ServerInfo>(install_dir, "/info").await?;
    let metrics = get_json::<Metrics>(install_dir, "/metrics").await?;
    Ok(Overview { info, metrics })
}

pub async fn players(install_dir: &Path) -> Result<Vec<Player>, String> {
    Ok(get_json::<PlayersResponse>(install_dir, "/players")
        .await?
        .players)
}

pub async fn announce(install_dir: &Path, message: &str) -> Result<(), String> {
    post(install_dir, "/announce", json!({ "message": message })).await
}

pub async fn kick(install_dir: &Path, userid: &str, message: &str) -> Result<(), String> {
    post(install_dir, "/kick", json!({ "userid": userid, "message": message })).await
}

pub async fn ban(install_dir: &Path, userid: &str, message: &str) -> Result<(), String> {
    post(install_dir, "/ban", json!({ "userid": userid, "message": message })).await
}

pub async fn unban(install_dir: &Path, userid: &str) -> Result<(), String> {
    post(install_dir, "/unban", json!({ "userid": userid })).await
}

pub async fn save(install_dir: &Path) -> Result<(), String> {
    post(install_dir, "/save", json!({})).await
}

pub async fn shutdown(install_dir: &Path, waittime: i64, message: &str) -> Result<(), String> {
    post(
        install_dir,
        "/shutdown",
        json!({ "waittime": waittime, "message": message }),
    )
    .await
}

// ---- Enable helper ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableResult {
    pub port: u16,
    pub admin_password: String,
    pub generated_password: bool,
}

/// Turn on the REST API in config: set `RESTAPIEnabled=True`, ensure a port, and
/// ensure an Admin Password exists (generating one if empty). Requires a server
/// restart to take effect.
pub fn enable(install_dir: &Path) -> Result<EnableResult, String> {
    let mut fields = pal_config::read(install_dir).unwrap_or_default();

    config::upsert(&mut fields, "RESTAPIEnabled", "true", "bool");

    let port = config::find(&fields, "RESTAPIPort")
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    config::upsert(&mut fields, "RESTAPIPort", &port.to_string(), "int");

    let existing = config::find(&fields, "AdminPassword").unwrap_or_default();
    let generated_password = existing.is_empty();
    let admin_password = if generated_password {
        let pw = random_password();
        config::upsert(&mut fields, "AdminPassword", &pw, "string");
        pw
    } else {
        existing
    };

    pal_config::write(install_dir, &fields)?;

    Ok(EnableResult {
        port,
        admin_password,
        generated_password,
    })
}

/// Lightweight random password for local admin convenience (not cryptographic).
/// Seeded from the current time; users can change it in Config.
fn random_password() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15)
        ^ std::process::id() as u64;

    (0..20)
        .map(|_| {
            // xorshift64
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            CHARS[(seed % CHARS.len() as u64) as usize] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_uses_palworlds_own_config_regardless_of_which_game_is_globally_active() {
        // Regression test for a real bug, confirmed live: rest.rs used to go
        // through the generic crate::config::read/write, which dispatches by
        // whatever game is globally "active" in the UI, not by which game this
        // install_dir actually belongs to. A backgrounded Palworld profile's
        // REST-based features (join/leave notifications, scheduled restarts'
        // graceful shutdown, auto-update's pre-restart save) silently failed
        // whenever some other game was the one currently selected.
        let dir = std::env::temp_dir().join(format!("palworld-rest-test-{}", std::process::id()));
        let config_dir = dir.join("Pal/Saved/Config/WindowsServer");
        std::fs::create_dir_all(&config_dir).unwrap();
        let ini_path = config_dir.join("PalWorldSettings.ini");
        std::fs::write(
            &ini_path,
            "[/Script/Pal.PalGameWorldSettings]\nOptionSettings=(Difficulty=None)\n",
        )
        .unwrap();

        // Deliberately set a DIFFERENT game active before calling enable() -- the
        // exact condition that broke it live.
        crate::game::set_active("ark-sa");
        let result = enable(&dir);
        crate::game::set_active("palworld"); // restore, don't leak state into other tests

        result.expect("enable() should succeed using Palworld's own config, not ARK's");

        // The real proof: did it actually write into *this* dir's real
        // PalWorldSettings.ini, not silently write ARK-formatted content
        // elsewhere (or nowhere) because it was reading ARK's parser instead?
        let written = std::fs::read_to_string(&ini_path).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert!(written.contains("RESTAPIEnabled=True"), "wrote: {written}");
    }
}
