//! Game-adapter layer.
//!
//! Each supported dedicated server is described by a [`GameSpec`] (static metadata:
//! Steam app id, launcher/process names, config/saves/mods paths, ports) and a
//! [`Game`] implementation (behavior that genuinely varies between games). The rest
//! of the app drives games through this trait, so adding a game = adding an adapter
//! rather than branching on game identity anywhere.
//!
//! See `docs/multi-game.md` for the full design and migration plan.

// Migration in progress: the engine now routes launcher/process/paths/port/app-id
// through the spec. Still unconsumed until later steps: `id` and `display_name`
// (per-profile game selection + UI labels) and `live_control` (capability gating).
#![allow(dead_code)]

mod palworld;

/// How a game exposes live control (players, kick/ban, announce) while running.
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
}

/// The currently active game.
///
/// Single-game for now (Palworld). The multi-game refactor will resolve this from
/// the active server profile (each profile pins a game); keeping it behind one
/// function means callers don't change when that happens.
pub fn active() -> &'static dyn Game {
    &palworld::Palworld
}
