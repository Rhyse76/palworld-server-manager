# RuneScape: Dragonwilds — dedicated server reference

**File layout verified against a real install (2026-10-04, 4.7 GB); the server has not been started yet**, so anything that only exists after a first run is still from documentation.
Sources: Steam's own app metadata (`steamcmd +app_info_print 4019830`), Jagex's
[dedicated servers how-to](https://dragonwilds.runescape.com/news/how-to-dedicated-servers),
the [official wiki](https://dragonwilds.runescape.wiki/w/Dedicated_Servers), and hosting-
provider guides for the literal ini text.

## GameSpec values

| Field | Value | Confidence |
|---|---|---|
| `steam_app_id` | `4019830` ("RuneScape: Dragonwilds Dedicated Server", free tool, anonymous login) | confirmed — Steam app metadata |
| `server_launcher` | `RSDragonwildsServer.exe` (install root) | confirmed — Steam app metadata's Windows launch entry. Jagex's how-to says `RSDragonwilds.exe`; Steam's own launch config wins. |
| `process_match` / `process_marker` | `IMAGENAME eq RSDragonwildsServer*` / `RSDragonwildsServer-Win64` | exe name confirmed on disk (`RSDragonwilds/Binaries/Win64/RSDragonwildsServer-Win64-Shipping.exe`); detection itself not yet exercised live. Deliberately not `RSDragonwilds*`: that would also match, and force-stop, the game client on the same PC. The marker is the name as `tasklist` prints it — it truncates image names to 25 characters, so `Shipping` never appears. |
| `config_rel` | `RSDragonwilds/Saved/Config/WindowsServer/DedicatedServer.ini` | documented (Jagex) |
| `saves_rel` | `RSDragonwilds/Saved/SaveGames` | documented (Jagex: the server loads the latest `.sav` there), but the wiki also lists `%LOCALAPPDATA%\RSDragonwilds\Saved\Savegames` — likely the client's path, **needs checking** |
| `default_game_port` | `7777` UDP | documented |
| `live_control` | `None` | no RCON/REST documented; admin actions are in-game (Server Management tab, unlocked by `AdminPassword`) |
| `mods` | `ModsKind::None` | no documented mod system for the dedicated server |

Steam's install folder name is `RuneScape Dragonwilds Dedicated Server` (used as the first
manual-location guess in `detect.rs`).

## Config: one small ini

```ini
[SectionsToSave]
bCanSaveAllSections=true

[/Script/Dominion.DedicatedServerSettings]
OwnerId=<32-char Player ID from the in-game Settings menu>
ServerName=My Community
DefaultWorldName=Rune Valley
AdminPassword=<required>
WorldPassword=
```

- `OwnerId`, `ServerName`, `DefaultWorldName`, `AdminPassword` are **mandatory** — the
  server won't start without them. `WorldPassword` is optional (blank = open world).
- So `config::read` returns the five fields blank when the file doesn't exist yet (as long
  as the launcher is installed) and `config::write` creates the file, letting the user fill
  them in before the first start.
- The section header and the `[SectionsToSave]` preamble come from a hosting-provider guide,
  not Jagex — **unverified** until seen in a real generated file.
- Jagex: "Changing those values while the server is running will result in those changes
  being lost" — the Config page's stop-the-server-first guard is enabled for this game.

## Launch

Jagex documents `-log -NewConsole`. The adapter passes only `-log`, since `server::start`
already gives the process its own console (same as ARK).

The game port is the `-port=<n>` launch argument, not an ini key. The adapter passes none
(default 7777); a second server on the same PC needs `-port=7778` in the profile's "Extra
launch arguments".

## Open items for the first live shakedown

1. Confirm start/stop/status actually work against the running process.
2. Where saves actually land on Windows (install dir vs `%LOCALAPPDATA%`) — decides whether
   backups work as-is.
3. A fresh install ships **no** `DedicatedServer.ini` and no `RSDragonwilds/Saved` folder at all (confirmed). Check what the server generates on first run and whether it matches the layout above.
4. Whether `-NewConsole` is needed for a stable console (Palworld's console build crashed
   without a real console).
5. Connect page / UPnP / firewall always use 7777 for this game — there is no config field to
   read a custom port from. Needs a per-profile port setting if non-default ports matter.
6. Graceful shutdown: none documented, so stop is a hard `taskkill`. Check whether that risks
   save corruption (cf. the ARK SQLite incident) and whether a console close is safer.
7. Player cap is 6 (as of 0.11); not configurable in the ini.
