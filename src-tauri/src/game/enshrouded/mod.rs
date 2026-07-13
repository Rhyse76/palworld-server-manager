//! Enshrouded adapter. Verified against a real install (2026-07, see
//! `docs/enshrouded-reference.md`): the server ships a *complete*
//! `enshrouded_server.json` on first run — unlike Palworld/ARK there's no "missing
//! keys" problem, so no defaults catalog is needed, just a straight JSON round-trip.
//! No live-control protocol and no documented mod system for the dedicated server.

use std::path::Path;

use crate::config::ConfigField;

use super::{Game, GameSpec, LiveControl, ModsKind};

mod config;

pub struct Enshrouded;

static SPEC: GameSpec = GameSpec {
    id: "enshrouded",
    display_name: "Enshrouded",
    steam_app_id: "2278520",
    server_launcher: "enshrouded_server.exe",
    // Not yet live-started through the app — best guess pending a start/stop test
    // (Palworld and ARK both shipped a running process name that differed from the
    // launcher exe name, so this may need correcting).
    process_match: "IMAGENAME eq enshrouded_server.exe",
    process_marker: "enshrouded_server",
    config_rel: "enshrouded_server.json",
    default_config: None, // no separate defaults file; the live file is always complete
    saves_rel: "savegame",
    mods: ModsKind::None, // no documented mod system for the dedicated server
    default_game_port: 15637,
    live_control: LiveControl::None,
};

impl Game for Enshrouded {
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
}
