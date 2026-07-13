# Enshrouded — dedicated server reference

Verified facts from a real installed server (2026-07). No secrets here — the sample
config's passwords were temporary placeholders the user was replacing anyway, and
aren't reproduced (only field names/structure).

## GameSpec values

| Field | Value | Confidence |
|---|---|---|
| `steam_app_id` | `2278520` | confirmed by user |
| `server_launcher` | `enshrouded_server.exe` (install root, no subfolder) | confirmed (real install layout) |
| `process_match` / `process_marker` | `IMAGENAME eq enshrouded_server.exe` / `enshrouded_server` | **unverified** — not yet live-started through the app. Palworld and ARK both shipped a *running* process name that differed from the launcher exe (`PalServer-Win64-Shipping-Cmd.exe`, `ArkAscendedServer.exe` behind a different launcher), so this may need correcting after a start/stop test. |
| `config_rel` | `enshrouded_server.json` (install root) | confirmed |
| `saves_rel` | `savegame` (install root; matches the JSON's own `saveDirectory: "./savegame"`) | confirmed |
| `default_game_port` | `15637` (the JSON's `queryPort`; no separate `port` field in the schema) | confirmed value exists, semantics (game port vs. query port relationship) unverified |
| `live_control` | `None` — no REST/RCON/any admin protocol | per original plan, not contradicted by anything found |
| `mods` | `ModsKind::None` — no documented mod system for the dedicated server | per original plan |
| `default_config` | `None` — no separate defaults file, but unlike ARK this doesn't matter: see below | confirmed |

## Config format: a single complete JSON file

Confirmed real install layout:

```
<install_dir>/
  enshrouded_server.exe
  enshrouded_server.json      <- config_rel
  appcache/
  config/
  logs/
  savegame/                   <- saves_rel
```

Unlike ARK, **the server writes a complete config on first run** — every field below
was already present with sane defaults, no "missing keys" problem. That means the
Enshrouded adapter needs no defaults catalog (contrast ARK's `catalog.rs`): `config::read`
just parses whatever's in the live file.

### Top-level fields (group "Server")

`name` (string), `ip` (string), `queryPort` (int), `slotCount` (int), `voiceChatMode`
(enum, seen: `"Proximity"`), `enableVoiceChat` / `enableTextChat` (bool),
`gameSettingsPreset` (enum, seen: `"Default"`), `tags` (string array — exposed as one
comma-joined field, empty in the sample). `saveDirectory`/`logDirectory` exist but
aren't exposed as editable (they're install-topology, not server behavior).

### `gameSettings` (group "Game Settings", ~36 fields)

All multiplier/factor fields default to `1` (float). Notable non-multiplier fields:
`enableDurability`, `enableStarvingDebuff`, `enableGliderTurbulences`,
`pacifyAllEnemies` (bool); `tombstoneMode`, `weatherFrequency`, `fishingDifficulty`,
`randomSpawnerAmount`, `aggroPoolAmount`, `tamingStartleRepercussion`, `curseModifier`
(enum strings, exact allowed value sets not enumerated — treated as free-text enum,
same permissive approach as ARK's unknown-enum fields). Three fields are **nanosecond
durations**, not counts — labeled explicitly in the UI so editing them isn't a
guessing game: `fromHungerToStarving` (600000000000 ns = 10 min in the sample),
`dayTimeDuration` (1800000000000 ns = 30 min), `nightTimeDuration` (720000000000 ns =
12 min).

### `userGroups` (group "Access & Permissions")

A fixed array of exactly four role objects — `Admin`, `Friend`, `Guest`, `Visitor`
(matched by name, not position, so field order in the file doesn't matter). Each has:
`password` (string — this is genuinely useful to expose/edit, since these are the
join passwords players use per role), `canKickBan`, `canAccessInventories`,
`canEditWorld`, `canEditBase`, `canExtendBase` (bool), `reservedSlots` (int).

### `bannedAccounts`

Empty in the sample, shape unconfirmed. **Not exposed as a field** — `config::write`
re-reads the live JSON fresh and patches only the known paths above back into it, so
`bannedAccounts` (and anything else we don't model) survives untouched. Same
preservation principle as ARK's in-place line edits, just applied to a JSON tree
instead of INI lines.

## Adapter build status

- ✅ **Scaffold + config parser built** (`src-tauri/src/game/enshrouded/`): `GameSpec`
  with the values above; `config.rs` does a full JSON round-trip (`read`/`write`/
  `import`) via `serde_json::Value`, patching recognized paths without disturbing
  unmodeled ones. Registered via `game::by_id("enshrouded")`; unit-tested against a
  redacted fixture (placeholder passwords, never the real ones).
- ✅ **Zero frontend changes needed** — `ConfigPage.tsx` is fully schema-driven off
  `group`, the "Add profile" game picker (`ProfilesCard.tsx`) is driven generically by
  `gamesList()`, and `DashboardPage`/Mods-page/Save-tools nav gating already handle
  `liveControl: "none"` / `modsKind: "none"` (built in anticipation of this adapter).
- ⏭️ **Not yet done:**
  1. **Live shakedown** — install via the app's SteamCMD flow, start/stop, confirm the
     real running process name (`process_match`/`process_marker` above are a best
     guess), and confirm `config::read`/`write` round-trips against the app's own
     install rather than just the one real file we've seen.
  2. **`detect.rs`** auto-detection is Palworld-only today (ARK has the same gap) —
     Enshrouded installs must be connected by manually setting the install folder for
     now.
  3. Allowed-value sets for the enum-ish string fields (`tombstoneMode`,
     `weatherFrequency`, etc.) aren't enumerated — they render as free-text for now
     rather than a dropdown with known options.
