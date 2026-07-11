# Multi-game refactor — design doc

**Goal:** turn the Palworld-specific manager into **one app that manages many dedicated
servers** (Palworld → ARK: Survival Ascended → Enshrouded, maybe Valheim) via a
**game-adapter architecture**. One app, one installer, one updater — NOT separate apps.
Final product name: **RhyseGaming Server Manager** (rename happens with this refactor; see
the backlog in [../CLAUDE.md](../CLAUDE.md)).

## Guiding principles

1. **Shared engine, thin adapters.** ~60–70% of the code (SteamCMD, process lifecycle,
   backups, automation/watchdog, Discord, connectivity/UPnP, metrics, updater, UI shell) is
   game-agnostic and stays. Each game contributes a small adapter for the ~30% that differs.
2. **Games never `if game == "palworld"`.** All game-specific behavior lives behind the
   `Game` trait. The engine calls the trait; it never branches on game identity.
3. **No behavior change for Palworld during extraction.** Step by step, Palworld becomes
   "adapter #1" with identical behavior; only after that do we add new games.
4. **Adapters advertise capabilities.** Live control (players/kick/announce) varies wildly per
   game; the UI shows only what the active adapter supports.

## What is shared vs game-specific

Seam inventory (from the current code) — everything hard-coded to Palworld today:

| Concern | Today (hard-coded) | Becomes (adapter-provided) |
|---|---|---|
| Steam app id | `steamcmd.rs`/`updates.rs` `2394010` | `spec.steam_app_id` |
| Server launcher exe | `server.rs` `PalServer.exe` | `spec.server_launcher` |
| Running-process match | `server.rs` `PalServer*` + `Shipping` | `spec.process_match` + `process_marker` |
| Config file path | `config.rs` `Pal/Saved/Config/WindowsServer/PalWorldSettings.ini` | `spec.config_rel` |
| Config defaults file | `config.rs` `DefaultPalWorldSettings.ini` | `spec.default_config` |
| Config **format** | INI `OptionSettings=(...)` blob | `Game::read_config`/`write_config` (per-game parser) |
| Saves dir | `saves.rs`/`backups.rs` `Pal/Saved/SaveGames` | `spec.saves_rel` |
| Mods dir | `mods.rs` `Pal/Content/Paks/~mods` | `spec.mods_rel` (Option) |
| Ban list | `bans.rs` `.../banlist.txt` | adapter (Option) |
| Live control | `rest.rs` (REST API) | `spec.live_control` + a client the adapter provides |
| Default game port | `network.rs` `8211` | `spec.default_game_port` |

## The `Game` trait (Rust)

Static metadata is a plain `GameSpec` struct; behavior that genuinely varies is trait methods.

```rust
pub enum LiveControl { RestApi, Rcon, None }

pub struct GameSpec {
    pub id: &'static str,               // "palworld"
    pub display_name: &'static str,     // "Palworld"
    pub steam_app_id: &'static str,     // "2394010"
    pub server_launcher: &'static str,  // "PalServer.exe"
    pub process_match: &'static str,    // tasklist "IMAGENAME eq PalServer*"
    pub process_marker: &'static str,   // require this substring ("Shipping")
    pub config_rel: &'static str,       // relative path to the live config file
    pub default_config: Option<&'static str>,
    pub saves_rel: &'static str,
    pub mods_rel: Option<&'static str>,
    pub default_game_port: u16,
    pub live_control: LiveControl,
}

pub trait Game: Send + Sync {
    fn spec(&self) -> &'static GameSpec;
    // Later steps add per-game behavior here, e.g.:
    // fn read_config(&self, dir: &Path) -> Result<Vec<ConfigField>, String>;
    // fn write_config(&self, dir: &Path, fields: &[ConfigField]) -> Result<(), String>;
    // fn live(&self) -> Option<Box<dyn LiveControlClient>>;
}

/// Resolve the active game. Single-game for now; the refactor will read this from
/// the active profile (each profile pins a game).
pub fn active() -> &'static dyn Game;
```

### Config: the schema-driven page (the biggest piece)

Config formats differ hard: Palworld = INI `OptionSettings` blob; ARK = `GameUserSettings.ini`
+ `Game.ini` + launch args; Enshrouded = JSON. The Config **page** must stop being hard-coded
and instead render from a **schema the adapter provides** — a list of `{key, label, type,
default, group, help}` — with the adapter owning parse↔write. Existing `config.rs` becomes the
Palworld adapter's config implementation; the generic `ConfigField` model already exists and is
a good starting shape.

**The config abstraction must NOT assume a single file.** A game's settings can span multiple
sources, and the adapter maps each schema field to the right one:

- **Palworld** — one file, the `OptionSettings=(...)` blob.
- **ARK: SA** — three sources: `GameUserSettings.ini` (main), `Game.ini` (advanced multipliers),
  and **launch/startup args** (some options exist ONLY as command-line flags). The ARK adapter
  reads/writes all three and, at start time, composes the launch-arg subset into the server
  command line (via `server::start`, which the adapter also parameterizes).
- **Enshrouded** — one JSON file.

So `GameSpec.config_rel` is just the *primary* file (used for detection/"open config folder");
the real parse↔write is a per-adapter method that returns/consumes the unified `ConfigField`
list regardless of how many underlying files/args it touches. The user always sees ONE grouped
settings list, never the file split.

### Live control capability matrix

| Game | Protocol | Players/kick/ban | Announce | Notes |
|---|---|---|---|---|
| Palworld | REST API (8212) + RCON | ✅ | ✅ | richest; already built |
| ARK: SA | **RCON only** | ✅ (via RCON) | ✅ | needs the backlogged `rcon.rs` |
| Enshrouded | **none** | ❌ | ❌ | install/update/start/stop/backup/config/automation only |

The UI hides unsupported controls based on `spec.live_control` + finer per-feature flags.

## Migration order

1. ✅ **Scaffold `game` module** — `GameSpec` + `Game` trait + `Palworld` adapter + `active()`.
   Routed the Steam app id through `spec()`. *No behavior change.*
2. ✅ **Route the rest of the engine** through `spec()` — `server.rs`, `config.rs`/`detect.rs` paths,
   `saves.rs`, `backups.rs`, `mods.rs`, `bans.rs`, `network.rs` default port.
3. ✅ **Config parse/write behind the trait** — Palworld INI format moved into the adapter
   (`game/palworld/config.rs`); `config.rs` is shared-only and delegates via `read_config`/
   `write_config`/`import_config`. *Remaining polish (not blocking):* per-game field **schema**
   (labels/groups/help) for a nicer Config page, and game-aware copy (the "PalWorldSettings.ini"
   labels in `ConfigPage.tsx`).
4. **Live control behind a trait** — `rest.rs` becomes Palworld's live client; wrap them in a
   trait. (`rcon.rs` client ✅ built + unit-tested; the *trait abstraction* is designed alongside
   the ARK adapter so it's shaped against a real second case.)
5. ✅ **Per-profile game selection** — each profile pins a `game`; a global `game::set_active`
   (synced from the active profile on startup + add/switch/delete) drives `active()`; game picker
   in the Add-profile flow; per-profile + sidebar game badges; `games_list` command. Switching
   profile now switches the whole app to that game. *Remaining polish:* game picker in the
   first-run wizard, and game-aware copy on the remaining pages.
6. **Add ARK adapter** — ✅ `GameSpec` + config parser done. Remaining: `launch_args`, live control
   behind a trait (RCON), config-UI schema/labels, then the download + live shakedown. Then
   **Enshrouded** (~2–3 days).
7. **Rebrand** — repo, updater endpoint, installer name (keep bundle identifier stable!), site,
   Store listing, in-app name → RhyseGaming Server Manager.
   - **Microsoft Store (decided 2026-07):** first submission goes out NOW as
     **"Server Manager for Palworld (Unofficial)"** (individual account, publisher display name
     "Rhyse"). At the rebrand, **rename the DISPLAY name on the SAME Partner Center product** to
     "RhyseGaming Server Manager" and upload the new package — do NOT create a new product. The
     Package Identity Name is permanent (and invisible to users), so keeping the same product means
     existing Store users get an upgrade, not a duplicate, and reviews/ratings carry over. This is
     the Store equivalent of "keep the bundle identifier stable".

## Frontend implications

- Nav/pages stay, but labels/visibility come from the active game's capabilities.
- Config page: schema-driven renderer instead of Palworld-specific fields.
- A game picker (per profile) + game shown in the sidebar/brand.
- Copy that says "Palworld" becomes game-aware (the *product* is game-neutral; per-page copy uses
  the active game's display name).

## Non-goals (for now)

- Linux/host-agnostic support (Windows-first stays).
- Web/remote access (separate big-bet backlog item).
