//! Game-adapter layer.
//!
//! Each supported dedicated server is described by a [`GameSpec`] (static metadata:
//! Steam app id, launcher/process names, config/saves/mods paths, ports) and a
//! [`Game`] implementation (behavior that genuinely varies between games). The rest
//! of the app drives games through this trait, so adding a game = adding an adapter
//! rather than branching on game identity anywhere.
//!
//! See `docs/multi-game.md` for the full design and migration plan.

use std::path::Path;

use crate::config::ConfigField;

mod ark;
mod palworld;

/// How a game exposes live control (players, kick/ban, announce) while running.
// `None` isn't constructed until the Enshrouded adapter lands.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LiveControl {
    /// HTTP REST admin API (Palworld).
    RestApi,
    /// Source RCON protocol (ARK: SA).
    Rcon,
    /// No live-control protocol (Enshrouded) — install/start/stop/backup only.
    None,
}

/// Static, compile-time metadata describing a game's dedicated server.
pub struct GameSpec {
    /// Stable slug, e.g. `"palworld"`.
    pub id: &'static str,
    /// Human-facing name, e.g. `"Palworld"`.
    pub display_name: &'static str,
    /// Steam app id for the dedicated server (anonymous SteamCMD download).
    pub steam_app_id: &'static str,
    /// Launcher executable in the install root, e.g. `"PalServer.exe"`.
    pub server_launcher: &'static str,
    /// `tasklist`/`taskkill` image filter matching the server process(es).
    pub process_match: &'static str,
    /// Substring the running process line must contain to count as "up"
    /// (e.g. `"Shipping"`), guarding against a lingering launcher.
    pub process_marker: &'static str,
    /// Install-relative path to the live config file.
    pub config_rel: &'static str,
    /// Install-root default-config filename, if the game ships one.
    pub default_config: Option<&'static str>,
    /// Install-relative path to the world/save directory.
    pub saves_rel: &'static str,
    /// Install-relative mods directory, if the game supports drop-in mods.
    pub mods_rel: Option<&'static str>,
    /// Default game port when config doesn't specify one.
    pub default_game_port: u16,
    /// Live-control capability.
    pub live_control: LiveControl,
}

/// A supported game. Static metadata via [`GameSpec`]; per-game behavior (config
/// parsing, live-control client) will be added as methods as the refactor proceeds.
pub trait Game: Send + Sync {
    fn spec(&self) -> &'static GameSpec;

    /// Command-line arguments to launch the server with, derived from the install
    /// as needed (e.g. ARK's `Map?listen?Port=...?RCONEnabled=True` + `-flags`).
    /// Defaults to none — Palworld's launcher needs no arguments.
    fn launch_args(&self, _install_dir: &Path) -> Vec<String> {
        Vec::new()
    }

    /// Read the game's config file(s) into a unified field list, with shipped
    /// defaults merged in where the game provides them.
    fn read_config(&self, install_dir: &Path) -> Result<Vec<ConfigField>, String>;

    /// Write a unified field list back to the game's config file(s).
    fn write_config(&self, install_dir: &Path, fields: &[ConfigField]) -> Result<(), String>;

    /// Parse a game-native config file (e.g. an imported settings file) into fields.
    fn import_config(&self, path: &Path) -> Result<Vec<ConfigField>, String>;
}

/// Resolve a game adapter by its id, or `None` if unknown.
pub fn by_id(id: &str) -> Option<&'static dyn Game> {
    match id {
        "palworld" => Some(&palworld::Palworld),
        "ark-sa" => Some(&ark::ArkSurvivalAscended),
        _ => None,
    }
}

/// The currently active game.
///
/// Still resolves to Palworld for now; the per-profile-game-selection step will
/// pass the active profile's game id here. Keeping it behind one function means
/// callers don't change when that happens.
pub fn active() -> &'static dyn Game {
    by_id("palworld").expect("palworld adapter is always registered")
}
