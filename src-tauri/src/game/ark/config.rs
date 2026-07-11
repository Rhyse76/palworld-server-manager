//! ARK: Survival Ascended config format.
//!
//! Unlike Palworld's single `OptionSettings` blob, ARK uses standard line-based
//! INI across two files in the same folder — `GameUserSettings.ini` (main server
//! settings) and `Game.ini` (gameplay multipliers). Key differences the parser
//! handles (see `docs/ark-reference.md`):
//!   * multiple `[Section]`s per file,
//!   * **duplicate keys are legal** (arrays like `ConfigOverrideItemMaxQuantity`),
//!     disambiguated here by occurrence index,
//!   * `;` comments and blank lines are preserved,
//!   * writing edits values **in place** — untouched lines keep their exact text,
//!     so comments, ordering, and formatting survive a round-trip.
//!
//! Each field is exposed to the shared model with a composite key
//! `"<file>|<section>|<key>#<occ>"` so it maps back to the exact source line.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::ConfigField;

fn gus_path(install_dir: &Path) -> PathBuf {
    install_dir.join(super::SPEC.config_rel)
}

/// `Game.ini` sits next to `GameUserSettings.ini`.
fn game_path(install_dir: &Path) -> PathBuf {
    gus_path(install_dir)
        .parent()
        .map(|d| d.join("Game.ini"))
        .unwrap_or_else(|| install_dir.join("Game.ini"))
}

struct Entry {
    composite: String,
    section: String,
    key: String,
    value: String,
    kind: String,
    line: usize,
}

/// Client/graphics sections that aren't server config — hidden from the UI.
const HIDDEN_SECTIONS: &[&str] = &[
    "[ScalabilityGroups]",
    "[/Script/ShooterGame.ShooterGameUserSettings]",
    "[/Script/Engine.GameUserSettings]",
    "[Startup]",
];

fn is_hidden(section: &str) -> bool {
    HIDDEN_SECTIONS.contains(&section)
}

/// A readable group heading from a raw INI section header.
fn friendly_section(section: &str) -> String {
    let s = section.trim_start_matches('[').trim_end_matches(']');
    match s {
        "ServerSettings" => "Server settings".into(),
        "SessionSettings" | "/Script/Engine.GameSession" => "Session".into(),
        "ModSettings" => "Mods".into(),
        "MessageOfTheDay" => "Message of the day".into(),
        "/Script/ShooterGame.ShooterGameMode" => "Gameplay (Game.ini)".into(),
        other => other.rsplit(['.', '/']).next().unwrap_or(other).to_string(),
    }
}

fn to_field(e: Entry) -> ConfigField {
    ConfigField {
        label: e.key,
        group: friendly_section(&e.section),
        key: e.composite,
        value: e.value,
        kind: e.kind,
    }
}

/// Parse a file's text into its lines plus the editable settings, tagging each
/// with a composite key unique across (file, section, key, occurrence).
fn parse(file_id: &str, text: &str) -> (Vec<String>, Vec<Entry>) {
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    let mut entries = Vec::new();
    let mut section = String::new();
    let mut occ: HashMap<(String, String), usize> = HashMap::new();

    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            section = t.to_string();
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = t[..eq].trim().to_string();
            let (value, kind) = classify(t[eq + 1..].trim());
            let n = occ.entry((section.clone(), key.clone())).or_insert(0);
            let composite = format!("{file_id}|{section}|{key}#{n}");
            *n += 1;
            entries.push(Entry {
                composite,
                section: section.clone(),
                key,
                value,
                kind,
                line: i,
            });
        }
    }
    (lines, entries)
}

/// Infer a field's logical value + type from its raw INI token.
fn classify(raw: &str) -> (String, String) {
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        return (raw[1..raw.len() - 1].to_string(), "string".into());
    }
    match raw.to_ascii_lowercase().as_str() {
        "true" => return ("true".into(), "bool".into()),
        "false" => return ("false".into(), "bool".into()),
        _ => {}
    }
    if raw.is_empty() {
        return (String::new(), "string".into());
    }
    if raw.parse::<i64>().is_ok() {
        return (raw.to_string(), "int".into());
    }
    if raw.contains('.') && raw.parse::<f64>().is_ok() {
        return (raw.to_string(), "float".into());
    }
    (raw.to_string(), "enum".into())
}

/// Serialize a logical value back to its INI token form.
fn serialize(value: &str, kind: &str) -> String {
    match kind {
        // Non-empty strings are quoted (URLs/paths); empty stays bare (e.g. `ActiveMods=`).
        "string" => {
            if value.is_empty() {
                String::new()
            } else {
                format!("\"{value}\"")
            }
        }
        "bool" => {
            if value == "true" {
                "True".into()
            } else {
                "False".into()
            }
        }
        _ => value.to_string(),
    }
}

fn read_file(file_id: &str, path: &Path) -> Vec<ConfigField> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let (_lines, entries) = parse(file_id, &text);
    entries
        .into_iter()
        .filter(|e| !is_hidden(&e.section))
        .map(to_field)
        .collect()
}

/// Apply changed values to a file's text in place, preserving everything else.
/// Only settings whose value actually changed are rewritten.
fn apply(file_id: &str, text: &str, map: &HashMap<String, String>) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing = text.ends_with('\n');
    let (mut lines, entries) = parse(file_id, text);
    for e in &entries {
        if let Some(new_val) = map.get(&e.composite) {
            if *new_val != e.value {
                lines[e.line] = format!("{}={}", e.key, serialize(new_val, &e.kind));
            }
        }
    }
    let mut out = lines.join(nl);
    if trailing {
        out.push_str(nl);
    }
    out
}

fn apply_to_file(file_id: &str, path: &Path, map: &HashMap<String, String>) -> Result<(), String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Ok(()), // file may not exist yet; nothing to update
    };
    fs::write(path, apply(file_id, &text, map)).map_err(|e| e.to_string())
}

// ---- Public API (called by the ARK adapter's Game trait impl) ----

pub fn read(install_dir: &Path) -> Result<Vec<ConfigField>, String> {
    let mut fields = read_file("gus", &gus_path(install_dir));
    fields.extend(read_file("game", &game_path(install_dir)));
    if fields.is_empty() {
        return Err("No ARK config found yet. Install the server and run it once to generate it.".into());
    }
    Ok(fields)
}

pub fn write(install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
    let map: HashMap<String, String> = fields
        .iter()
        .map(|f| (f.key.clone(), f.value.clone()))
        .collect();
    apply_to_file("gus", &gus_path(install_dir), &map)?;
    apply_to_file("game", &game_path(install_dir), &map)?;
    Ok(())
}

/// Parse a single ARK config file (e.g. an imported `GameUserSettings.ini`).
pub fn import(path: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let (_lines, entries) = parse("gus", &text);
    if entries.is_empty() {
        return Err("No settings were found in that file.".into());
    }
    Ok(entries
        .into_iter()
        .filter(|e| !is_hidden(&e.section))
        .map(to_field)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUS: &str = ";METADATA=(Diff=true)\n\
[ServerSettings]\n\
XPMultiplier=5\n\
DifficultyOffset=1.00\n\
ExclusiveJoin=True\n\
AdminListURL=\"file://C:/ARK/allowed.txt\"\n\
ActiveMods=\n\
MapSelection=TheIsland_WP\n\
\n\
[SessionSettings]\n\
SessionName=Rhyse Island\n";

    const GAME: &str = "[/Script/ShooterGame.ShooterGameMode]\n\
PerLevelStatsMultiplier_DinoWild[0]=1.0\n\
ConfigOverrideItemMaxQuantity=(A)\n\
ConfigOverrideItemMaxQuantity=(B)\n";

    #[test]
    fn classifies_value_types() {
        assert_eq!(classify("5"), ("5".into(), "int".into()));
        assert_eq!(classify("1.00"), ("1.00".into(), "float".into()));
        assert_eq!(classify("True"), ("true".into(), "bool".into()));
        assert_eq!(classify("\"x\""), ("x".into(), "string".into()));
        assert_eq!(classify(""), ("".into(), "string".into()));
        assert_eq!(classify("TheIsland_WP"), ("TheIsland_WP".into(), "enum".into()));
    }

    #[test]
    fn parses_sections_and_values() {
        let (_l, e) = parse("gus", GUS);
        let get = |k: &str| e.iter().find(|x| x.composite == k).unwrap();
        assert_eq!(get("gus|[ServerSettings]|XPMultiplier#0").value, "5");
        assert_eq!(get("gus|[ServerSettings]|ExclusiveJoin#0").kind, "bool");
        assert_eq!(get("gus|[ServerSettings]|AdminListURL#0").value, "file://C:/ARK/allowed.txt");
        assert_eq!(get("gus|[ServerSettings]|ActiveMods#0").value, "");
        assert_eq!(get("gus|[SessionSettings]|SessionName#0").value, "Rhyse Island");
    }

    #[test]
    fn duplicate_keys_get_distinct_composites() {
        let (_l, e) = parse("game", GAME);
        let dupes: Vec<_> = e
            .iter()
            .filter(|x| x.key == "ConfigOverrideItemMaxQuantity")
            .collect();
        assert_eq!(dupes.len(), 2);
        assert!(dupes[0].composite.ends_with("#0"));
        assert!(dupes[1].composite.ends_with("#1"));
        assert_eq!(dupes[0].value, "(A)");
        assert_eq!(dupes[1].value, "(B)");
    }

    #[test]
    fn write_edits_in_place_and_preserves_the_rest() {
        let mut map = HashMap::new();
        map.insert("gus|[ServerSettings]|XPMultiplier#0".to_string(), "10".to_string());
        map.insert("gus|[ServerSettings]|ExclusiveJoin#0".to_string(), "false".to_string());
        let out = apply("gus", GUS, &map);

        assert!(out.contains("XPMultiplier=10")); // changed
        assert!(out.contains("ExclusiveJoin=False")); // bool re-serialized
        assert!(out.contains(";METADATA=(Diff=true)")); // comment preserved
        assert!(out.contains("DifficultyOffset=1.00")); // untouched value keeps exact text
        assert!(out.contains("AdminListURL=\"file://C:/ARK/allowed.txt\"")); // quotes preserved
        assert!(out.contains("[SessionSettings]")); // section preserved
        // Unchanged fields (same value supplied) must not alter their line.
        let mut same = HashMap::new();
        same.insert("gus|[ServerSettings]|DifficultyOffset#0".to_string(), "1.00".to_string());
        assert!(apply("gus", GUS, &same).contains("DifficultyOffset=1.00"));
    }
}
