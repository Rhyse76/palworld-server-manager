//! Valheim adapter. Verified against a real install (2026-09-14):
//! `C:\Program Files (x86)\Steam\steamapps\common\Valheim dedicated server`
//! (Steam appmanifest confirms app id `896660`). See `docs/valheim-reference.md`
//! for the full confidence breakdown and open items, and `docs/multi-game.md`,
//! which called this out as a candidate 4th game.
//!
//! Unlike Palworld/ARK/Enshrouded, the vanilla dedicated server keeps NO structured
//! settings file of its own — every setting is an inline CLI flag on the single
//! `valheim_server ...` line inside `start_headless_server.bat`. So here that batch
//! file doubles as the "config file": `config.rs` rewrites that one line, and
//! `launch_args` reads the same file to build the actual command line (the same
//! relationship ARK's ini-sourced `launch_args` has to its config).
//!
//! `saves_rel` is `"saves"` **by construction, not by game default** — the real
//! server has no `-savedir` and saves outside the install dir entirely (under
//! `%userprofile%\AppData\LocalLow\IronGate\Valheim\worlds_local\<world>`, confirmed
//! against Rhyse's live test server). `config::write`/`launch_args` force a
//! `-savedir` pointing at `<install_dir>\saves` so backups/saves management works
//! like every other adapter — see `config.rs` module docs for the migration caveat
//! this creates for already-running manual installs.

use std::path::Path;

use crate::config::ConfigField;

use super::{Game, GameSpec, LiveControl, ModsKind};

mod config;

pub struct Valheim;

static SPEC: GameSpec = GameSpec {
    id: "valheim",
    display_name: "Valheim",
    // Confirmed via C:\...\Steam\steamapps\appmanifest_896660.acf on a real install.
    steam_app_id: "896660",
    server_launcher: "valheim_server.exe",
    process_match: "IMAGENAME eq valheim_server.exe",
    process_marker: "valheim_server",
    config_rel: "start_headless_server.bat", // confirmed real filename
    default_config: None,
    saves_rel: "saves", // enforced by config.rs, not the game's own default — see above
    mods: ModsKind::None, // TODO: BepInEx plugin folder support, revisit later
    default_game_port: 2456, // confirmed (real file's -port value)
    live_control: LiveControl::None, // no built-in RCON/admin API on the vanilla server
};

impl Game for Valheim {
    fn spec(&self) -> &'static GameSpec {
        &SPEC
    }

    fn read_config(&self, install_dir: &Path) -> Result<Vec<ConfigField>, String> {
        config::read(install_dir)
    }

    fn write_config(&self, install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
        config::write(install_dir, fields)
    }

    fn import_config(&self, path: &Path) -> Result<Vec<ConfigField>, String> {
        config::import(path)
    }

    fn launch_args(&self, install_dir: &Path) -> Vec<String> {
        config::launch_args(install_dir)
    }
}
