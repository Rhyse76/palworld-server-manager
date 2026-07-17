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
use std::sync::RwLock;

use crate::config::ConfigField;

mod ark;
mod enshrouded;
pub mod live;
mod palworld;

/// Ids of all supported games, in display order (for the game picker).
const IDS: &[&str] = &["palworld", "ark-sa", "enshrouded"];

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

/// How a game exposes user-installable mods.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModsKind {
    /// Drop-in files in an install-relative directory (Palworld `.pak`).
    LocalFiles(&'static str),
    /// A comma-separated CurseForge project-id list stored in one config field
    /// (its composite `ConfigField` key), e.g. ARK: SA's `ActiveMods`. The game's own
    /// launcher downloads/updates the mod content itself from the id list into
    /// `cache_dir_rel` (subfolders named `<mod-id>_<file-id>`, one level under an
    /// opaque session/list-hash folder) — we manage which ids are active and can
    /// clear a mod's cached files, but never download/unpack anything ourselves.
    /// `curseforge_game_id` is this game's numeric id in CurseForge's own catalog
    /// (distinct from `cache_dir_rel`'s ids), used to scope mod search.
    CurseForgeIds { active_key: &'static str, cache_dir_rel: &'static str, curseforge_game_id: u32 },
    /// No mod support (Enshrouded, for now).
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
    /// How this game supports mods (drop-in files, a CurseForge id list, or none).
    pub mods: ModsKind,
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
        "enshrouded" => Some(&enshrouded::Enshrouded),
        _ => None,
    }
}

/// Every supported game's spec, in display order — for the game picker.
pub fn all() -> Vec<&'static GameSpec> {
    IDS.iter().filter_map(|id| by_id(id).map(|g| g.spec())).collect()
}

/// Resolve a game adapter by id, falling back to Palworld if unknown — the same
/// fallback `active()` uses, exposed for callers (e.g. the automation scheduler)
/// resolving a specific profile's game rather than the globally active one.
pub fn by_id_or_default(id: &str) -> &'static dyn Game {
    by_id(id).unwrap_or_else(|| by_id("palworld").expect("palworld adapter is always registered"))
}

/// The active game's id. Global because `active()` is called from deep in the
/// engine (no `AppHandle` in scope); the app keeps it in sync with the active
/// profile via `set_active` on startup and whenever the active profile changes.
static ACTIVE_GAME: RwLock<String> = RwLock::new(String::new());

/// Point the engine at a game by id (falls back to Palworld if unknown/unset).
pub fn set_active(id: &str) {
    if let Ok(mut g) = ACTIVE_GAME.write() {
        *g = id.to_string();
    }
}

/// The currently active game (the active profile's game).
pub fn active() -> &'static dyn Game {
    let id = ACTIVE_GAME.read().ok().map(|g| g.clone()).unwrap_or_default();
    by_id_or_default(&id)
}
