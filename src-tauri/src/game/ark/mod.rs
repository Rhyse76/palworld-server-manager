//! ARK: Survival Ascended adapter. Verified metadata + config format from a real
//! server; see `docs/ark-reference.md`. Not yet selectable at runtime (per-profile
//! game selection is a later step) — but fully wired behind the `Game` trait.

use std::path::Path;

use crate::config::ConfigField;

use super::{Game, GameSpec, LiveControl, ModsKind};

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
    // ActiveMods is a comma-separated CurseForge project-id list in [ServerSettings];
    // ARK downloads/updates the mod content itself from the ids via `-mods=`/`-allowcfcore`,
    // caching it under cache_dir_rel (confirmed against a real install, 2026-07: e.g.
    // `.../Mods/83374/940975_8362419/`, the leading number an opaque session/list-hash dir).
    mods: ModsKind::CurseForgeIds {
        active_key: "gus|[ServerSettings]|ActiveMods#0",
        cache_dir_rel: "ShooterGame/Binaries/Win64/ShooterGame/Mods",
        // 83374 is CurseForge's catalog id for ARK: Survival Ascended. Backed by the
        // same real install referenced above — its Mods cache folder is literally
        // named `Mods/83374/<mod-id>_<file-id>/`, matching CurseForge's own
        // `<gameId>/<modId>_<fileId>` cache layout convention.
        curseforge_game_id: 83374,
    },
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
