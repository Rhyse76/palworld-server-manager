//! Palworld config format: nearly every setting lives in one
//! `OptionSettings=(Key=Value,Key=Value,...)` line inside `PalWorldSettings.ini`,
//! with shipped defaults in `DefaultPalWorldSettings.ini`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{upsert, ConfigField};

const HEADER: &str = "[/Script/Pal.PalGameWorldSettings]";

fn config_path(install_dir: &Path) -> PathBuf {
    install_dir.join(super::SPEC.config_rel)
}

/// Read the full settings list: start from the shipped defaults so every setting
/// is shown, then overlay the live `PalWorldSettings.ini` values on top. A partial
/// live config (e.g. one written by "Enable REST API") still shows the complete set
/// with the user's overrides applied.
pub fn read(install_dir: &Path) -> Result<Vec<ConfigField>, String> {
    let default_fields = super::SPEC
        .default_config
        .and_then(|d| parse_ini(&install_dir.join(d)));
    let live_fields = parse_ini(&config_path(install_dir));

    let mut fields = default_fields.unwrap_or_default();
    if let Some(live) = live_fields {
        for lf in live {
            upsert(&mut fields, &lf.key, &lf.value, &lf.kind);
        }
    }

    if fields.is_empty() {
        return Err("No config found yet. Install the server first.".into());
    }
    Ok(fields)
}

pub fn write(install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
    let path = config_path(install_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let line = format!("OptionSettings=({})", serialize_fields(fields));
    let contents = format!("{HEADER}\n{line}\n");
    fs::write(&path, contents).map_err(|e| e.to_string())
}

/// Parse a `PalWorldSettings.ini` file into fields (used by config import).
pub fn import(path: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let blob = extract_option_settings(&text)
        .ok_or("File is neither a valid config preset (.json) nor a PalWorldSettings.ini")?;
    let fields = parse_fields(&blob);
    if fields.is_empty() {
        return Err("No settings were found in that file.".into());
    }
    Ok(fields)
}

/// Parse the `OptionSettings` fields from a specific `.ini` file, if it exists and
/// contains an `OptionSettings=(...)` line.
fn parse_ini(path: &Path) -> Option<Vec<ConfigField>> {
    let text = fs::read_to_string(path).ok()?;
    let blob = extract_option_settings(&text)?;
    Some(parse_fields(&blob))
}

/// Pull the parenthesized body out of the `OptionSettings=(...)` line.
fn extract_option_settings(text: &str) -> Option<String> {
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("OptionSettings="))?;
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(line[open + 1..close].to_string())
}

/// Split on top-level commas (commas inside double quotes don't count).
fn split_top_level(blob: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in blob.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
}

fn parse_fields(blob: &str) -> Vec<ConfigField> {
    split_top_level(blob)
        .into_iter()
        .filter_map(|pair| {
            let (key, raw) = pair.split_once('=')?;
            let key = key.trim().to_string();
            let raw = raw.trim();
            let (value, kind) = classify(raw);
            Some(ConfigField { key, value, kind })
        })
        .collect()
}

/// Infer a field's type + logical value from its raw INI token.
fn classify(raw: &str) -> (String, String) {
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        return (raw[1..raw.len() - 1].to_string(), "string".into());
    }
    match raw {
        "True" | "true" => return ("true".into(), "bool".into()),
        "False" | "false" => return ("false".into(), "bool".into()),
        _ => {}
    }
    if raw.parse::<i64>().is_ok() {
        return (raw.to_string(), "int".into());
    }
    if raw.contains('.') && raw.parse::<f64>().is_ok() {
        return (raw.to_string(), "float".into());
    }
    (raw.to_string(), "enum".into())
}

fn serialize_fields(fields: &[ConfigField]) -> String {
    fields
        .iter()
        .map(|f| format!("{}={}", f.key, serialize_value(f)))
        .collect::<Vec<_>>()
        .join(",")
}

fn serialize_value(f: &ConfigField) -> String {
    match f.kind.as_str() {
        "string" => format!("\"{}\"", f.value),
        "bool" => {
            if f.value == "true" {
                "True".into()
            } else {
                "False".into()
            }
        }
        _ => f.value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_representative_blob() {
        let text = format!(
            "{HEADER}\nOptionSettings=(Difficulty=None,DayTimeSpeedRate=1.000000,\
             bIsPvP=False,ServerName=\"My, Server\",PublicPort=8211)\n"
        );
        let blob = extract_option_settings(&text).unwrap();
        let fields = parse_fields(&blob);
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[3].key, "ServerName");
        assert_eq!(fields[3].value, "My, Server"); // comma inside quotes preserved
        assert_eq!(fields[3].kind, "string");
        assert_eq!(fields[2].kind, "bool");
        assert_eq!(fields[4].kind, "int");

        let out = serialize_fields(&fields);
        assert!(out.contains("ServerName=\"My, Server\""));
        assert!(out.contains("bIsPvP=False"));
        assert!(out.contains("PublicPort=8211"));
    }
}
