//! ARK: Survival Ascended live control over RCON.
//!
//! ARK has no REST API; admin actions go through RCON commands (see the ASA admin
//! command reference). Connection details come from `GameUserSettings.ini`
//! `[ServerSettings]` — `RCONPort` (default 27020) and `ServerAdminPassword`.
//! RCON must be enabled in the ini (`RCONEnabled=True`).

use std::path::Path;
use std::time::Duration;

use crate::rcon::RconClient;
use crate::rest::Player;

use super::config;

const RCON_TIMEOUT: Duration = Duration::from_secs(6);

/// Open an authenticated RCON connection using the install's config.
fn client(install_dir: &Path) -> Result<RconClient, String> {
    let port = config::server_setting(install_dir, "RCONPort")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(27020);
    let password = config::server_setting(install_dir, "ServerAdminPassword")
        .filter(|s| !s.trim().is_empty())
        .ok_or("No ServerAdminPassword set in GameUserSettings.ini — RCON needs one.")?;
    RconClient::connect("127.0.0.1", port, &password, RCON_TIMEOUT)
}

pub fn players(install_dir: &Path) -> Result<Vec<Player>, String> {
    let out = client(install_dir)?.exec("ListPlayers")?;
    Ok(parse_players(&out))
}

pub fn announce(install_dir: &Path, message: &str) -> Result<(), String> {
    client(install_dir)?.exec(&format!("ServerChat {message}")).map(|_| ())
}

pub fn kick(install_dir: &Path, user_id: &str) -> Result<(), String> {
    client(install_dir)?.exec(&format!("KickPlayer {user_id}")).map(|_| ())
}

pub fn ban(install_dir: &Path, user_id: &str) -> Result<(), String> {
    client(install_dir)?.exec(&format!("BanPlayer {user_id}")).map(|_| ())
}

pub fn unban(install_dir: &Path, user_id: &str) -> Result<(), String> {
    client(install_dir)?.exec(&format!("UnbanPlayer {user_id}")).map(|_| ())
}

pub fn save(install_dir: &Path) -> Result<(), String> {
    client(install_dir)?.exec("SaveWorld").map(|_| ())
}

/// Broadcast a warning, then save + shut the server down. ARK's `DoExit` saves
/// and exits; the `seconds` warning is best-effort via a broadcast first.
pub fn shutdown(install_dir: &Path, _seconds: i64, message: &str) -> Result<(), String> {
    let mut c = client(install_dir)?;
    if !message.trim().is_empty() {
        let _ = c.exec(&format!("ServerChat {message}"));
    }
    let _ = c.exec("SaveWorld");
    c.exec("DoExit").map(|_| ())
}

/// Parse `ListPlayers` output. ARK returns lines like `0. PlayerName, 7656119…`
/// (index, name, platform id); anything that doesn't match (e.g. "No Players
/// Connected.") is ignored.
fn parse_players(out: &str) -> Vec<Player> {
    out.lines()
        .filter_map(|line| {
            let after_index = line.split_once('.')?.1.trim();
            let (name, id) = after_index.rsplit_once(',')?;
            let name = name.trim();
            let id = id.trim();
            if name.is_empty() || id.is_empty() {
                return None;
            }
            Some(Player {
                name: name.to_string(),
                user_id: id.to_string(),
                ..Default::default()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listplayers_output() {
        let out = "0. Rhyse, 76561198012345678\n1. Buddy, 76561198087654321\n";
        let players = parse_players(out);
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].name, "Rhyse");
        assert_eq!(players[0].user_id, "76561198012345678");
        assert_eq!(players[1].name, "Buddy");
    }

    #[test]
    fn ignores_no_players_message() {
        assert!(parse_players("No Players Connected.\n").is_empty());
        assert!(parse_players("").is_empty());
    }
}
