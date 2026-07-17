# ARK: Survival Ascended — dedicated server reference

Verified facts + a real-world launch line to build the ARK adapter against. No
secrets here (passwords live in `GameUserSettings.ini`, kept out of the repo).

## GameSpec values (verified)

| Field | Value |
|---|---|
| `steam_app_id` | `2430930` (ARK: SA Dedicated Server) |
| `server_launcher` | `ShooterGame/Binaries/Win64/ArkAscendedServer.exe` |
| `process_match` / `process_marker` | `IMAGENAME eq ArkAscendedServer.exe` / `ArkAscendedServer` |
| `config_rel` (primary) | `ShooterGame/Saved/Config/WindowsServer/GameUserSettings.ini` (+ `Game.ini`) |
| `saves_rel` | `ShooterGame/Saved/SavedArks` |
| `default_game_port` | 7777 (query 27015 default; RCON 27020) |
| `live_control` | RCON (enable via `RCONEnabled=True` + `ServerAdminPassword` in the ini) |
| `default_config` / `mods_rel` | None (no shipped defaults file; mods are launch-arg/CurseForge) |

## Real launch line (from the user's running server)

```
TheIsland_WP?listen -Port=7777 -QueryPort=27025 -RCONPort=27020 -WinLiveMaxPlayers=70
-ServerPlatform=ALL -culture=en -NoBattlEye -exclusivejoin -mods=940975,927090
-ClusterDirOverride=C:\ARK\Cluster -allowcfcore -cfcoretimeout=30 -log
```

### Structure (this is how `launch_args` must assemble the command line)

`<Map>?<url-options> -<dash-flags…>` — the map is the first positional token, followed
by `?`-joined URL options (no spaces), then space-separated `-` flags. As a `Vec<String>`
for `Command::args`, each token is one element:
`["TheIsland_WP?listen", "-Port=7777", "-QueryPort=27025", …]`.

### Token breakdown

| Token | Meaning | Source |
|---|---|---|
| `TheIsland_WP` | map id (positional, required) | config choice |
| `?listen` | run as a listen/queryable server | ~always on |
| `-Port=7777` | game port (UDP) | config (also in ini) |
| `-QueryPort=27025` | Steam query port (note: NOT the 27015 default) | config |
| `-RCONPort=27020` | RCON port | config (RCON enabled in ini) |
| `-WinLiveMaxPlayers=70` | max players (ASA replaced `?MaxPlayers`) | config |
| `-ServerPlatform=ALL` | crossplay (PC/Xbox/PS) | config |
| `-culture=en` | language | config |
| `-NoBattlEye` | disable BattlEye | toggle |
| `-exclusivejoin` | whitelist-only mode | toggle |
| `-mods=940975,927090` | CurseForge mod ids (comma list) | mod manager |
| `-ClusterDirOverride=C:\ARK\Cluster` | shared cluster save dir | config (advanced) |
| `-allowcfcore` / `-cfcoretimeout=30` | CurseForge core + timeout | mods |
| `-log` | log to console | ~always on |

### Adapter implications

- **Launch args are built from config, not hardcoded.** The user confirmed the exact line
  depends on how the inis are set up. Some values live in `GameUserSettings.ini` (Port,
  QueryPort, RCONPort, max players) AND can be overridden on the command line; some exist
  **only** as command-line flags (map, `-mods`, `-NoBattlEye`, `-ClusterDirOverride`,
  `-ServerPlatform`, `-exclusivejoin`). So the ARK config model must cover three homes:
  `GameUserSettings.ini`, `Game.ini`, and launch flags — exactly the multi-source design.
- **Mods are a launch-arg list** (`-mods=`), not a drop-in folder → `mods_rel: None`; ARK mod
  management is its own feature (edit the `-mods` list), unlike Palworld's `.pak` files.
- **Ports aren't all defaults** (query 27025 here) → never assume defaults; read from config.
- `-log` gives ARK a real console log (unlike Palworld) — a future ARK-specific log feature.

## Confirmed INI structure (from the user's real sample files)

Sample files received at `C:\Users\Rhyse\Documents\ark-samples\` (kept out of the repo —
`GameUserSettings.ini` holds passwords). Structure is **standard line-based INI**, NOT
Palworld's single-line `OptionSettings` blob:

- **`GameUserSettings.ini`** sections: `[ServerSettings]` (~144 keys — the main server config),
  `[SessionSettings]` (session name), `[/Script/Engine.GameSession]` (MaxPlayers), `[ModSettings]`,
  `[MessageOfTheDay]`, `[ScalabilityGroups]`, `[/Script/ShooterGame.ShooterGameUserSettings]`
  (~300 lines — graphics/client, **filter out of the server config UI**),
  `[/Script/Engine.GameUserSettings]`, `[Startup]`, `[OmegaTeleporters]` (a mod's settings).
- **`Game.ini`** section: `[/Script/ShooterGame.ShooterGameMode]` (gameplay multipliers).

### Parser design notes (these differ from Palworld — get them right)

1. **Line-based**: `[Section]` headers then `Key=Value` lines, one per line. Split each line on the
   **first `=`** only; the rest is the value verbatim. (Much simpler than Palworld's comma-split blob.)
2. **Multi-section across two files** — each field must remember its `(file, section)` to write back.
3. **Duplicate keys are legal and must be preserved** — arrays like `OverridePlayerLevelEngramPoints`,
   `ConfigOverrideItemMaxQuantity`, `LevelExperienceRampOverrides` repeat the same key on many lines.
   The Palworld model uses unique-key `upsert`; **ARK cannot** — it needs order-preserving fields with
   duplicates allowed, which means the frontend's edit-by-key (`update(key, ...)`) also needs a
   per-field id/index, not the bare key.
4. **Indexed keys**: `PerLevelStatsMultiplier_DinoWild[0]=1.0` — treat the whole `Key[i]` as the key
   string (they're then unique); no special array handling needed for these.
5. **Complex parenthesized values** exist, e.g.
   `ConfigAddNPCSpawnEntriesContainer=(NPCSpawnEntriesContainerClassString="...",NPCSpawnEntries=(...))`
   — because parsing is line-based and splits on the first `=`, the nested `=`/`,`/`()` in the value
   are preserved verbatim. Do NOT try to parse the structure.
6. **Comments**: lines starting with `;` (e.g. `;METADATA=(...)` on line 1) — skip on read, and
   decide whether to preserve on write (safest: keep unknown/comment lines as-is on round-trip).
7. Value types: `True`/`False` bools, ints, floats — same `classify` logic as Palworld works.

**Implication for the shared model:** ARK's need for duplicate keys + `(file, section)` means the
ARK adapter likely keeps its own richer internal field struct and maps to/from the shared
`ConfigField` for the UI (possibly extending `ConfigField` with an optional stable id). Decide this
at the start of the ARK config work.

## Reference: how an existing ARK manager organizes it (user screenshots)

~26 screenshots of "ARK Ascended Server Manager v0.8.7" in `C:\Users\Rhyse\Documents\ark-samples\`.
Used for **information architecture only** — what to surface and how to group it. We build our own
layout in the RhyseGaming look; not cloning their visuals.

Takeaways (from a sampled subset — full set reviewed per-group at build time):

- **Per-server tabs** (Dashboard + one tab per server, each showing RUNNING/STOPPED + player count)
  → validates our per-profile model: each profile = a server, shown with live status.
- **Server header bar**: name + online pill + stat chips (Map / Port / Players / Mode / Uptime) +
  Start / Stop / Restart / Update / Folder buttons + settings search + a "Store (N)" **staged-changes**
  badge (edits queued, applied on restart). Consider staging config edits similarly.
- **Config split into ~16 category tabs** — the schema-driven Config page needs a `group` field to
  organize ARK's hundreds of settings into: Server Settings, Rules, Player Settings, Dino Settings,
  Environment, Chat & Notifications, HUD & Visuals, Structures, Backup & Restore, ASA-API, RCON,
  Cluster Settings, Access Control, Overrides, Mod Manager, Maintenance.
- **Control patterns worth adopting (own styling):**
  - Multiplier **sliders with quick-preset buttons** (1x / 5x / 10x / 100x) + a numeric box + range
    label — for the many rate multipliers (Environment / Dino / Player).
  - Boolean rules as a **grid of toggle cards** (highlighted when on).
  - Per-setting and per-section **reset-to-default** (history icon).
  - Global **"search settings"** across all groups.
- **Mod Manager** = dedicated tab that edits the `-mods` id list.
- **RCON** tab: connection status, scheduled commands, admin password auto-read from
  `GameUserSettings.ini`, RCON port — pairs with our `rcon.rs` client.
- **Access Control**: whitelist / admin management by Player ID (ARK GUID), "get from RCON
  `listplayers`" — backs the `-exclusivejoin` whitelist; ties to our RCON client + player list.
- **Maintenance**: open `Game.ini`/`GameUserSettings.ini`/folder, open firewall ports, install certs,
  clean logs, force-clean steamapps, update-available check (build id), auto-restart-on-crash +
  immediate-updates toggles. We already have connectivity/UPnP, update checks, and a crash watchdog —
  good overlap to reuse.

## Adapter build status

- ✅ `GameUserSettings.ini` + `Game.ini` samples at `C:\Users\Rhyse\Documents\ark-samples\` (analyzed).
- ✅ **ARK adapter scaffold + config parser built** (`src-tauri/src/game/ark/`): `GameSpec` with the
  verified values above, and a section-aware INI parser/in-place writer for both files handling the
  duplicate-key arrays, quotes/empties/bools, and comment/format preservation. Registered via
  `game::by_id("ark-sa")`; unit-tested. Fields use composite keys `<file>|<section>|<key>#<occ>`.
- ✅ **`launch_args`** — assembles `<Map>?listen -Port=… -QueryPort=… -RCONPort=… [-mods=…]
  [-exclusivejoin] -log` from `GameUserSettings [ServerSettings]`; `server::start` appends it.
- ✅ **Per-profile game selection** — ARK is selectable; switching profile switches the whole app.
- ✅ **Config UI** — grouped/labeled fields; graphics section filtered out.
- ✅ **Live control over RCON** — `game/ark/live.rs` + `game::live` dispatch (REST vs RCON by
  capability); Dashboard + nav are capability-gated.
- ✅ **LIVE SHAKEDOWN PASSED (2026-07)** — installed the 12 GB server via the app (app 2430930 is
  anonymous), launched with our exact `launch_args` (`TheIsland_WP?listen -Port=7777 -QueryPort=27015
  -RCONPort=27020 -log`), process detection flipped to Running, config parser rendered the real
  generated `GameUserSettings.ini`, and the Dashboard connected over RCON (`ListPlayers`). **The full
  ARK adapter works end-to-end against a real server.**
  - RCON gotcha found + fixed live: ASA ignores empty commands, so our old empty-command "sentinel"
    hung; `rcon::exec` now reads the first packet then drains with a short timeout.
- ✅ **ARK settings catalog** (2026-07) — `src-tauri/src/game/ark/catalog.rs` curates ~120 well-known
  `GameUserSettings.ini [ServerSettings]`/`Game.ini` keys (rates & multipliers, difficulty/PvP,
  player/tribe, dinos & taming, structures, access & whitelist, session/engine/MOTD, gameplay rules),
  each with a shipped-engine default, grouped for Config-page tabs. `config::read` seeds the field
  list from the catalog then overlays live file values on top (mirrors Palworld's defaults overlay).
  `config::apply`/`write` generalized: fields with no existing line yet (catalog-only settings the
  user edits) are inserted into their section via `upsert_section` (also now backs `enable_rcon`'s
  `[ServerSettings]` insert), creating the section if the file doesn't have it. Dynamic array
  settings (per-level stat overrides, item overrides) are intentionally NOT catalogued — they only
  show up if already present in the live file.
- ✅ **"Enable RCON" helper** — `config::enable_rcon` (backed by `upsert_section`) sets
  `RCONEnabled=True` + a generated `ServerAdminPassword`, dispatched via `game::live::enable` →
  the `enable_live_control` Tauri command → the Dashboard's capability-aware "Enable RCON" button.
- ✅ **Mods page unhidden for ARK** (2026-07) — mods are a CurseForge project-id list in
  `ActiveMods` (`[ServerSettings]`), not drop-in files; `GameSpec.mods_rel: Option<&str>` became a
  `ModsKind` enum (`LocalFiles`/`CurseForgeIds`/`None`) so mod *mechanism* drives the UI/backend,
  not just presence. `mods.rs`: `list_ids`/`add_id`/`remove_id` manage the active list (through the
  existing generic `ConfigField` read/write path — reuses the catalog's insert-on-write for a field
  that doesn't exist in a fresh ini yet).
  - **Confirmed mod cache path on a real install**: ARK downloads mod content under
    `ShooterGame/Binaries/Win64/ShooterGame/Mods/<opaque-session-id>/<mod-id>_<file-id>/` (e.g.
    `.../Mods/83374/940975_8362419/` — `940975`/`927090` matched the real server's `ActiveMods`
    exactly). The leading `<opaque-session-id>` folder doesn't need to be understood — `mods.rs`'s
    `delete_cached_files` just scans one level under `cache_dir_rel` for any subfolder starting
    with `<mod-id>_` and removes it, so it's robust regardless of what that id represents or
    whether it changes. `uninstall_id` (Tauri: `mod_id_delete_files`) combines this with
    `remove_id` for the Mods page's "Delete files" button (vs. plain "Remove", which only drops the
    id from the active list and leaves cached files in place for a redownload-free re-add).
- ✅ **Access & Whitelist: exclusive-join + admin ID lists** (2026-07) — `ExclusiveJoin`/
  `AdminListURL` added to the catalog; new `access.rs` manages
  `PlayersExclusiveJoinList.txt` and the admin list (target file resolved from
  `AdminListURL`, defaulting to `ShooterGame/Saved/adminlist.txt`), both plain text,
  one EOS/Steam ID per line — confirmed against Steam ASA discussion threads +
  ark.wiki.gg, not guessed. **Live-verified (2026-07)**: user confirmed the exclusive-join
  list actually gates joins correctly (in list → can join, not in list → blocked).
  **Confirmed gotcha (2026-07, community-documented, not an app bug)**: ARK: SA keeps
  its loaded settings in memory and rewrites `GameUserSettings.ini`/`Game.ini` from that
  snapshot when the server shuts down — silently discarding **any** config edit (not
  just these two fields; the user hit it on `ExclusiveJoin` and `WhitelistOn`
  specifically, but it's a `GameUserSettings.ini`-wide behavior, confirmed by multiple
  hosting-provider docs) made while the server was running, once it's stopped/restarted.
  The exclusive-join/admin **list files** above are unaffected (separate files ARK
  doesn't cache in memory) — only in-ini key/value settings. Fix is UX, not code: the
  Config page and the ARK Mods page (`ActiveMods` is the same ini field) now show a
  warning and disable Save/Add/Remove while the server is running, instead of letting
  edits silently vanish on next restart.
- ⏭️ **Remaining (polish, not blocking function):**
  1. Install-progress bar didn't render for the ARK download (minor UI bug to chase).
- ❌ **CurseForge mod search — decided against (2026-07)**: third-party API access isn't guaranteed
  (CurseForge tightened this post-Overwolf), so it's not worth building against. `ModsPage.tsx`'s
  `CurseForgeIdMods` stays manual add-by-numeric-id (find the id on the mod's CurseForge page).
