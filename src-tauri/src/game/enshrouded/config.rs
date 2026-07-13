//! Enshrouded config format: a single JSON file (`enshrouded_server.json`) the server
//! generates complete on first run. Unlike Palworld's blob or ARK's line-based INI,
//! there's no unknown/partial-key problem to solve — this just maps a known, verified
//! set of JSON paths to and from [`ConfigField`]s.
//!
//! `write` re-reads the live file into a `serde_json::Value` and patches only the
//! recognized paths back into it (rather than rebuilding the JSON from the field list),
//! so anything we don't model — `bannedAccounts`, future keys a game update adds — is
//! preserved untouched, the same principle as ARK's in-place line edits.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::ConfigField;

fn config_path(install_dir: &Path) -> PathBuf {
    install_dir.join(super::SPEC.config_rel)
}

/// Top-level scalar fields: `(json key, kind)`. `tags` is handled separately (it's a
/// JSON string array, exposed as one comma-joined field).
const TOP_FIELDS: &[(&str, &str)] = &[
    ("name", "string"),
    ("ip", "string"),
    ("queryPort", "int"),
    ("slotCount", "int"),
    ("voiceChatMode", "enum"),
    ("enableVoiceChat", "bool"),
    ("enableTextChat", "bool"),
    ("gameSettingsPreset", "enum"),
];

/// `gameSettings.*` fields: `(json key, kind)`, in the order the real file has them.
const GAME_SETTINGS_FIELDS: &[(&str, &str)] = &[
    ("playerHealthFactor", "float"),
    ("playerManaFactor", "float"),
    ("playerStaminaFactor", "float"),
    ("playerBodyHeatFactor", "float"),
    ("playerDivingTimeFactor", "float"),
    ("enableDurability", "bool"),
    ("enableStarvingDebuff", "bool"),
    ("foodBuffDurationFactor", "float"),
    ("fromHungerToStarving", "int"),
    ("shroudTimeFactor", "float"),
    ("tombstoneMode", "enum"),
    ("enableGliderTurbulences", "bool"),
    ("weatherFrequency", "enum"),
    ("fishingDifficulty", "enum"),
    ("miningDamageFactor", "float"),
    ("plantGrowthSpeedFactor", "float"),
    ("resourceDropStackAmountFactor", "float"),
    ("factoryProductionSpeedFactor", "float"),
    ("perkUpgradeRecyclingFactor", "float"),
    ("perkCostFactor", "float"),
    ("experienceCombatFactor", "float"),
    ("experienceMiningFactor", "float"),
    ("experienceExplorationQuestsFactor", "float"),
    ("randomSpawnerAmount", "enum"),
    ("aggroPoolAmount", "enum"),
    ("enemyDamageFactor", "float"),
    ("enemyHealthFactor", "float"),
    ("enemyStaminaFactor", "float"),
    ("enemyPerceptionRangeFactor", "float"),
    ("bossDamageFactor", "float"),
    ("bossHealthFactor", "float"),
    ("threatBonus", "float"),
    ("pacifyAllEnemies", "bool"),
    ("tamingStartleRepercussion", "enum"),
    ("dayTimeDuration", "int"),
    ("nightTimeDuration", "int"),
    ("curseModifier", "enum"),
];

/// Fields carrying nanosecond durations rather than plain counts — labeled so editing
/// them isn't a guessing game.
fn duration_label(key: &str) -> Option<&'static str> {
    match key {
        "fromHungerToStarving" => Some("Time from hunger to starving (nanoseconds)"),
        "dayTimeDuration" => Some("Day length (nanoseconds)"),
        "nightTimeDuration" => Some("Night length (nanoseconds)"),
        _ => None,
    }
}

/// The four fixed user-group roles the game itself defines.
const USER_GROUPS: &[&str] = &["Admin", "Friend", "Guest", "Visitor"];
/// Each group's boolean permission flags.
const GROUP_BOOL_FIELDS: &[&str] =
    &["canKickBan", "canAccessInventories", "canEditWorld", "canEditBase", "canExtendBase"];

fn field(key: String, value: String, kind: &str, group: &str, label: &str) -> ConfigField {
    ConfigField { key, value, kind: kind.to_string(), label: label.to_string(), group: group.to_string() }
}

fn value_to_string(v: &Value, kind: &str) -> String {
    match kind {
        "bool" => v.as_bool().unwrap_or(false).to_string(),
        "int" => v.as_i64().map(|n| n.to_string()).unwrap_or_default(),
        "float" => v.as_f64().map(|n| n.to_string()).unwrap_or_default(),
        _ => v.as_str().unwrap_or_default().to_string(), // string, enum
    }
}

fn string_to_value(value: &str, kind: &str) -> Value {
    match kind {
        "bool" => Value::Bool(value == "true"),
        "int" => Value::Number(value.parse::<i64>().unwrap_or(0).into()),
        "float" => serde_json::Number::from_f64(value.parse::<f64>().unwrap_or(0.0))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        _ => Value::String(value.to_string()), // string, enum
    }
}

fn to_fields(root: &Value) -> Vec<ConfigField> {
    let mut fields = Vec::new();

    for &(key, kind) in TOP_FIELDS {
        if let Some(v) = root.get(key) {
            fields.push(field(key.to_string(), value_to_string(v, kind), kind, "Server", ""));
        }
    }
    let tags = root
        .get("tags")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(","))
        .unwrap_or_default();
    fields.push(field("tags".to_string(), tags, "string", "Server", "Tags"));

    if let Some(gs) = root.get("gameSettings") {
        for &(key, kind) in GAME_SETTINGS_FIELDS {
            if let Some(v) = gs.get(key) {
                let composite = format!("gameSettings.{key}");
                let label = duration_label(key).unwrap_or_default();
                fields.push(field(composite, value_to_string(v, kind), kind, "Game Settings", label));
            }
        }
    }

    if let Some(groups) = root.get("userGroups").and_then(Value::as_array) {
        for &group_name in USER_GROUPS {
            let Some(g) = groups.iter().find(|g| g.get("name").and_then(Value::as_str) == Some(group_name))
            else {
                continue;
            };
            let pw = g.get("password").and_then(Value::as_str).unwrap_or_default();
            fields.push(field(
                format!("userGroups.{group_name}.password"),
                pw.to_string(),
                "string",
                "Access & Permissions",
                &format!("{group_name} password"),
            ));
            for &perm in GROUP_BOOL_FIELDS {
                let v = g.get(perm).and_then(Value::as_bool).unwrap_or(false);
                fields.push(field(
                    format!("userGroups.{group_name}.{perm}"),
                    v.to_string(),
                    "bool",
                    "Access & Permissions",
                    &format!("{group_name}: {perm}"),
                ));
            }
            let slots = g.get("reservedSlots").and_then(Value::as_i64).unwrap_or(0);
            fields.push(field(
                format!("userGroups.{group_name}.reservedSlots"),
                slots.to_string(),
                "int",
                "Access & Permissions",
                &format!("{group_name} reserved slots"),
            ));
        }
    }

    fields
}

/// Patch recognized `fields` back into a freshly-read `root`, leaving everything else
/// (unmodeled keys, `bannedAccounts`, etc.) untouched.
fn apply(root: &mut Value, fields: &[ConfigField]) {
    let by_key: HashMap<&str, &ConfigField> = fields.iter().map(|f| (f.key.as_str(), f)).collect();

    for &(key, kind) in TOP_FIELDS {
        if let Some(f) = by_key.get(key) {
            root[key] = string_to_value(&f.value, kind);
        }
    }
    if let Some(f) = by_key.get("tags") {
        let arr: Vec<Value> = f
            .value
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Value::String(s.to_string()))
            .collect();
        root["tags"] = Value::Array(arr);
    }

    for &(key, kind) in GAME_SETTINGS_FIELDS {
        let composite = format!("gameSettings.{key}");
        if let Some(f) = by_key.get(composite.as_str()) {
            root["gameSettings"][key] = string_to_value(&f.value, kind);
        }
    }

    if let Some(groups) = root.get_mut("userGroups").and_then(Value::as_array_mut) {
        for group in groups.iter_mut() {
            let Some(name) = group.get("name").and_then(Value::as_str).map(str::to_string) else {
                continue;
            };
            if let Some(f) = by_key.get(format!("userGroups.{name}.password").as_str()) {
                group["password"] = Value::String(f.value.clone());
            }
            for &perm in GROUP_BOOL_FIELDS {
                if let Some(f) = by_key.get(format!("userGroups.{name}.{perm}").as_str()) {
                    group[perm] = Value::Bool(f.value == "true");
                }
            }
            if let Some(f) = by_key.get(format!("userGroups.{name}.reservedSlots").as_str()) {
                group["reservedSlots"] = Value::Number(f.value.parse::<i64>().unwrap_or(0).into());
            }
        }
    }
}

pub fn read(install_dir: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(config_path(install_dir))
        .map_err(|_| "No config found yet. Install the server and run it once to generate it.".to_string())?;
    let root: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(to_fields(&root))
}

pub fn write(install_dir: &Path, fields: &[ConfigField]) -> Result<(), String> {
    let path = config_path(install_dir);
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut root: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    apply(&mut root, fields);
    let out = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    fs::write(&path, out).map_err(|e| e.to_string())
}

/// Parse a single Enshrouded config file (e.g. an imported `enshrouded_server.json`).
pub fn import(path: &Path) -> Result<Vec<ConfigField>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|_| "File is neither a valid config preset (.json) nor an enshrouded_server.json".to_string())?;
    let fields = to_fields(&root);
    if fields.is_empty() {
        return Err("No settings were found in that file.".into());
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Structurally identical to a real generated file but with placeholder passwords —
    // never the real ones.
    const SAMPLE: &str = r#"{
        "name": "Test Server",
        "saveDirectory": "./savegame",
        "logDirectory": "./logs",
        "ip": "0.0.0.0",
        "queryPort": 15637,
        "slotCount": 16,
        "tags": [],
        "voiceChatMode": "Proximity",
        "enableVoiceChat": false,
        "enableTextChat": false,
        "gameSettingsPreset": "Default",
        "gameSettings": {
            "playerHealthFactor": 1,
            "enableDurability": true,
            "fromHungerToStarving": 600000000000,
            "tombstoneMode": "AddBackpackMaterials",
            "perkUpgradeRecyclingFactor": 0.5
        },
        "userGroups": [
            { "name": "Admin", "password": "placeholder-admin", "canKickBan": true,
              "canAccessInventories": true, "canEditWorld": true, "canEditBase": true,
              "canExtendBase": true, "reservedSlots": 0 },
            { "name": "Friend", "password": "placeholder-friend", "canKickBan": false,
              "canAccessInventories": true, "canEditWorld": true, "canEditBase": true,
              "canExtendBase": false, "reservedSlots": 0 },
            { "name": "Guest", "password": "placeholder-guest", "canKickBan": false,
              "canAccessInventories": false, "canEditWorld": true, "canEditBase": false,
              "canExtendBase": false, "reservedSlots": 0 },
            { "name": "Visitor", "password": "placeholder-visitor", "canKickBan": false,
              "canAccessInventories": false, "canEditWorld": false, "canEditBase": false,
              "canExtendBase": false, "reservedSlots": 0 }
        ],
        "bannedAccounts": []
    }"#;

    #[test]
    fn parses_top_level_and_game_settings() {
        let root: Value = serde_json::from_str(SAMPLE).unwrap();
        let fields = to_fields(&root);
        let get = |k: &str| fields.iter().find(|f| f.key == k).unwrap();

        assert_eq!(get("name").value, "Test Server");
        assert_eq!(get("queryPort").kind, "int");
        assert_eq!(get("queryPort").value, "15637");
        assert_eq!(get("enableVoiceChat").kind, "bool");
        assert_eq!(get("gameSettings.playerHealthFactor").value, "1");
        assert_eq!(get("gameSettings.perkUpgradeRecyclingFactor").value, "0.5");
        assert_eq!(get("gameSettings.fromHungerToStarving").value, "600000000000");
        assert!(get("gameSettings.fromHungerToStarving").label.contains("nanoseconds"));
    }

    #[test]
    fn parses_user_groups_by_role() {
        let root: Value = serde_json::from_str(SAMPLE).unwrap();
        let fields = to_fields(&root);
        let get = |k: &str| fields.iter().find(|f| f.key == k).unwrap();

        assert_eq!(get("userGroups.Admin.password").value, "placeholder-admin");
        assert_eq!(get("userGroups.Admin.canKickBan").value, "true");
        assert_eq!(get("userGroups.Guest.canEditBase").value, "false");
        assert_eq!(get("userGroups.Visitor.reservedSlots").kind, "int");
    }

    #[test]
    fn apply_patches_known_fields_and_preserves_the_rest() {
        let mut root: Value = serde_json::from_str(SAMPLE).unwrap();
        let changes = vec![
            field("name".into(), "Renamed".into(), "string", "Server", ""),
            field("gameSettings.playerHealthFactor".into(), "2.5".into(), "float", "Game Settings", ""),
            field("userGroups.Admin.password".into(), "new-admin-pw".into(), "string", "Access & Permissions", ""),
            field("tags".into(), "pvp, modded".into(), "string", "Server", "Tags"),
        ];
        apply(&mut root, &changes);

        assert_eq!(root["name"], "Renamed");
        assert_eq!(root["gameSettings"]["playerHealthFactor"], 2.5);
        assert_eq!(root["userGroups"][0]["password"], "new-admin-pw");
        assert_eq!(root["tags"], serde_json::json!(["pvp", "modded"]));

        // Untouched fields survive verbatim.
        assert_eq!(root["queryPort"], 15637);
        assert_eq!(root["userGroups"][1]["password"], "placeholder-friend");
        assert_eq!(root["bannedAccounts"], serde_json::json!([]));
    }

    #[test]
    fn import_rejects_non_json() {
        assert!(import(Path::new("does-not-exist.json")).is_err());
    }
}
