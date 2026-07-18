# RhyseGaming Server Manager

A next-gen desktop GUI for installing, configuring, and running **Palworld, ARK: Survival
Ascended, and Enshrouded dedicated servers** on Windows. Goal: a polished, reliable
alternative to the existing community managers (which are largely abandoned or partly
broken).

This project is **independent** of any other repo on this machine.

> Current status, the active punch list, and working notes live in `CLAUDE.local.md`
> (gitignored, not published). It also auto-loads for Claude.

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
  - **DECIDED (2026-07): the game-neutral name is "RhyseGaming Server Manager"**. **Cosmetic
    rename DONE (2026-07)**: `tauri.conf.json` (`productName`/`mainBinaryName`/window title,
    `identifier` deliberately left as `com.palmanager.desktop`), `index.html`, About page,
    UPnP description string, `msix/` build script + manifest, READMEs, this file's release-process
    installer filename. **Still deferred** (live/external, needs explicit sequencing — not done
    casually): GitHub repo rename + updater endpoint URL (must ship together as one coordinated,
    version-bumped release so self-update keeps working for existing installs), MS Store listing
    name (external Partner Center action; also don't touch while the current cert review is
    in flight), rhysegaming.com/palworld page (outside this repo).
  - Rename gotcha when the repo/updater part happens: KEEP the Tauri `bundle.identifier` stable
    (changing it makes the installer a NEW app, not an upgrade). Repo rename is fine (GitHub
    301-redirects the old updater endpoint) but still update the URL in `tauri.conf.json` for new
    builds rather than relying on the redirect indefinitely.

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
   `msix/*` (`0.x.0.0`), **and the website** — separate repo/directory
   (`C:\Users\Rhyse\rhysegaming-site`, not this one), `servermanager/index.html`'s
   `softwareVersion` JSON-LD field and the `vX.Y.Z` text near the download button. Easy to
   forget since it's outside this repo and easy to open — has been missed more than once.
2. **Signed build** (updater key lives OUTSIDE the repo — path is in `CLAUDE.local.md`'s
   secrets section; empty password; pubkey is in `tauri.conf.json > plugins.updater`).
   **Must run in Git Bash, not PowerShell** — PowerShell's `$env:VAR = ""` deletes the
   variable instead of setting it empty, which makes the updater-key step fall back to an
   interactive password prompt and hang forever in a non-interactive shell:
   ```
   TAURI_SIGNING_PRIVATE_KEY="$(cat <path-from-CLAUDE.local.md>)" \
   TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
   CURSEFORGE_API_KEY="$(cat <curseforge-key-path-from-CLAUDE.local.md>)" \
   AZURE_CLIENT_ID="$(cat <path-from-CLAUDE.local.md>)" \
   AZURE_CLIENT_SECRET="$(cat <path-from-CLAUDE.local.md>)" \
   AZURE_TENANT_ID="$(cat <path-from-CLAUDE.local.md>)" \
   npm run tauri build
   ```
   Produces `…-setup.exe` and `…-setup.exe.sig` under `target/release/bundle/nsis/`.
   `CURSEFORGE_API_KEY` is baked into the binary at compile time (`option_env!` in
   `curseforge.rs`) so the Mods page's CurseForge search works out of the box for every
   user without them needing their own key (Settings still lets a user override with
   their own key, which always takes priority). Omitting this var still produces a
   working build — search just falls back to "no key configured" until a user sets
   their own.

   **Authenticode signing (2026-07, via Azure Artifact Signing)**: `tauri.conf.json`'s
   `bundle.windows.signCommand` runs `artifact-signing-cli` (installed via
   `cargo install artifact-signing-cli`) against the `Server-Manager` Artifact Signing
   Account + `PublicTrust` Certificate Profile, using the `AZURE_CLIENT_ID`/
   `AZURE_CLIENT_SECRET`/`AZURE_TENANT_ID` env vars above (a dedicated App Registration,
   `RhyseGamingServerManager-CodeSigning`, scoped to just the `Artifact Signing
   Certificate Profile Signer` role on that one account — not the user's own login).
   This signs the exe, DLLs, MSI, and NSIS installer automatically as part of
   `npm run tauri build` — verify with `signtool verify /pa /v <path>`. Separate from
   (and in addition to) the updater `.sig` above — one is Authenticode (stops
   SmartScreen's unrecognized-publisher warning), the other is Tauri's own
   updater-integrity signature.
3. Create the GitHub release `vX.Y.Z`, upload the installer as `RhyseGamingServerManager-Setup.exe`,
   and upload a `latest.json` asset:
   `{"version":"X.Y.Z","notes":"…","pub_date":"<ISO>","platforms":{"windows-x86_64":
   {"signature":"<contents of the .sig>","url":"https://github.com/Rhyse76/palworld-server-manager/releases/download/vX.Y.Z/RhyseGamingServerManager-Setup.exe"}}}`
   (Release body via `curl` must have NO unescaped double-quotes.)
4. Users on ≥ v0.4.0 then get the update in-app.

## Environment notes

- Rust (stable-msvc), MSVC Build Tools (VS18), Windows SDK, WebView2, Node 24 — all installed.
