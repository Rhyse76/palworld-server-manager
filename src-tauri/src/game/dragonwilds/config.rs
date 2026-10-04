//! RuneScape: Dragonwilds config format: one Unreal ini, `DedicatedServer.ini`, with
//! every user setting under a single section. Verified against a real server run
//! (2026-10-04) — this is the file after its first start:
//!
//! ```ini
//! ;METADATA=(Diff=true, UseCommands=true)
//! [SectionsToSave]
//! bCanSaveAllSections=true
//!
//! [/Script/Dominion.DedicatedServerSettings]
//! ServerGuid=1719B90442140D4EF80358908459483A
//! OwnerId=0123456789abcdef0123456789abcdef
//! ServerName=My Community
//! DefaultWorldName=Rune Valley
//! WorldPassword=
//! PlatformPolicy=Crossplay
//! bAllowSendingCrashDumps=True
//! ```
//!
//! A fresh install ships no ini at all, and the server won't start a joinable session
//! without `OwnerId`, `ServerName`, `DefaultWorldName` and `AdminPassword`. So unlike
//! the other adapters `read` returns the known fields (blank) when the file doesn't
//! exist yet and `write` creates it — the user fills these in *before* the first start.
//!
//! Real-server behavior this module works around:
//! - The server rewrites the file on startup and **removes the `AdminPassword` line**
//!   once it has read it. So a blank value for a key the file doesn't have is never
//!   written — otherwise every save would add back an empty `AdminPassword=`.
//! - `OwnerId` must be 32 hex characters with no dashes. The dashed GUID form is
//!   rejected ("Session setting [OwnerId] is too long") and the session fails to
//!   create, so `write` strips dashes.
//! - `ServerGuid` is the server's own generated identity; it isn't exposed as a field.
//!
//! `write` edits the section in place and leaves every other line alone, the same
//! preserve-the-rest principle as ARK's ini edits.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::ConfigField;

const SECTION: &str = "/Script/Dominion.DedicatedServerSettings";
const GROUP: &str = "Server";

/// Settings always shown, even before the file exists: `(ini key, label)`.
const KNOWN: &[(&str, &str)] = &[
    ("OwnerId", "Owner ID (your Player ID, from the in-game Settings menu)"),
    ("ServerName", "Server name"),
    ("DefaultWorldName", "Default world name"),
    ("AdminPassword", "Admin password (the server removes it from the file once read; blank = unchanged)"),
    ("WorldPassword", "World password (blank = open)"),
];

/// Labels for settings the server adds to the file itself on first start.
const GENERATED_LABELS: &[(&str, &str)] = &[
    ("PlatformPolicy", "Platform policy"),
    ("bAllowSendingCrashDumps", "Send crash dumps to Jagex"),
];

/// Keys in the settings section that are never exposed as fields.
const HIDDEN: &[&str] = &["ServerGuid"];

fn config_path(install_dir: &Path) -> PathBuf {
    install_dir.join(super::SPEC.config_rel)
}

fn section_name(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[')?.strip_suffix(']')
}

fn field(key: &str, value: &str) -> ConfigField {
    let label = KNOWN
        .iter()
        .chain(GENERATED_LABELS)
        .find(|(k, _)| *k == key)
        .map(|(_, l)| *l)
        .unwrap_or("");
    let is_bool = value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false");
    ConfigField {
        key: key.to_string(),
        value: if is_bool { value.to_lowercase() } else { value.to_string() },
        kind: if is_bool { "bool" } else { "string" }.to_string(),
        label: label.to_string(),
        group: GROUP.to_string(),
        options: Vec::new(),
    }
}

/// The value as it's written to the file.
fn serialize(f: &ConfigField) -> String {
    match (f.key.as_str(), f.kind.as_str()) {
        ("OwnerId", _) => f.value.trim().replace('-', ""),
        (_, "bool") => if f.value == "true" { "True" } else { "False" }.to_string(),
        _ => f.value.clone(),
    }
}

/// The known settings (blank when absent), followed by any other key the live file
/// has in the settings section.
fn parse(text: &str) -> Vec<ConfigField> {
    let mut fields: Vec<ConfigField> = KNOWN.iter().map(|(k, _)| field(k, "")).collect();
    let mut in_section = false;
    for line in text.lines() {
        if let Some(name) = section_name(line) {
            in_section = name == SECTION;
            continue;
        }
        if !in_section || line.trim_start().starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());
        if HIDDEN.contains(&key) {
            continue;
        }
        match fields.iter_mut().find(|f| f.key == key) {
            Some(f) => *f = field(key, value),
            None => fields.push(field(key, value)),
        }
    }
    fields
}

/// Patch `fields` into the settings section of `text`, appending keys the section
/// doesn't have yet (unless blank — see module docs) and creating the section if
/// it's missing.
fn apply(text: &str, fields: &[ConfigField]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let start = match lines.iter().position(|l| section_name(l) == Some(SECTION)) {
        Some(i) => i,
        None => {
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(format!("[{SECTION}]"));
            lines.len() - 1
        }
    };
    let mut end = lines[start + 1..]
        .iter()
        .position(|l| section_name(l).is_some())
        .map_or(lines.len(), |i| start + 1 + i);
    // Insert new keys before any blank lines separating this section from the next.
    while end > start + 1 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    for f in fields.iter().filter(|f| !HIDDEN.contains(&f.key.as_str())) {
        let value = serialize(f);
        let rendered = format!("{}={}", f.key, value);
        let existing = (start + 1..end)
            .find(|&i| lines[i].split_once('=').is_some_and(|(k, _)| k.trim() == f.key));
        match existing {
            Some(i) => lines[i] = rendered,
            None if value.is_empty() => {}
            None => {
                lines.insert(end, rendered);
                end += 1;
            }
        }
    }
    lines.join("\r\n") + "\r\n"
}

pub fn read(install_dir: &Path) -> Result<Vec<ConfigField>, String> {
    match fs::read_to_string(config_path(install_dir)) {
        Ok(text) => Ok(parse(&text)),
        Err(_) if install_dir.join(super::SPEC.server_launcher).exists() => Ok(parse("")),
        Err(_) => Err("No DedicatedServer.ini found yet. Install the server first.".to_string()),
    }
}

pub fn write(install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
    let path = config_path(install_dir);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            "[SectionsToSave]\r\nbCanSaveAllSections=true\r\n".to_string()
        }
    };
    fs::write(&path, apply(&text, fields)).map_err(|e| e.to_string())
}

/// Parse a single Dragonwilds config file (e.g. an imported `DedicatedServer.ini`).
pub fn import(path: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if !text.lines().any(|l| section_name(l) == Some(SECTION)) {
        return Err("File is neither a valid config preset (.json) nor a DedicatedServer.ini".to_string());
    }
    Ok(parse(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The file a real server left behind after its first start (2026-10-04), with
    // placeholder ids, plus a trailing foreign section to prove it's left alone.
    const SAMPLE: &str = "\
;METADATA=(Diff=true, UseCommands=true)
[SectionsToSave]
bCanSaveAllSections=true

[/Script/Dominion.DedicatedServerSettings]
ServerGuid=00000000000000000000000000000000
OwnerId=0123456789abcdef0123456789abcdef
ServerName=My Community
DefaultWorldName=Rune Valley
WorldPassword=
PlatformPolicy=Crossplay
bAllowSendingCrashDumps=True

[Some/Other.Section]
Keep=Me
";

    fn get<'a>(fields: &'a [ConfigField], key: &str) -> &'a ConfigField {
        fields.iter().find(|f| f.key == key).unwrap()
    }

    fn set(fields: &mut [ConfigField], key: &str, value: &str) {
        fields.iter_mut().find(|f| f.key == key).unwrap().value = value.to_string();
    }

    #[test]
    fn parses_the_real_format() {
        let fields = parse(SAMPLE);
        assert_eq!(fields.len(), 7); // 5 known + 2 server-generated, ServerGuid hidden
        assert_eq!(get(&fields, "ServerName").value, "My Community");
        assert_eq!(get(&fields, "AdminPassword").value, ""); // removed by the server
        assert_eq!(get(&fields, "PlatformPolicy").value, "Crossplay");
        assert_eq!(get(&fields, "bAllowSendingCrashDumps").kind, "bool");
        assert_eq!(get(&fields, "bAllowSendingCrashDumps").value, "true");
        assert!(fields.iter().all(|f| f.key != "ServerGuid"));
    }

    #[test]
    fn apply_edits_in_place_and_preserves_the_rest() {
        let mut fields = parse(SAMPLE);
        set(&mut fields, "ServerName", "Renamed");
        set(&mut fields, "bAllowSendingCrashDumps", "false");
        fields.push(field("NewKey", "1"));
        let out = apply(SAMPLE, &fields);

        assert!(out.contains("ServerName=Renamed"));
        assert!(out.contains("bAllowSendingCrashDumps=False"));
        assert!(out.contains(";METADATA=(Diff=true, UseCommands=true)"));
        assert!(out.contains("ServerGuid=00000000000000000000000000000000"));
        assert!(out.contains("[Some/Other.Section]\r\nKeep=Me"));
        // The new key lands inside the settings section, not the one after it.
        assert!(out.find("NewKey=1").unwrap() < out.find("[Some/Other.Section]").unwrap());
    }

    #[test]
    fn a_blank_admin_password_is_not_written_back() {
        let mut fields = parse(SAMPLE);
        assert!(!apply(SAMPLE, &fields).contains("AdminPassword"));

        set(&mut fields, "AdminPassword", "changed");
        assert!(apply(SAMPLE, &fields).contains("AdminPassword=changed"));
    }

    #[test]
    fn owner_id_is_written_without_dashes() {
        // The dashed form made a real server fail to create its session.
        let mut fields = parse(SAMPLE);
        set(&mut fields, "OwnerId", "01234567-89ab-cdef-0123-456789abcdef");
        assert!(apply(SAMPLE, &fields).contains("OwnerId=0123456789abcdef0123456789abcdef\r\n"));
    }

    #[test]
    fn write_creates_the_file_on_a_fresh_install() {
        // Calls this module's own read/write directly, not the shared dispatch via
        // the global active-game switch.
        let dir = std::env::temp_dir().join(format!("pwsm-dragonwilds-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(read(&dir).is_err()); // not installed

        fs::write(dir.join(super::super::SPEC.server_launcher), "").unwrap();
        let mut fields = read(&dir).unwrap();
        assert_eq!(get(&fields, "ServerName").value, "");
        set(&mut fields, "ServerName", "Fresh");
        write(&dir, &fields).unwrap();

        let text = fs::read_to_string(config_path(&dir)).unwrap();
        assert!(text.starts_with("[SectionsToSave]\r\nbCanSaveAllSections=true\r\n"));
        assert!(!text.contains("WorldPassword")); // blank and absent: left out
        assert_eq!(get(&read(&dir).unwrap(), "ServerName").value, "Fresh");

        fs::remove_dir_all(&dir).ok();
    }
}
