//! RuneScape: Dragonwilds config format: one Unreal ini, `DedicatedServer.ini`, with
//! every user setting under a single section:
//!
//! ```ini
//! [SectionsToSave]
//! bCanSaveAllSections=true
//!
//! [/Script/Dominion.DedicatedServerSettings]
//! OwnerId=0123456789abcdef0123456789abcdef
//! ServerName=My Community
//! DefaultWorldName=Rune Valley
//! AdminPassword=replace_this_admin_password
//! WorldPassword=
//! ```
//!
//! The server refuses to start until `OwnerId`, `ServerName`, `DefaultWorldName` and
//! `AdminPassword` are all set, so unlike the other adapters `read` returns the known
//! fields (blank) when the file doesn't exist yet and `write` creates it — the user
//! has to be able to fill these in *before* the first start.
//!
//! `write` edits the section in place and leaves every other line alone, the same
//! preserve-the-rest principle as ARK's ini edits.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::ConfigField;

const SECTION: &str = "/Script/Dominion.DedicatedServerSettings";
const GROUP: &str = "Server";

/// Known settings: `(ini key, label)`. All are plain strings.
const KNOWN: &[(&str, &str)] = &[
    ("OwnerId", "Owner ID (your Player ID, from the in-game Settings menu)"),
    ("ServerName", "Server name"),
    ("DefaultWorldName", "Default world name"),
    ("AdminPassword", "Admin password"),
    ("WorldPassword", "World password (blank = open)"),
];

fn config_path(install_dir: &Path) -> PathBuf {
    install_dir.join(super::SPEC.config_rel)
}

fn section_name(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[')?.strip_suffix(']')
}

fn field(key: &str, value: &str) -> ConfigField {
    let label = KNOWN.iter().find(|(k, _)| *k == key).map(|(_, l)| *l).unwrap_or("");
    ConfigField {
        key: key.to_string(),
        value: value.to_string(),
        kind: "string".to_string(),
        label: label.to_string(),
        group: GROUP.to_string(),
        options: Vec::new(),
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
        match fields.iter_mut().find(|f| f.key == key) {
            Some(f) => f.value = value.to_string(),
            None => fields.push(field(key, value)),
        }
    }
    fields
}

/// Patch `fields` into the settings section of `text`, appending keys the section
/// doesn't have yet and creating the section if it's missing.
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

    for f in fields {
        let rendered = format!("{}={}", f.key, f.value);
        let existing = (start + 1..end)
            .find(|&i| lines[i].split_once('=').is_some_and(|(k, _)| k.trim() == f.key));
        match existing {
            Some(i) => lines[i] = rendered,
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

    // Shape documented by Jagex's how-to + hosting guides, with placeholder values.
    const SAMPLE: &str = "\
[SectionsToSave]
bCanSaveAllSections=true

[/Script/Dominion.DedicatedServerSettings]
OwnerId=0123456789abcdef0123456789abcdef
ServerName=My Community
DefaultWorldName=Rune Valley
AdminPassword=placeholder
WorldPassword=

[Some/Other.Section]
Keep=Me
";

    fn get<'a>(fields: &'a [ConfigField], key: &str) -> &'a str {
        &fields.iter().find(|f| f.key == key).unwrap().value
    }

    #[test]
    fn parses_the_documented_format() {
        let fields = parse(SAMPLE);
        assert_eq!(fields.len(), 5);
        assert_eq!(get(&fields, "OwnerId"), "0123456789abcdef0123456789abcdef");
        assert_eq!(get(&fields, "ServerName"), "My Community");
        assert_eq!(get(&fields, "WorldPassword"), "");
    }

    #[test]
    fn apply_edits_in_place_and_preserves_the_rest() {
        let mut fields = parse(SAMPLE);
        fields.iter_mut().find(|f| f.key == "ServerName").unwrap().value = "Renamed".to_string();
        fields.push(field("NewKey", "1"));
        let out = apply(SAMPLE, &fields);

        assert!(out.contains("ServerName=Renamed"));
        assert!(out.contains("bCanSaveAllSections=true"));
        assert!(out.contains("[Some/Other.Section]\r\nKeep=Me"));
        // The new key lands inside the settings section, not the one after it.
        assert!(out.find("NewKey=1").unwrap() < out.find("[Some/Other.Section]").unwrap());
        assert_eq!(parse(&out).len(), 6);
    }

    #[test]
    fn write_creates_the_file_on_a_fresh_install() {
        // Calls this module's own read/write directly, not the shared dispatch via
        // the global active-game switch, which races under parallel test execution.
        let dir = std::env::temp_dir().join(format!("pwsm-dragonwilds-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(read(&dir).is_err()); // not installed

        fs::write(dir.join(super::super::SPEC.server_launcher), "").unwrap();
        let mut fields = read(&dir).unwrap();
        assert_eq!(get(&fields, "ServerName"), "");
        fields.iter_mut().find(|f| f.key == "ServerName").unwrap().value = "Fresh".to_string();
        write(&dir, &fields).unwrap();

        let text = fs::read_to_string(config_path(&dir)).unwrap();
        assert!(text.starts_with("[SectionsToSave]\r\nbCanSaveAllSections=true\r\n"));
        assert_eq!(get(&read(&dir).unwrap(), "ServerName"), "Fresh");

        fs::remove_dir_all(&dir).ok();
    }
}
