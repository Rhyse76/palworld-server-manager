//! Valheim config format — verified against a real install (2026-09-14):
//! `C:\Program Files (x86)\Steam\steamapps\common\Valheim dedicated server`.
//!
//! Unlike every other adapter, Valheim's dedicated server has NO structured settings
//! file (no ini/json). The official `start_headless_server.bat` it ships with sets
//! one infra env var (`SteamAppId=892970`, required for Steamworks — untouched here,
//! not user-editable) and then launches the server with everything else as **inline
//! CLI flags on a single line**:
//!
//! ```bat
//! valheim_server -nographics -batchmode -name "My server" -port 2456 -world "Dedicated" -password "secret" -crossplay
//! ```
//!
//! So this app's "config" for Valheim is that one line. `read`/`import` tokenize it;
//! `write` **rebuilds the whole line** from the known fields (see the module-level
//! limitation note below — this is the biggest gap to close tonight).
//!
//! **Saves are NOT install-relative by default** — confirmed: with no `-savedir`
//! flag, the game writes world files to
//! `%userprofile%\AppData\LocalLow\IronGate\Valheim\worlds_local\<world>\`, outside
//! the install dir entirely (and `adminlist.txt`/`bannedlist.txt`/`permittedlist.txt`
//! live in that same LocalLow folder, not per-world). That breaks this app's
//! install-relative `saves_rel`/backup assumption, so `write`/`launch_args` **force**
//! a `-savedir` flag pointing at `<install_dir>\saves`, making `saves_rel = "saves"`
//! true by construction rather than by the game's own default. This is a deliberate
//! design choice, not a discovered fact — flag it for review tonight, since it means
//! an existing manually-run server's world data (like the real one found on this
//! machine, saved under `-world "Dedicated"` with no `-savedir`) won't be seen by the
//! app until that data is migrated into `<install_dir>\saves`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::ConfigField;

fn script_path(install_dir: &Path) -> PathBuf {
    install_dir.join(super::SPEC.config_rel)
}

/// Install-relative save directory this app enforces via a forced `-savedir` flag
/// (see module docs) — kept as one constant so `config.rs`/`mod.rs` agree.
const SAVES_DIR_NAME: &str = "saves";

struct Launch {
    /// Everything on the line before the flags (e.g. `valheim_server` or a custom
    /// `start "" valheim_server.exe` prefix) — preserved verbatim.
    prefix: String,
    name: Option<String>,
    port: Option<String>,
    world: Option<String>,
    password: Option<String>,
    public: Option<String>,
    crossplay: bool,
    /// Bare/valueless flags we don't model (e.g. `-nographics`, `-batchmode`) or
    /// unknown flags with their values — preserved verbatim, in original order.
    /// TODO tonight: this is a coarse "everything we don't recognize" bucket; a
    /// hand-added flag we DO recognize (name/port/world/password/public) but with
    /// unusual placement still round-trips fine, but anything truly exotic (custom
    /// mod-loader flags, etc.) surviving `write` depends on ending up in here.
    other_tokens: Vec<String>,
}

/// Split a command line into tokens, respecting `"double quoted"` segments.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for c in line.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

/// The real shipped file always quotes `-name`/`-world`/`-password` even when the
/// value has no spaces (`-world "Dedicated"`, `-password "secret"`) — matched here
/// unconditionally rather than only-when-needed, so a round-trip stays byte-for-byte
/// consistent with the game's own convention.
fn quoted(s: &str) -> String {
    format!("\"{s}\"")
}

/// Flags that take a following value (as opposed to bare boolean flags).
const VALUE_FLAGS: &[&str] = &["-name", "-port", "-world", "-password", "-public", "-savedir"];

fn parse_launch(line: &str) -> Launch {
    let tokens = tokenize(line);
    let mut launch = Launch {
        prefix: String::new(),
        name: None,
        port: None,
        world: None,
        password: None,
        public: None,
        crossplay: false,
        other_tokens: Vec::new(),
    };

    let mut i = 0;
    // Everything up to (and including) the executable token, i.e. up to the first
    // token that looks like a flag, is the prefix.
    let mut prefix_tokens = Vec::new();
    while i < tokens.len() && !tokens[i].starts_with('-') {
        prefix_tokens.push(tokens[i].clone());
        i += 1;
    }
    launch.prefix = prefix_tokens.join(" ");

    while i < tokens.len() {
        let t = &tokens[i];
        let lower = t.to_lowercase();
        if VALUE_FLAGS.contains(&lower.as_str()) {
            let value = tokens.get(i + 1).cloned().unwrap_or_default();
            match lower.as_str() {
                "-name" => launch.name = Some(value),
                "-port" => launch.port = Some(value),
                "-world" => launch.world = Some(value),
                "-password" => launch.password = Some(value),
                "-public" => launch.public = Some(value),
                "-savedir" => {} // app-managed; original value discarded
                _ => unreachable!(),
            }
            i += 2;
        } else if lower == "-crossplay" {
            launch.crossplay = true;
            i += 1;
        } else {
            launch.other_tokens.push(t.clone());
            i += 1;
        }
    }
    launch
}

fn find_launch_line(lines: &[String]) -> Option<usize> {
    lines.iter().position(|l| l.to_lowercase().contains("valheim_server"))
}

fn to_fields(launch: &Launch) -> Vec<ConfigField> {
    let f = |key: &str, value: String, kind: &str, label: &str| ConfigField {
        key: key.to_string(),
        value,
        kind: kind.to_string(),
        label: label.to_string(),
        group: "Server".to_string(),
        options: Vec::new(),
    };
    vec![
        f("name", launch.name.clone().unwrap_or_default(), "string", "Server name"),
        f("port", launch.port.clone().unwrap_or_else(|| "2456".to_string()), "int", "Port"),
        f("world", launch.world.clone().unwrap_or_default(), "string", "World name"),
        f("password", launch.password.clone().unwrap_or_default(), "string", "Password"),
        f("crossplay", launch.crossplay.to_string(), "bool", "Crossplay (Xbox/PS/PC)"),
        f("public", (launch.public.as_deref() == Some("1")).to_string(), "bool", "Show in public server list"),
    ]
}

fn apply_fields(launch: &mut Launch, fields: &[ConfigField]) {
    let get = |k: &str| fields.iter().find(|f| f.key == k).map(|f| f.value.clone());
    if let Some(v) = get("name") {
        launch.name = Some(v);
    }
    if let Some(v) = get("port") {
        launch.port = Some(v);
    }
    if let Some(v) = get("world") {
        launch.world = Some(v);
    }
    if let Some(v) = get("password") {
        launch.password = Some(v);
    }
    if let Some(v) = get("crossplay") {
        launch.crossplay = v == "true";
    }
    launch.public = get("public").map(|v| if v == "true" { "1".to_string() } else { "0".to_string() });
}

/// Render a [`Launch`] back into a command line, forcing `-savedir` to this app's
/// managed install-relative directory (see module docs).
fn render(launch: &Launch, install_dir: &Path) -> String {
    let saves_dir = install_dir.join(SAVES_DIR_NAME);
    let mut parts = vec![launch.prefix.clone()];
    parts.push("-nographics".to_string());
    parts.push("-batchmode".to_string());
    parts.push(format!("-name {}", quoted(launch.name.as_deref().unwrap_or("My server"))));
    parts.push(format!("-port {}", launch.port.as_deref().unwrap_or("2456")));
    parts.push(format!("-world {}", quoted(launch.world.as_deref().unwrap_or("Dedicated"))));
    parts.push(format!("-password {}", quoted(launch.password.as_deref().unwrap_or(""))));
    if launch.crossplay {
        parts.push("-crossplay".to_string());
    }
    if let Some(public) = &launch.public {
        parts.push(format!("-public {public}"));
    }
    parts.push(format!("-savedir {}", quoted(&saves_dir.display().to_string())));
    for t in &launch.other_tokens {
        parts.push(t.clone());
    }
    parts.join(" ")
}

pub fn read(install_dir: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(script_path(install_dir))
        .map_err(|_| "No start_headless_server.bat found yet. Install the server first.".to_string())?;
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let idx = find_launch_line(&lines)
        .ok_or_else(|| "start_headless_server.bat doesn't contain a recognizable valheim_server launch line.".to_string())?;
    Ok(to_fields(&parse_launch(&lines[idx])))
}

pub fn write(install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
    let path = script_path(install_dir);
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let idx = find_launch_line(&lines)
        .ok_or_else(|| "start_headless_server.bat doesn't contain a recognizable valheim_server launch line.".to_string())?;

    let mut launch = parse_launch(&lines[idx]);
    apply_fields(&mut launch, fields);
    lines[idx] = render(&launch, install_dir);

    fs::write(&path, lines.join("\r\n")).map_err(|e| e.to_string())
}

pub fn import(path: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let idx = find_launch_line(&lines).ok_or_else(|| "No valheim_server launch line found in that file.".to_string())?;
    Ok(to_fields(&parse_launch(&lines[idx])))
}

/// Build the actual `valheim_server.exe` command-line arguments from the batch
/// file's current values (forcing the app-managed `-savedir`, per module docs).
pub fn launch_args(install_dir: &Path) -> Vec<String> {
    let text = match fs::read_to_string(script_path(install_dir)) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let Some(idx) = find_launch_line(&lines) else { return Vec::new() };
    let launch = parse_launch(&lines[idx]);
    let saves_dir = install_dir.join(SAVES_DIR_NAME);

    let mut args = vec![
        "-nographics".to_string(),
        "-batchmode".to_string(),
        "-name".to_string(),
        launch.name.filter(|s| !s.is_empty()).unwrap_or_else(|| "My server".to_string()),
        "-port".to_string(),
        launch.port.filter(|s| !s.is_empty()).unwrap_or_else(|| "2456".to_string()),
        "-world".to_string(),
        launch.world.filter(|s| !s.is_empty()).unwrap_or_else(|| "Dedicated".to_string()),
        "-password".to_string(),
        launch.password.unwrap_or_default(),
    ];
    if launch.crossplay {
        args.push("-crossplay".to_string());
    }
    if let Some(public) = launch.public {
        args.push("-public".to_string());
        args.push(public);
    }
    args.push("-savedir".to_string());
    args.push(saves_dir.display().to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    // Same shape as the real shipped file (verified 2026-09-14), with the stock
    // placeholder name/password the game itself ships — never a real user's.
    const SAMPLE: &str = "\
@echo off
set SteamAppId=892970

echo \"Starting server PRESS CTRL-C to exit\"

REM Tip: Make a local copy of this script to avoid it being overwritten by steam.
valheim_server -nographics -batchmode -name \"My server\" -port 2456 -world \"Dedicated\" -password \"secret\" -crossplay
";

    #[test]
    fn parses_the_real_shipped_format() {
        let fields = to_fields(&parse_launch(SAMPLE.lines().find(|l| l.contains("valheim_server")).unwrap()));
        let get = |k: &str| fields.iter().find(|f| f.key == k).unwrap();

        assert_eq!(get("name").value, "My server");
        assert_eq!(get("port").value, "2456");
        assert_eq!(get("world").value, "Dedicated");
        assert_eq!(get("password").value, "secret");
        assert_eq!(get("crossplay").value, "true");
        assert_eq!(get("public").value, "false");
    }

    #[test]
    fn write_forces_savedir_and_preserves_steamappid_line() {
        // Calls this module's own read/write directly (not the shared
        // crate::config::{read,write} dispatch via the global active-game switch) —
        // that global races under parallel test execution, as the enshrouded
        // adapter's equivalent test already flagged in its own comments.
        let dir = std::env::temp_dir().join(format!("pwsm-valheim-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("start_headless_server.bat"), SAMPLE).unwrap();

        let mut fields = read(&dir).unwrap();
        for f in fields.iter_mut() {
            if f.key == "name" {
                f.value = "Renamed".to_string();
            }
        }
        write(&dir, &fields).unwrap();

        let text = fs::read_to_string(dir.join("start_headless_server.bat")).unwrap();
        assert!(text.contains("set SteamAppId=892970"));
        assert!(text.contains("-name \"Renamed\""));
        assert!(text.contains(&format!("-savedir \"{}\"", dir.join("saves").display())));

        fs::remove_dir_all(&dir).ok();
    }
}
