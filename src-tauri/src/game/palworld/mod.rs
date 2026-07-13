//! Palworld adapter — the first game. Static metadata lives in `SPEC`; the
//! Palworld config format (the `OptionSettings=(...)` INI blob) lives in the
//! `config` submodule.

use std::path::Path;

use crate::config::ConfigField;

use super::{Game, GameSpec, LiveControl, ModsKind};

mod config;

pub struct Palworld;

static SPEC: GameSpec = GameSpec {
    id: "palworld",
    display_name: "Palworld",
    steam_app_id: "2394010",
    server_launcher: "PalServer.exe",
    process_match: "IMAGENAME eq PalServer*",
    process_marker: "Shipping",
    config_rel: "Pal/Saved/Config/WindowsServer/PalWorldSettings.ini",
    default_config: Some("DefaultPalWorldSettings.ini"),
    saves_rel: "Pal/Saved/SaveGames",
    mods: ModsKind::LocalFiles("Pal/Content/Paks/~mods"),
    default_game_port: 8211,
    live_control: LiveControl::RestApi,
};

impl Game for Palworld {
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
