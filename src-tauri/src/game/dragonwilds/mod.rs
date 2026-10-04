//! RuneScape: Dragonwilds adapter. **Verified against a real install and a live
//! server run (2026-10-04)** — see `docs/dragonwilds-reference.md` for the per-field
//! confidence breakdown and the open items a first live shakedown has to settle.
//!
//! Confirmed straight from Steam (`steamcmd +app_info_print 4019830`, 2026-10-04):
//! the app id, the install folder name (`RuneScape Dragonwilds Dedicated Server`) and
//! the Windows launcher (`RSDragonwildsServer.exe`). Everything else comes from
//! Jagex's own how-to plus the official wiki.
//!
//! Config is one small Unreal ini (`DedicatedServer.ini`) holding five settings. No
//! live-control protocol (admin actions happen in-game, via the Server Management
//! tab) and no documented mod system for the dedicated server.
//!
//! The game port is NOT in the ini — it's the `-port=` launch argument (default
//! 7777/UDP). We pass none, so a profile that needs another port adds `-port=7778`
//! through its "Extra launch arguments" box.

use std::path::Path;

use crate::config::ConfigField;

use super::{Game, GameSpec, LiveControl, ModsKind};

mod config;

pub struct Dragonwilds;

static SPEC: GameSpec = GameSpec {
    id: "dragonwilds",
    display_name: "RuneScape: Dragonwilds",
    steam_app_id: "4019830", // confirmed via steamcmd app_info_print
    server_launcher: "RSDragonwildsServer.exe", // confirmed via steamcmd app_info_print
    // Confirmed live: the launcher starts
    // `RSDragonwilds/Binaries/Win64/RSDragonwildsServer-Win64-Shipping.exe`. Kept to
    // the `RSDragonwildsServer` prefix on purpose — a broader `RSDragonwilds*` would
    // also match (and `taskkill`) the game *client* running on the same PC.
    process_match: "IMAGENAME eq RSDragonwildsServer*",
    // Not "Shipping": `tasklist` cuts image names off at 25 characters, which for
    // this exe is exactly `RSDragonwildsServer-Win64`. Still excludes the launcher.
    process_marker: "RSDragonwildsServer-Win64",
    config_rel: "RSDragonwilds/Saved/Config/WindowsServer/DedicatedServer.ini",
    default_config: None,
    saves_rel: "RSDragonwilds/Saved/SaveGames",
    mods: ModsKind::None,
    default_game_port: 7777,
    live_control: LiveControl::None,
    // Confirmed live: logs "Engine exit requested (reason: ConsoleCtrl RequestExit)"
    // and is gone within ~2s. Plain Ctrl+C is ignored.
    exits_on_console_break: true,
};

impl Game for Dragonwilds {
    fn spec(&self) -> &'static GameSpec {
        &SPEC
    }

    fn launch_args(&self, _install_dir: &Path) -> Vec<String> {
        // Jagex documents `-log -NewConsole`. We already give the server its own
        // console (or a hidden one), so only `-log` is passed — confirmed enough for
        // a stable live run.
        vec!["-log".to_string()]
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
