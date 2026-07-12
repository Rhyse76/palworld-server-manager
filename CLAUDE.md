# Palworld Server Manager

A next-gen desktop GUI for installing, configuring, and running a **Palworld dedicated
server** on Windows. Goal: a polished, reliable alternative to the existing community
managers (which are largely abandoned or partly broken).

This project is **independent** of any other repo on this machine.

## Current status — multi-game is SHIPPED (2026-07)

The "multi-game support" plan in the backlog below is **done, not future**. The app is now a
game-adapter engine (`src-tauri/src/game/`, `Game` trait + `GameSpec`, `game::active()` resolves
the active game off the active profile) driving two live-validated games:

- **Palworld** — REST live control (port 8212).
- **ARK: Survival Ascended** — RCON live control (Steam app `2430930`), validated end-to-end
  against a real server. `rcon.rs` (Source RCON) is built. Config spans `GameUserSettings.ini`
  + `Game.ini` + launch args.

The UI is capability-aware (`liveControl`: rest/rcon/none, `hasMods`). Current version **0.4.4**.

**Punch list (what's left):**
1. **ARK settings catalog** (biggest — start a FRESH session). Merge a curated catalog of known
   ARK settings (types/defaults/groups) with the live file so Config shows the full set, not just
   the ~46 ARK writes. Needs: merge infra in `ark::config::read`; write-back that INSERTS new keys
   into the right file+section (generalize `upsert_server_settings`, route by composite key); and
   authoring the data. Config-page tabs already wired to receive it.
2. **Full rebrand → "RhyseGaming Server Manager"** (repo, updater endpoint, installer, Store
   display name — keep the bundle identifier stable). Decision + gotchas in the multi-game section
   below.
3. **Enshrouded adapter** (3rd game — install/update/config/automation only, no live protocol).
4. **MS Store cert for v0.4.4** — pending. If it fails again for the same reason, reply to the
   feedback / open a Partner Center ticket — don't blind-resubmit.
5. Small/manual: ARK detection in `detect.rs` (Palworld-only today); About-page trademark
   disclaimer stays Palworld-specific until the rebrand.

**Working style (user is credit-conscious):** batch edits → `npx tsc --noEmit` + `cargo check` →
commit → push. Keep responses lean. Do NOT run `tauri dev` — its recompile kills the ARK child
server. Start a fresh session per task; long sessions cost more per turn and compaction spikes it.

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
- `server.rs` — start (`PalServer.exe` + `CREATE_NEW_CONSOLE`) / stop (taskkill `PalServer*`)
  / status (tasklist `PalServer*` → "Shipping").
- `automation.rs` — 60s scheduler: scheduled backups/restarts + crash watchdog; `logs.rs`
  activity log.
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
- **M3 (done):** Automation (scheduled restarts + scheduled backups w/ pruning + **crash
  watchdog**, 60s scheduler thread), **manager activity log**, multi-server profiles (config
  migrated to profiles). Server launched via `PalServer.exe` in its own console (stable).
- **M4 (deferred):** GVAS save editing (players, pals, inventory, guilds) — read-only
  viewer first, then edits (give items/levels) with forced backups + server-stop.
- **M5:** Polish + packaged `.msi`/`.exe` release, auto-update.

## Backlog / feature ideas (prioritized)

- Quick wins: **Discord webhooks** (up/down/crash, join/leave, backups — IN PROGRESS),
  scheduled announcements/MOTD, auto-update on new server versions, smart (0-player) restarts.
- Differentiators: **connectivity/port-forward helper** (public IP + reachability + UPnP —
  the "friends can't connect" fix), off-site/cloud backups (paid-tier), metrics-history graphs,
  mod manager (local `.pak` + UE4SS/Lua), first-run setup wizard.
- Email alerts (low priority — Discord already covers most users): add an Email card mirroring
  the Discord one, subscribing to the same automation events (up/down/crash/backup). Ship TWO
  easy routes in one card — (1) user-provided SMTP + app password via `lettre`, and (2) bring-
  your-own transactional API key (Resend/Postmark/SES) via one HTTPS POST. Store the password/
  key in Windows Credential Manager (`keyring` crate), NOT plaintext config. Deliverability
  requires egress through a real provider (never raw SMTP from a home IP → spam/blocked).
  DEFER OAuth/XOAUTH2 SMTP: code is a few days, but Gmail's restricted mail scope needs an
  annual Google CASA security assessment + Azure app registration — only worth it as a funded
  paid-tier feature.
- Big bets: remote/web access (headless service + web UI).
- **Multi-game support (ONE app, adapter architecture — NOT separate apps per game).** User wants
  a single manager for Palworld + ARK: Survival Ascended + Enshrouded (and possibly Valheim). The
  clean structure: a `Game` trait each game implements (`app_id`, `server_exe`, save paths, default
  ports, config model/schema, optional live-control client, mods), driven by the existing shared
  engine (SteamCMD, process, backups, automation/watchdog, Discord, UPnP, metrics, updater, UI
  shell = ~60-70% reused). Games stay isolated behind the trait so they don't step on each other.
  - Effort: one-time "extract the engine" refactor (~1-2 wks), biggest piece = making the Config
    page **schema-driven** instead of hardcoded to Palworld's OptionSettings blob. Then per game:
    ARK ~3-5 days, Enshrouded ~2-3 days.
  - Per-game live-control reality (adapter advertises capabilities; UI hides unsupported):
    Palworld = REST + RCON (richest); ARK:SA = **RCON only** (finally needs the backlogged
    `rcon.rs`); Enshrouded = **no live protocol** (install/update/start/stop/backup/config/
    automation only — no players/kick/announce). Config formats differ: Palworld INI OptionSettings
    blob, ARK `GameUserSettings.ini` + `Game.ini` + launch args, Enshrouded JSON.
  - Non-code cost: **rebrand** off "Palworld" — repo name, rhysegaming.com/palworld page, MS Store
    listing, and the updater endpoint URL are all Palworld-specific today.
  - Plan: keep shipping Palworld as-is; when ready, do the engine extraction ONCE with Palworld as
    adapter #1, then add ARK, then Enshrouded, under a game-neutral name.
  - **DECIDED (2026-07): the game-neutral name is "RhyseGaming Server Manager"** (on-brand for
    rhysegaming.com, game-neutral so it survives the shift). Rename is DEFERRED to the multi-game
    shift to avoid a name/marketing mismatch while it's Palworld-only. Already done now: app
    recolored to the RhyseGaming palette (navy/slate + blue `#3b82f6`, green for online). Also
    doable independently now: MS Store **publisher identity = RhyseGaming (individual, not a
    company)** — publisher can be RhyseGaming while the app stays the Palworld listing for now.
  - Rename gotcha when it happens: KEEP the Tauri `bundle.identifier` stable (changing it makes the
    installer a NEW app, not an upgrade). Repo rename is fine (GitHub 301-redirects the old updater
    endpoint). Rename touches: in-app title/About, repo, updater endpoint, installer name, Store
    listing, rhysegaming.com/palworld page. Also delete dead `src/App.css` (Vite template leftover,
    not imported by `main.tsx`).

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
- **No usable server log file**: Palworld writes no `Pal/Saved/Logs/Pal.log`; only Steam/EOS
  SDK logs exist under `Pal/Binaries/Win64/logs/`. The launcher `PalServer.exe` produces no
  stdout, and the console build `...-Cmd.exe` emits only ~2 startup lines then nothing.
- **Console build needs a real console**: launching `...-Cmd.exe` with stdout redirected to a
  file (to capture logs) makes it crash with `LowLevelFatalError ... SECURE CRT: Invalid
  parameter detected` (an Assert, seen live ~3 min in). So `server::start` instead launches
  `PalServer.exe` with **`CREATE_NEW_CONSOLE`** (its own console) — the stable, standard method
  (same as double-clicking). We do NOT capture game stdout.
- **Logs → activity log**: because there's no good game log, `logs.rs` keeps a *manager*
  activity log (`<appdata>/logs/activity.log`) of app actions (start/stop/install/REST/
  automation/crash), persisted + streamed via `activity-log` events.
- **Crash watchdog**: `SECURE CRT` crashes are a known, common Palworld server issue regardless
  of launcher. `automation.rs` supervises servers the app started (`supervise` flag) and
  auto-restarts them if they die unexpectedly (toggle: `autoRestartOnCrash`, default on).

## Release process (IMPORTANT — keeps self-update working)

The app self-updates via `tauri-plugin-updater`, reading
`https://github.com/Rhyse76/palworld-server-manager/releases/latest/download/latest.json`.
Every release MUST be **signed** and ship a matching `latest.json`, or self-update breaks.

1. Bump version everywhere: `tauri.conf.json`, `src/App.tsx` footer, `SettingsPage` About,
   `msix/*` (`0.x.0.0`), and the site (`softwareVersion` + `vX` label).
2. **Signed build** (private key lives OUTSIDE the repo at `~/.tauri-keys/palworld-updater.key`,
   empty password; pubkey is in `tauri.conf.json > plugins.updater`):
   ```
   TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri-keys/palworld-updater.key)" \
   TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" npm run tauri build
   ```
   Produces `…-setup.exe` and `…-setup.exe.sig` under `target/release/bundle/nsis/`.
3. Create the GitHub release `vX.Y.Z`, upload the installer as `PalworldServerManager-Setup.exe`,
   and upload a `latest.json` asset:
   `{"version":"X.Y.Z","notes":"…","pub_date":"<ISO>","platforms":{"windows-x86_64":
   {"signature":"<contents of the .sig>","url":"https://github.com/Rhyse76/palworld-server-manager/releases/download/vX.Y.Z/PalworldServerManager-Setup.exe"}}}`
   (Release body via `curl` must have NO unescaped double-quotes.)
4. Users on ≥ v0.4.0 then get the update in-app.

## Environment notes

- Rust (stable-msvc), MSVC Build Tools (VS18), Windows SDK, WebView2, Node 24 — all installed.
