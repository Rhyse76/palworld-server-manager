# Palworld Server Manager

A clean, modern, all-in-one desktop app for running your own **Palworld dedicated server**
on Windows — install, configure, monitor, back up, and automate it, all from one window.

**➡️ Download & screenshots: [rhysegaming.com/palworld](https://rhysegaming.com/palworld)**

> Unofficial, community-made tool. Not affiliated with or endorsed by Pocketpair, Inc.
> "Palworld" is a trademark of its respective owner.

## Features

- **One-click install & update** — bootstraps SteamCMD and installs/updates the dedicated
  server (Steam app `2394010`) with live progress. You don't need to own Palworld.
- **Full config editor** — every setting from `PalWorldSettings.ini` as friendly toggles and
  inputs, with search. Import/export presets.
- **Live dashboard** — online players, server FPS/uptime, broadcast, kick/ban, save, graceful
  shutdown, via the server's REST API (one-click enable).
- **Backups & restore** — one-click zipped snapshots of the world save; restore anytime.
- **Automation** — scheduled restarts, scheduled backups (with pruning), and a **crash
  watchdog** that auto-restarts the server if it dies unexpectedly.
- **Multi-server profiles** — manage several servers and switch between them instantly.

## Download

Grab the latest Windows installer from the
[**Releases**](https://github.com/Rhyse76/palworld-server-manager/releases/latest) page, or
from [rhysegaming.com/palworld](https://rhysegaming.com/palworld).

The installer isn't code-signed yet, so Windows SmartScreen may warn you — click
**More info → Run anyway**.

## Tech

[Tauri 2](https://tauri.app) (Rust backend) + React/TypeScript frontend. Tiny native binary
(~13 MB), builds `.msi` and `.exe` installers.

### Build from source

Prerequisites: [Rust](https://rustup.rs) (MSVC toolchain), [Node.js](https://nodejs.org),
and the Windows C++ build tools + WebView2 (present on Windows 11).

```bash
npm install
npm run tauri dev     # run in development
npm run tauri build   # produce release installers in src-tauri/target/release/bundle/
```

## Support

If this saves you time running your server, you can support development — see the button on
[rhysegaming.com/palworld](https://rhysegaming.com/palworld). Thank you! ♥
