//! ARK: Survival Ascended adapter. Verified metadata + config format from a real
//! server; see `docs/ark-reference.md`. Not yet selectable at runtime (per-profile
//! game selection is a later step) — but fully wired behind the `Game` trait.

use std::path::Path;

use crate::config::ConfigField;

use super::{Game, GameSpec, LiveControl};

mod catalog;
pub(super) mod config;
pub mod live;

pub struct ArkSurvivalAscended;

static SPEC: GameSpec = GameSpec {
    id: "ark-sa",
    display_name: "ARK: Survival Ascended",
    steam_app_id: "2430930",
    server_launcher: "ShooterGame/Binaries/Win64/ArkAscendedServer.exe",
    process_match: "IMAGENAME eq ArkAscendedServer.exe",
    process_marker: "ArkAscendedServer",
    config_rel: "ShooterGame/Saved/Config/WindowsServer/GameUserSettings.ini",
    default_config: None, // ARK ships no defaults file; the game generates the ini
    saves_rel: "ShooterGame/Saved/SavedArks",
    mods_rel: None, // mods are a launch-arg CurseForge list, not a drop-in folder
    default_game_port: 7777,
    live_control: LiveControl::Rcon,
};

impl Game for ArkSurvivalAscended {
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
