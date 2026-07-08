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

## Still needed for the adapter build

- `GameUserSettings.ini` + `Game.ini` samples at `C:\Users\Rhyse\Documents\ark-samples\`
  (to build/unit-test the multi-section INI parser against real structure).
- ~11 GB dedicated-server download (app 2430930) only for the final live shakedown
  (SteamCMD install + start/stop detection + an RCON round-trip).
