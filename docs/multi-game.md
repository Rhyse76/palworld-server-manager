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

1. **Scaffold `game` module** — `GameSpec` + `Game` trait + `Palworld` adapter + `active()`.
   Route the trivial constants (Steam app id, later launcher/process/paths) through `spec()`.
   *No behavior change.* ← **starting here**
2. **Route the rest of the engine** through `spec()` — `server.rs`, `config.rs` paths, `saves.rs`,
   `backups.rs`, `mods.rs`, `bans.rs`, `network.rs` default port.
3. **Config becomes schema-driven** — adapter provides the field schema; Config page renders it.
4. **Live control behind a trait** — `rest.rs` becomes Palworld's live client; add `rcon.rs`.
5. **Per-profile game selection** — each server profile pins a game; `active()` reads it; first-run
   wizard asks which game.
6. **Add ARK adapter** (~3–5 days), then **Enshrouded** (~2–3 days).
7. **Rebrand** — repo, updater endpoint, installer name (keep bundle identifier stable!), site,
   Store listing, in-app name → RhyseGaming Server Manager.

## Frontend implications

- Nav/pages stay, but labels/visibility come from the active game's capabilities.
- Config page: schema-driven renderer instead of Palworld-specific fields.
- A game picker (per profile) + game shown in the sidebar/brand.
- Copy that says "Palworld" becomes game-aware (the *product* is game-neutral; per-page copy uses
  the active game's display name).

## Non-goals (for now)

- Linux/host-agnostic support (Windows-first stays).
- Web/remote access (separate big-bet backlog item).
