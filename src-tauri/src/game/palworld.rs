//! Palworld adapter — the first game (`GameSpec` values match the constants that
//! were previously hard-coded across the backend).

use super::{Game, GameSpec, LiveControl};

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
    mods_rel: Some("Pal/Content/Paks/~mods"),
    default_game_port: 8211,
    live_control: LiveControl::RestApi,
};

impl Game for Palworld {
    fn spec(&self) -> &'static GameSpec {
        &SPEC
    }
}
