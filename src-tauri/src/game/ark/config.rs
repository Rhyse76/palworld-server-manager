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

use std::collections::{HashMap, HashSet};
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

/// Apply changed values to a file's text in place, preserving everything else. Fields
/// whose composite key already exists as a line have only their value rewritten (and
/// only if it actually changed); fields with no existing line (catalog-only settings
/// the user has just seen for the first time) are inserted into their section, which
/// is created if the file doesn't have it yet.
fn apply(file_id: &str, text: &str, fields: &[ConfigField]) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing = text.ends_with('\n');
    let (mut lines, entries) = parse(file_id, text);

    let by_key: HashMap<&str, &ConfigField> = fields.iter().map(|f| (f.key.as_str(), f)).collect();
    let mut matched: HashSet<&str> = HashSet::new();
    for e in &entries {
        if let Some(f) = by_key.get(e.composite.as_str()) {
            matched.insert(e.composite.as_str());
            if f.value != e.value {
                lines[e.line] = format!("{}={}", e.key, serialize(&f.value, &e.kind));
            }
        }
    }

    // New-to-this-file fields, grouped by section so each section's inserts land together.
    let mut by_section: Vec<(&str, Vec<(&str, String)>)> = Vec::new();
    for f in fields {
        if matched.contains(f.key.as_str()) {
            continue;
        }
        let mut parts = f.key.splitn(3, '|');
        let (Some(fid), Some(section), Some(key_occ)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if fid != file_id {
            continue;
        }
        let key = key_occ.split('#').next().unwrap_or(key_occ);
        let value = serialize(&f.value, &f.kind);
        match by_section.iter_mut().find(|(s, _)| *s == section) {
            Some((_, changes)) => changes.push((key, value)),
            None => by_section.push((section, vec![(key, value)])),
        }
    }
    for (section, changes) in &by_section {
        upsert_section(&mut lines, section, changes);
    }

    let mut out = lines.join(nl);
    if trailing {
        out.push_str(nl);
    }
    out
}

fn apply_to_file(file_id: &str, path: &Path, fields: &[ConfigField]) -> Result<(), String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Ok(()), // file may not exist yet; nothing to update
    };
    fs::write(path, apply(file_id, &text, fields)).map_err(|e| e.to_string())
}

// ---- Public API (called by the ARK adapter's Game trait impl) ----

/// Read the full settings list: start from the curated catalog (see `catalog.rs`) so
/// every well-known setting is shown, then overlay live file values on top — mirrors
/// Palworld's shipped-defaults overlay, since ARK has no defaults file of its own.
pub fn read(install_dir: &Path) -> Result<Vec<ConfigField>, String> {
    let live_gus = read_file("gus", &gus_path(install_dir));
    let live_game = read_file("game", &game_path(install_dir));
    if live_gus.is_empty() && live_game.is_empty() {
        return Err("No ARK config found yet. Install the server and run it once to generate it.".into());
    }

    let mut fields = super::catalog::fields();
    for lf in live_gus.into_iter().chain(live_game) {
        merge_field(&mut fields, lf);
    }
    Ok(fields)
}

/// Overlay a live-parsed field onto the catalog-seeded list: update value/kind if the
/// key is already known (keeping the catalog's label/group), otherwise append it as a
/// new field using its own derived label/group — a live setting the catalog doesn't
/// know about yet.
fn merge_field(fields: &mut Vec<ConfigField>, live: ConfigField) {
    match fields.iter_mut().find(|f| f.key == live.key) {
        Some(existing) => {
            existing.value = live.value;
            existing.kind = live.kind;
        }
        None => fields.push(live),
    }
}

pub fn write(install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
    apply_to_file("gus", &gus_path(install_dir), fields)?;
    apply_to_file("game", &game_path(install_dir), fields)?;
    Ok(())
}

/// Look up a `[ServerSettings]` value from `GameUserSettings.ini`.
pub(super) fn server_setting(install_dir: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(gus_path(install_dir)).ok()?;
    let (_lines, entries) = parse("gus", &text);
    entries
        .into_iter()
        .find(|e| e.section == "[ServerSettings]" && e.key == key)
        .map(|e| e.value)
}

/// Assemble the ARK server launch command line from config values, with sane
/// defaults for anything unset. Shape: `<Map>?listen -Port=… -QueryPort=…
/// -RCONPort=… [-mods=…] [-exclusivejoin] -log`.
pub fn launch_args(install_dir: &Path) -> Vec<String> {
    build_launch_args(|k| server_setting(install_dir, k))
}

fn build_launch_args(get: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let val = |k: &str, default: &str| {
        get(k).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| default.to_string())
    };
    let map = val("MapSelection", "TheIsland_WP");
    let mut args = vec![
        format!("{map}?listen"),
        format!("-Port={}", val("Port", "7777")),
        format!("-QueryPort={}", val("QueryPort", "27015")),
        format!("-RCONPort={}", val("RCONPort", "27020")),
    ];
    if let Some(mods) = get("ActiveMods").filter(|s| !s.trim().is_empty()) {
        args.push(format!("-mods={mods}"));
    }
    if get("ExclusiveJoin").as_deref() == Some("true") {
        args.push("-exclusivejoin".into());
    }
    args.push("-log".into());
    args
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

/// Enable RCON: set `RCONEnabled=True`/`RCONPort` and ensure a `ServerAdminPassword`
/// (generating one if none), all in `[ServerSettings]`. Returns the port + password.
/// The server must be stopped when this runs (ARK rewrites the ini on shutdown) and
/// restarted to apply — the caller enforces the stopped state.
pub fn enable_rcon(install_dir: &Path) -> Result<crate::rest::EnableResult, String> {
    let path = gus_path(install_dir);
    let text = fs::read_to_string(&path)
        .map_err(|_| "No GameUserSettings.ini yet — run the server once to generate it.".to_string())?;
    let port: u16 = server_setting(install_dir, "RCONPort")
        .and_then(|s| s.parse().ok())
        .unwrap_or(27020);
    let existing = server_setting(install_dir, "ServerAdminPassword").filter(|s| !s.trim().is_empty());
    let generated_password = existing.is_none();
    let password = existing.unwrap_or_else(random_password);
    let out = upsert_server_settings(
        &text,
        &[
            ("RCONEnabled", "True".to_string()),
            ("RCONPort", port.to_string()),
            ("ServerAdminPassword", password.clone()),
        ],
    );
    fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(crate::rest::EnableResult { port, admin_password: password, generated_password })
}

/// Update or insert keys within `[ServerSettings]`, preserving the rest of the file.
fn upsert_server_settings(text: &str, changes: &[(&str, String)]) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    upsert_section(&mut lines, "[ServerSettings]", changes);
    let mut out = lines.join(nl);
    if trailing {
        out.push_str(nl);
    }
    out
}

/// Update or insert `changes` within `section`, preserving the rest of the file.
/// Missing keys are inserted right after the section header; the section is appended
/// (on its own line, blank-separated from prior content) if it doesn't exist yet.
fn upsert_section(lines: &mut Vec<String>, section: &str, changes: &[(&str, String)]) {
    let hi = match lines.iter().position(|l| l.trim() == section) {
        Some(i) => i,
        None => {
            if lines.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push(section.to_string());
            lines.len() - 1
        }
    };
    // End of the section (next header or EOF), computed before any inserts.
    let end = (hi + 1..lines.len())
        .find(|&i| lines[i].trim().starts_with('['))
        .unwrap_or(lines.len());

    let mut to_insert = Vec::new();
    for (key, value) in changes {
        let existing = (hi + 1..end).find(|&i| {
            lines[i].split_once('=').map(|(k, _)| k.trim() == *key).unwrap_or(false)
        });
        match existing {
            Some(i) => lines[i] = format!("{key}={value}"),
            None => to_insert.push(format!("{key}={value}")),
        }
    }
    for (offset, line) in to_insert.into_iter().enumerate() {
        lines.insert(hi + 1 + offset, line);
    }
}

/// Small non-cryptographic password for local admin convenience.
fn random_password() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        ^ std::process::id() as u64;
    (0..16)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            CHARS[(seed % CHARS.len() as u64) as usize] as char
        })
        .collect()
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
    fn builds_launch_command_from_config() {
        let vals: HashMap<&str, &str> = [
            ("MapSelection", "Ragnarok_WP"),
            ("Port", "7777"),
            ("QueryPort", "27025"),
            ("RCONPort", "27020"),
            ("ActiveMods", "940975,927090"),
            ("ExclusiveJoin", "true"),
        ]
        .into_iter()
        .collect();
        let args = build_launch_args(|k| vals.get(k).map(|s| s.to_string()));
        assert_eq!(args[0], "Ragnarok_WP?listen");
        assert!(args.contains(&"-Port=7777".to_string()));
        assert!(args.contains(&"-QueryPort=27025".to_string()));
        assert!(args.contains(&"-RCONPort=27020".to_string()));
        assert!(args.contains(&"-mods=940975,927090".to_string()));
        assert!(args.contains(&"-exclusivejoin".to_string()));
        assert!(args.contains(&"-log".to_string()));

        // Defaults when nothing is configured; no mods/exclusivejoin.
        let d = build_launch_args(|_| None);
        assert_eq!(d[0], "TheIsland_WP?listen");
        assert!(d.contains(&"-QueryPort=27015".to_string()));
        assert!(!d.iter().any(|a| a.starts_with("-mods")));
        assert!(!d.contains(&"-exclusivejoin".to_string()));
    }

    #[test]
    fn upsert_adds_and_updates_server_settings() {
        let text = "[ServerSettings]\nXPMultiplier=5\n\n[SessionSettings]\nSessionName=Rhyse\n";
        let out = upsert_server_settings(
            text,
            &[
                ("XPMultiplier", "10".into()), // update existing
                ("RCONEnabled", "True".into()), // insert new
            ],
        );
        assert!(out.contains("XPMultiplier=10"));
        assert!(out.contains("RCONEnabled=True"));
        assert!(out.contains("SessionName=Rhyse")); // other section preserved
        // The inserted key lands inside [ServerSettings], before the next section.
        assert!(out.find("RCONEnabled").unwrap() < out.find("[SessionSettings]").unwrap());
    }

    fn field(key: &str, value: &str, kind: &str) -> ConfigField {
        ConfigField { key: key.into(), value: value.into(), kind: kind.into(), ..Default::default() }
    }

    #[test]
    fn write_edits_in_place_and_preserves_the_rest() {
        let fields = vec![
            field("gus|[ServerSettings]|XPMultiplier#0", "10", "int"),
            field("gus|[ServerSettings]|ExclusiveJoin#0", "false", "bool"),
        ];
        let out = apply("gus", GUS, &fields);

        assert!(out.contains("XPMultiplier=10")); // changed
        assert!(out.contains("ExclusiveJoin=False")); // bool re-serialized
        assert!(out.contains(";METADATA=(Diff=true)")); // comment preserved
        assert!(out.contains("DifficultyOffset=1.00")); // untouched value keeps exact text
        assert!(out.contains("AdminListURL=\"file://C:/ARK/allowed.txt\"")); // quotes preserved
        assert!(out.contains("[SessionSettings]")); // section preserved

        // Unchanged fields (same value supplied) must not alter their line.
        let same = vec![field("gus|[ServerSettings]|DifficultyOffset#0", "1.00", "float")];
        assert!(apply("gus", GUS, &same).contains("DifficultyOffset=1.00"));
    }

    #[test]
    fn write_inserts_catalog_only_fields_into_their_existing_section() {
        // No line for this key in GUS yet (as if seeded by the catalog and edited by
        // the user) — it must be inserted into [ServerSettings], not dropped.
        let fields = vec![field("gus|[ServerSettings]|MaxTamedDinos#0", "6000", "int")];
        let out = apply("gus", GUS, &fields);
        assert!(out.contains("MaxTamedDinos=6000"));
        assert!(out.find("MaxTamedDinos").unwrap() < out.find("[SessionSettings]").unwrap());
    }

    #[test]
    fn write_inserts_a_missing_section_when_needed() {
        let fields = vec![field(
            "game|[/Script/ShooterGame.ShooterGameMode]|MaxTribeLogs#0",
            "800",
            "int",
        )];
        let out = apply("game", "", &fields);
        assert!(out.contains("[/Script/ShooterGame.ShooterGameMode]"));
        assert!(out.contains("MaxTribeLogs=800"));
    }

    #[test]
    fn merge_field_overlays_catalog_and_appends_live_only_keys() {
        let mut fields = vec![ConfigField {
            group: "Rates & Multipliers".into(),
            ..field("gus|[ServerSettings]|XPMultiplier#0", "1.0", "float")
        }];

        merge_field(&mut fields, field("gus|[ServerSettings]|XPMultiplier#0", "3.0", "float"));
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].value, "3.0");
        assert_eq!(fields[0].group, "Rates & Multipliers"); // catalog group preserved

        merge_field(
            &mut fields,
            ConfigField {
                group: "Session".into(),
                ..field("gus|[SessionSettings]|SessionName#0", "Rhyse Island", "string")
            },
        );
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[1].group, "Session");
    }
}
