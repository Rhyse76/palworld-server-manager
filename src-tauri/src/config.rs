//! Shared config model.
//!
//! [`ConfigField`] is the game-agnostic representation of one setting. Parsing and
//! writing a game's actual config file(s) is owned by that game's adapter (see the
//! [`crate::game::Game`] trait); [`read`]/[`write`]/[`import_file`] here delegate to
//! the active game so the rest of the app has one stable API regardless of the
//! underlying format (Palworld INI blob, ARK's multiple INIs + args, Enshrouded JSON).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::game;

/// A single setting. `kind` is one of `bool`, `int`, `float`, `string`, `enum` and
/// drives the UI control; `value` is the logical value (inner text for strings,
/// `"true"`/`"false"` for bools, the number for numbers, the raw token for enums).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigField {
    pub key: String,
    pub value: String,
    pub kind: String,
    /// Friendly display name; the UI falls back to `key` when empty.
    #[serde(default)]
    pub label: String,
    /// Group/section heading for the UI; empty renders ungrouped.
    #[serde(default)]
    pub group: String,
}

/// Find a field's logical value by key.
pub fn find(fields: &[ConfigField], key: &str) -> Option<String> {
    fields.iter().find(|f| f.key == key).map(|f| f.value.clone())
}

/// Insert or update a field, preserving position when it already exists.
pub fn upsert(fields: &mut Vec<ConfigField>, key: &str, value: &str, kind: &str) {
    if let Some(f) = fields.iter_mut().find(|f| f.key == key) {
        f.value = value.to_string();
        f.kind = kind.to_string();
    } else {
        fields.push(ConfigField {
            key: key.to_string(),
            value: value.to_string(),
            kind: kind.to_string(),
            ..Default::default()
        });
    }
}

/// Read the active game's config into a unified field list.
pub fn read(install_dir: &Path) -> Result<Vec<ConfigField>, String> {
    game::active().read_config(install_dir)
}

/// Write a unified field list back to the active game's config.
pub fn write(install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
    game::active().write_config(install_dir, fields)
}

/// Write the given fields to a portable JSON preset file (game-agnostic).
pub fn export_json(fields: &[ConfigField], dest: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(fields).map_err(|e| e.to_string())?;
    fs::write(dest, json).map_err(|e| e.to_string())
}

/// Load fields from either a JSON preset (exported by this app, any game) or the
/// active game's native config file. Returned for review — not written to disk.
pub fn import_file(path: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if let Ok(fields) = serde_json::from_str::<Vec<ConfigField>>(&text) {
        if !fields.is_empty() {
            return Ok(fields);
        }
    }
    game::active().import_config(path)
}
