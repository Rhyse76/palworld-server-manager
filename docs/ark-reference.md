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
- ⏭️ **Remaining for ARK:**
  1. **Live control behind a trait** — wrap `rest.rs` (Palworld) and `rcon.rs` (ARK) in a
     `LiveControlClient` trait; ARK's players/kick/ban/announce go over RCON commands
     (`ListPlayers`, `KickPlayer`, `BanPlayer`, `Broadcast`, `SaveWorld`, `DoExit`). Also an
     "enable RCON" helper (set `RCONEnabled=True` + `ServerAdminPassword` in the ini).
  2. **~11 GB dedicated-server download** (app 2430930) for the final live shakedown: SteamCMD install +
     `ArkAscendedServer.exe` start/stop detection + an RCON round-trip against a running server.
