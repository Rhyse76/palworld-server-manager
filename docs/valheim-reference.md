# Valheim — dedicated server reference

Verified facts from a real installed server (2026-09-14):
`C:\Program Files (x86)\Steam\steamapps\common\Valheim dedicated server` on Rhyse's
machine, plus a live test world running from it (`-world "Dedicated"`). No real
secrets reproduced here — the sample values below are the game's own shipped
placeholders (`"My server"` / `"secret"`), not the real server's actual name/password.

## GameSpec values

| Field | Value | Confidence |
|---|---|---|
| `steam_app_id` | `896660` | confirmed — matches `steamapps\appmanifest_896660.acf` (`"name" "Valheim Dedicated Server"`) |
| `server_launcher` / `process_match` / `process_marker` | `valheim_server.exe` / `IMAGENAME eq valheim_server.exe` / `valheim_server` | confirmed real install layout; **not yet confirmed via a live start/stop through the app** (unlike Enshrouded/ARK, which were) |
| `config_rel` | `start_headless_server.bat` (install root) | confirmed filename |
| `saves_rel` | `saves` (install-relative) | **enforced by this app, not the game's default** — see below |
| `default_game_port` | `2456` | confirmed (real file's `-port` value; game+query+Steam ports are `2456`-`2458`, per the bat's own comment) |
| `live_control` | `None` | vanilla server ships no RCON/REST admin API |
| `mods` | `ModsKind::None` | vanilla only; BepInEx (`denikson-BepInExPack_Valheim`) is the real modding path — TODO, not scaffolded |
| `default_config` | `None` | the shipped bat is itself the live "config"; no separate defaults file |

## Config format: one CLI line, not a settings file

There is no ini/json settings file. The shipped `start_headless_server.bat`:

```bat
@echo off
set SteamAppId=892970

echo "Starting server PRESS CTRL-C to exit"

REM Tip: Make a local copy of this script to avoid it being overwritten by steam.
REM NOTE: Minimum password length is 5 characters & Password cant be in the server name.
REM NOTE: You need to make sure the ports 2456-2458 is being forwarded to your server through your local router & firewall.
valheim_server -nographics -batchmode -name "My server" -port 2456 -world "Dedicated" -password "secret" -crossplay
```

- `SteamAppId=892970` is a required Steamworks env var (892970 = Valheim's base-game
  app id, distinct from the dedicated server's own download app id 896660) — **not**
  user-editable, preserved untouched. A `steam_appid.txt` containing `892970` also
  ships in the install root, so this may be redundant with the env var; unconfirmed
  which one the game actually reads.
- Everything else — name, port, world, password, `-crossplay` — is an inline flag on
  that one line. `config.rs` tokenizes it (quote-aware), maps known flags to
  `ConfigField`s, and **rewrites the whole line** on save.
- `-public 0|1` is a documented Valheim flag not present in this real file (the
  server here relies on `-crossplay` instead) — modeled as an optional field anyway
  since both exist in the wild.

**Known v1 limitation:** `write` rebuilds the entire launch line from the fields it
knows plus a fixed `-nographics -batchmode` prefix; any hand-added flag it doesn't
recognize (e.g. a modloader arg) is preserved in the token bucket but **appended at
the end**, not kept in its original position. Fine for the basics; revisit if that
turns out to matter.

## Saves directory — the one real architectural gap

Confirmed: with no `-savedir` flag, the game does **not** save under the install
directory. The real test server's world data lives at:

```
%userprofile%\AppData\LocalLow\IronGate\Valheim\
  worlds_local\<world>\        <- the world files (.db2/.fwl2/.chunk)
  worlds_local\<world>_backup_auto-<timestamp>\   <- Valheim's OWN auto-backups
  adminlist.txt / bannedlist.txt / permittedlist.txt   <- global, not per-world
```

This is per-**Windows-user**, not per-install — a problem for a multi-profile
manager. Every other adapter (Palworld/ARK/Enshrouded) saves under its own install
dir, which is what this app's backup feature assumes.

**Decision made here (not a discovery):** `config::write`/`launch_args` force a
`-savedir "<install_dir>\saves"` flag onto the line regardless of what the on-disk
bat had, so `saves_rel = "saves"` holds true for any server this app manages.

**Open item for tonight:** the real existing server predates this and has no
`-savedir` — its actual world data is sitting in the LocalLow path above, not
`<install_dir>\saves`. Decide whether to:
1. Migrate that world folder into `<install_dir>\saves` once, then let the app take
   over, or
2. Add a "detected an existing non-managed save location" import/adopt flow, or
3. Just start a fresh world under app management and treat the old one as the
   manual test that got us here.

## Full flag list (from the official manual)

Source: `Valheim Dedicated Server Manual.pdf`, shipped in the install root — this is
Iron Gate's own doc, so higher confidence than the single real bat file alone (which
only exercises a subset). Confirms `-name`/`-port`/`-world`/`-password`/`-savedir`/
`-public`/`-crossplay` exactly as scaffolded, plus these **not yet modeled**:

| Flag | Purpose | Default |
|---|---|---|
| `-logFile "path"` | Where to write the log file | none |
| `-saveinterval 1800` | Seconds between world autosaves | 1800 (30 min) |
| `-backups 4` | How many automatic backups to keep | 4 |
| `-backupshort 7200` | Seconds between the first automatic backups | 7200 (2 hr) |
| `-backuplong 43200` | Seconds between subsequent automatic backups | 43200 (12 hr) |
| `-instanceid "1"` | Disambiguates multiple servers on the same port/MAC for PlayFab | none |
| `-preset hard` | World modifier preset — `Normal, Casual, Easy, Hard, Hardcore, Immersive, Hammer` | none |
| `-modifier raids none` | Individual world modifier — `Combat` (veryeasy/easy/hard/veryhard), `DeathPenalty` (casual/veryeasy/easy/hard/hardcore), `Resources` (muchless/less/more/muchmore/most), `Raids` (none/muchless/less/more/muchmore), `Portals` (casual/hard/veryhard) | none |
| `-setkey nomap` | World modifier checkbox — `nobuildcost, playerevents, passivemobs, nomap` | none |

Also confirmed by the manual (not flags, just useful facts): the default save path
really is `%USERPROFILE%\AppData\LocalLow\IronGate\Valheim` absent `-savedir`
(matches what was found on disk); `adminlist.txt`/`bannedlist.txt`/
`permittedlist.txt` live in that same save path and take one `[Platform]_[UserID]`
per line (from the server log or in-game F2 panel); putting anyone on
`permittedlist.txt` switches the server to allowlist-only.

**TODO tonight:** decide which of the above to expose as `ConfigField`s (presets/
modifiers seem like the highest-value additions for the Config page) and extend
`config.rs`'s `VALUE_FLAGS`/tokenizer accordingly — `-modifier` is trickier since it
can repeat (one `-modifier <name> <value>` pair per modifier), unlike every other
flag here which appears at most once.

## Rhyse's live server: running a pre-1.0 downgrade for Switch crossplay — research pending

Rhyse's *actual* hosted server (not the one on this dev machine) is intentionally
downgraded to Valheim **1.0.7**, not the current 1.0.12/"1.0.x" the game shipped
with — because of a known bug (Iron Gate is actively working on it as of
2026-09-14) that breaks crossplay with Switch on newer 1.0.x builds. Getting a
SteamCMD server onto an older build than "latest" requires:

- A `-beta <branchname>` (and possibly `-betapassword`) argument to `steamcmd`'s
  `app_update` — the specific branch name Valheim uses for this needs research
  (Rhyse mentioned something like a `default_old`-ish branch, unconfirmed spelling).
- Rhyse recalled needing to **log into SteamCMD with real Steam credentials**
  instead of `+login anonymous` before the beta branch would install — also needs
  confirming (anonymous login normally works fine for the vanilla dedicated server
  depot; some beta/legacy branches do require an authenticated account that owns the
  app).
- A "start SteamID" / "force install at correct location" step Rhyse mentioned
  before login — unclear exactly what this refers to yet (possibly
  `+force_install_dir <path>` needing to come before `+login`, which is a real
  SteamCMD ordering requirement).

**Not researched yet — Rhyse asked to hold off until tonight.** When picking this
back up: confirm the exact Valheim beta branch name/password (if any) for
pre-crossplay-fix builds, the correct `steamcmd` command ordering
(`force_install_dir` → `login` → `app_update ... -beta <branch>`), and whether the
game's own beta-branch selection through Steam's GUI ("Betas" tab in game
properties) is simpler to script against than raw `steamcmd` flags. This would
become a per-profile "pinned version/branch" option in the adapter/installer, not a
`GameSpec` constant, since it's a workaround for a temporary bug rather than a
permanent per-game fact.

## Not yet done

- No live start/stop verification through the app itself (Enshrouded/ARK both got
  this pass; Valheim hasn't).
- `admin/ban/permit` list management (`adminlist.txt` etc.) — real files exist and
  are empty on the test server; ARK's `bans.rs` access-list pattern is the closest
  precedent if this gets built out.
- BepInEx mod support.
- Frontend copy/labels for Valheim in `ConfigPage.tsx`/`ServerPage.tsx`/etc. — not
  touched yet, backend-only scaffold so far.
