# Palworld Server Manager

A next-gen desktop GUI for installing, configuring, and running a **Palworld dedicated
server** on Windows. Goal: a polished, reliable alternative to the existing community
managers (which are largely abandoned or partly broken).

This project is **independent** of any other repo on this machine.

## Stack

- **Tauri 2** (Rust backend + web frontend) — small native binaries, builds `.msi` (WiX)
  and `.exe` (NSIS) installers out of the box.
- **Frontend:** React + TypeScript + Vite (`src/`).
- **Backend:** Rust (`src-tauri/src/`), exposed to the UI via Tauri `#[tauri::command]`s.

## How a Palworld server actually works (there is NO database)

The dedicated server exposes **four** surfaces the manager wraps. Do not look for a SQL DB —
world data is Unreal binary saves.

1. **Config** — `Pal/Saved/Config/WindowsServer/PalWorldSettings.ini`. Nearly every setting
   lives in one `OptionSettings=(Key=Value,Key=Value,...)` blob. Defaults come from
   `DefaultPalWorldSettings.ini` in the install root. Editing this is the core config feature.
2. **REST API** (preferred live control) — enable `RESTAPIEnabled=True`, default port `8212`,
   HTTP Basic auth with the admin password. Endpoints: `/v1/api/info`, `/players`, `/settings`,
   `/metrics`, `/announce`, `/kick`, `/ban`, `/unban`, `/save`, `/shutdown`, `/stop`.
   (Verify the exact set against the running server version.)
3. **RCON** (fallback live control) — Source RCON protocol, default port `25575`, admin password.
4. **Save/world data** — `Pal/Saved/SaveGames/0/<worldid>/*.sav` (Unreal **GVAS** binary,
   often compressed). Editing players/pals/guilds means parsing GVAS (ref: community
   `palworld-save-tools`). Advanced tier — later phase.

## Install / update the server

Free anonymous SteamCMD download — **Steam App ID `2394010`** ("Palworld Dedicated Server").
No game ownership required. The app bootstraps SteamCMD, then runs:
`steamcmd +login anonymous +app_update 2394010 validate +quit`
Same command updates. Server binary on Windows is `PalServer.exe` in the install root.

## Planned backend module layout (`src-tauri/src/`)

- `steamcmd.rs` — download/bootstrap SteamCMD, install/update server, stream progress.
- `server.rs` — start/stop/restart `PalServer.exe`, status, log tailing, auto-restart.
- `config.rs` — parse ↔ write `PalWorldSettings.ini` `OptionSettings` blob (typed model);
  JSON preset import/export + import from any `PalWorldSettings.ini`.
- `detect.rs` — auto-detect existing server installs (Steam libraries via registry +
  `libraryfolders.vdf`, and the app-managed folder); connect to one by setting install dir.
- `rest.rs` — REST API client (players, metrics, announce, kick/ban, save, shutdown).
- `rest.rs` — REST API client (info, metrics, players, announce, kick/ban/unban, save,
  shutdown) + `enable()` helper that flips `RESTAPIEnabled`/port/AdminPassword in config.
- `rcon.rs` — RCON fallback client (not built yet).
- `backups.rs` — zip/restore `SaveGames` (timestamped archives under app data dir).
- `save/` — GVAS save parsing/editing (later).

## Roadmap (build in steps toward the full thing)

- **M1 (done):** SteamCMD install/update → start/stop server → INI config editor;
  auto-detect/connect to existing installs; config preset import/export.
- **M2 (done):** Live dashboard via REST (info/metrics, players w/ kick/ban, broadcast,
  save, graceful shutdown) + one-click "Enable REST API"; SaveGames backup/restore.
- **M3:** Scheduled restarts/backups, multi-server profiles, log viewer.
- **M4:** GVAS save editing (players, pals, inventory, guilds).
- **M5:** Polish + packaged `.msi`/`.exe` release, auto-update.

## Commands

- `npm install` — install frontend deps.
- `npm run tauri dev` — run the app in dev (hot-reload UI + Rust).
- `npm run tauri build` — produce release `.msi` and `.exe` in
  `src-tauri/target/release/bundle/`.
- Cargo isn't always on PATH in a fresh shell; prefix with `export PATH="$HOME/.cargo/bin:$PATH"`.

## Real-server gotchas (verified against server v0.7.3, 2026-07)

- **SteamCMD first run** self-updates the Steam client, relaunches, and exits with
  **code 7** *without* running `app_update`. Must run a second time. `steamcmd::run_update`
  retries once and also treats "PalServer.exe now exists" as success.
- **Shipping process name varies by version**: it runs as `PalServer-Win64-Shipping-Cmd.exe`
  (not `...Shipping.exe`). Detect/stop by the `PalServer*` image prefix + "Shipping" —
  never hardcode the exact shipping exe name.
- **REST JSON contract** confirmed: `/info` {version,servername,description,worldguid},
  `/metrics` {currentplayernum,serverfps,serverframetime,maxplayernum,uptime,days,basecampnum},
  `/players` {players:[]}; POST `/announce` {message}, `/save` {}; bad auth → 401.
- REST comes up within a few seconds of launch once `RESTAPIEnabled=True` + `AdminPassword` set.

## Environment notes

- Rust (stable-msvc), MSVC Build Tools (VS18), Windows SDK, WebView2, Node 24 — all installed.
