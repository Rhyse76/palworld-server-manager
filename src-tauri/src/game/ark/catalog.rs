//! Curated catalog of known ARK: Survival Ascended server settings.
//!
//! Unlike Palworld, ARK ships no defaults file — a fresh `GameUserSettings.ini` only
//! has the handful of keys the server happened to write on first launch. This catalog
//! fills in the rest of the well-known settings (shipped-engine defaults) so the Config
//! page shows the full set instead of ~46 keys. `config::read` overlays live file
//! values on top of these; `config::write` inserts any catalog key the user changed
//! into the right file/section since it won't already exist as a line.
//!
//! Each entry maps to the same composite-key shape the live INI parser uses —
//! `"<file>|<section>|<key>#0"` — so catalog and live entries merge by key. Dynamic
//! array settings (per-level stat multiplier overrides, item overrides, etc.) aren't
//! catalogued here; they only show up if already present in the live file.

use crate::config::ConfigField;

/// `(file_id, section, key, kind, default, group)`.
type Entry = (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str);

/// Known ASA map launch codes, confirmed against Steam's DLC listing for app 2399830
/// (ARK: Survival Ascended) plus the community wiki's server-config page — not
/// guessed. Maps released after this was written won't be missing entirely (the
/// field falls back to free text if a live value isn't in this list), just absent
/// from the dropdown until added here.
pub const MAP_OPTIONS: &[&str] = &[
    "TheIsland_WP",
    "TheCenter_WP",
    "ScorchedEarth_WP",
    "Ragnarok_WP",
    "Aberration_WP",
    "Extinction_WP",
    "Valguero_WP",
    "Genesis_WP",
    "Astraeos_WP",
    "LostColony_WP",
    "BobsMissions_WP",
];

const ENTRIES: &[Entry] = &[
    // ---- Map ----
    ("gus", "[ServerSettings]", "MapSelection", "enum", "TheIsland_WP", "Map"),

    // ---- Rates & Multipliers (GameUserSettings.ini [ServerSettings]) ----
    ("gus", "[ServerSettings]", "XPMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "TamingSpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "HarvestAmountMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "HarvestHealthMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "ResourcesRespawnPeriodMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "MatingIntervalMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "EggHatchSpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "BabyMatureSpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "BabyFoodConsumptionSpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "BabyCuddleIntervalMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "BabyImprintingStatScaleMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "BabyCuddleGracePeriodMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "BabyCuddleLoseImprintQualitySpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "CropGrowthSpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "CropDecaySpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "LayEggIntervalMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PoopIntervalMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "FuelConsumptionIntervalMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "GlobalSpoilingTimeMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "GlobalItemDecompositionTimeMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "GlobalCorpseDecompositionTimeMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "HairGrowthSpeedMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "ItemStackSizeMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "CraftingSkillBonusMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "SupplyCrateLootQualityMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "FishingLootQualityMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PassiveTameIntervalMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "WildDinoCharacterFoodDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "TamedDinoCharacterFoodDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "WildDinoTorporDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "TamedDinoTorporDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "DinoCharacterStaminaDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "DinoCharacterHealthRecoveryMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PlayerCharacterWaterDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PlayerCharacterFoodDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PlayerCharacterStaminaDrainMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PlayerCharacterHealthRecoveryMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PvPZoneStructureDamageMultiplier", "float", "6.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PerPlatformMaxStructuresMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "StructurePreventResourceRadiusMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PvEDinoDecayPeriodMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "PvEStructureDecayPeriodMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "CustomRecipeEffectivenessMultiplier", "float", "1.0", "Rates & Multipliers"),
    ("gus", "[ServerSettings]", "CustomRecipeSkillMultiplier", "float", "1.0", "Rates & Multipliers"),

    // ---- Difficulty & PvP ----
    ("gus", "[ServerSettings]", "DifficultyOffset", "float", "1.0", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "OverrideOfficialDifficulty", "float", "5.0", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "ServerPVE", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "ServerHardcore", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "ServerCrosshair", "bool", "true", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "ServerForceNoHUD", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "ShowMapPlayerLocation", "bool", "true", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "EnablePVPGamma", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "DisableStructurePlacementCollision", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "PreventOfflinePvP", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "PreventOfflinePvPInterval", "int", "900", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "IncreasePvPRespawnInterval", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "IncreasePvPRespawnIntervalCheckPeriod", "int", "300", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "IncreasePvPRespawnIntervalMultiplier", "float", "2.0", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "IncreasePvPRespawnIntervalBaseAmount", "int", "60", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "AllowCaveBuildingPvE", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "DisableImprintDinoBuff", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "AllowAnyoneBabyImprintCuddle", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "AutoSavePeriodMinutes", "float", "15.0", "Difficulty & PvP"),

    // ---- Player & Tribe ----
    ("gus", "[ServerSettings]", "MaxNumberOfPlayersInTribe", "int", "0", "Player & Tribe"),
    ("gus", "[ServerSettings]", "AllowThirdPersonPlayer", "bool", "true", "Player & Tribe"),
    ("gus", "[ServerSettings]", "AlwaysNotifyPlayerJoined", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "AlwaysNotifyPlayerLeft", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "ShowFloatingDamageText", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "AllowHitMarkers", "bool", "true", "Player & Tribe"),
    ("gus", "[ServerSettings]", "AllowUnlimitedRespecs", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "KickIdlePlayersPeriod", "int", "3600", "Player & Tribe"),
    ("gus", "[ServerSettings]", "TribeNameChangeCooldown", "int", "15", "Player & Tribe"),
    ("gus", "[ServerSettings]", "OverrideMaxExperiencePointsPlayer", "int", "0", "Player & Tribe"),
    ("gus", "[ServerSettings]", "PreventDownloadSurvivors", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "PreventUploadSurvivors", "bool", "false", "Player & Tribe"),

    // ---- Dinos & Taming ----
    ("gus", "[ServerSettings]", "MaxTamedDinos", "int", "5000", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "MaxPersonalTamedDinos", "int", "0", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "PersonalTamedDinosSaddleStructureCost", "int", "19", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "PreventDownloadDinos", "bool", "false", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "PreventUploadDinos", "bool", "false", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "OverrideMaxExperiencePointsDino", "int", "0", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "AllowFlyerCarryPVE", "bool", "false", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "PassiveDefensesDamageRiderlessDinos", "bool", "true", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "RandomSupplyCratePoints", "bool", "false", "Dinos & Taming"),

    // ---- Structures ----
    ("gus", "[ServerSettings]", "MaxPlatformSaddleStructureLimit", "int", "130", "Structures"),
    ("gus", "[ServerSettings]", "StructureDamageRepairCooldown", "int", "0", "Structures"),
    ("gus", "[ServerSettings]", "EnableExtraStructurePreventionVolumes", "bool", "false", "Structures"),
    ("gus", "[ServerSettings]", "AllowCrateSpawnsOnTopOfStructures", "bool", "false", "Structures"),
    ("gus", "[ServerSettings]", "ClampResourceHarvestDamage", "bool", "false", "Structures"),
    ("gus", "[ServerSettings]", "PreventDownloadItems", "bool", "false", "Structures"),
    ("gus", "[ServerSettings]", "PreventUploadItems", "bool", "false", "Structures"),
    ("gus", "[ServerSettings]", "NoTributeDownloads", "bool", "false", "Structures"),

    // ---- Access & Whitelist ----
    ("gus", "[ServerSettings]", "ServerPassword", "string", "", "Access & Whitelist"),
    ("gus", "[ServerSettings]", "ServerAdminPassword", "string", "", "Access & Whitelist"),
    ("gus", "[ServerSettings]", "SpectatorPassword", "string", "", "Access & Whitelist"),
    ("gus", "[ServerSettings]", "WhitelistOn", "bool", "false", "Access & Whitelist"),
    ("gus", "[ServerSettings]", "AllowCustomRecipes", "bool", "true", "Access & Whitelist"),

    // ---- Misc ----
    ("gus", "[ServerSettings]", "AllowFlyingStaminaRecovery", "bool", "false", "Misc"),
    ("gus", "[ServerSettings]", "AllowMultipleAttachedC4", "bool", "false", "Misc"),
    ("gus", "[ServerSettings]", "ServerAllowAnsel", "bool", "false", "Misc"),
    ("gus", "[ServerSettings]", "UseCorpseLocator", "bool", "true", "Misc"),
    ("gus", "[ServerSettings]", "ClampItemSpoilingTimes", "bool", "false", "Misc"),

    // ---- Session / engine / message of the day ----
    ("gus", "[SessionSettings]", "SessionName", "string", "My ARK Server", "Server Identity"),
    ("gus", "[/Script/Engine.GameSession]", "MaxPlayers", "int", "70", "Server Identity"),
    ("gus", "[MessageOfTheDay]", "Message", "string", "", "Server Identity"),
    ("gus", "[MessageOfTheDay]", "Duration", "int", "20", "Server Identity"),

    // ---- Gameplay rules (Game.ini [/Script/ShooterGame.ShooterGameMode]) ----
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bUseSingleplayerSettings", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bDisableStructureDecayPvE", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bDisableFriendlyFire", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bDisableDinoRiding", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bDisableDinoTaming", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bPvEAllowTribeWar", "bool", "true", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bPvEAllowTribeWarCancel", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bAllowUnclaimDinos", "bool", "true", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bOnlyAllowSpecifiedEngrams", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bAutoPvETimer", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bAutoPvEUseSystemTime", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "AutoPvEStartTimeSeconds", "int", "0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "AutoPvEStopTimeSeconds", "int", "0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "MaxTribeLogs", "int", "400", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "KillXPMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "HarvestXPMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "CraftXPMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "GenericXPMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "SpecialXPMultiplier", "float", "1.0", "Gameplay Rules"),
];

/// The catalog as a `ConfigField` list, keyed the same way the live INI parser keys
/// its entries (occurrence `#0` — none of these are duplicate-key array settings).
pub fn fields() -> Vec<ConfigField> {
    ENTRIES
        .iter()
        .map(|&(file, section, key, kind, default, group)| ConfigField {
            key: format!("{file}|{section}|{key}#0"),
            value: default.to_string(),
            kind: kind.to_string(),
            label: if key == "MapSelection" { "Map".to_string() } else { String::new() },
            group: group.to_string(),
            options: if key == "MapSelection" {
                MAP_OPTIONS.iter().map(|s| s.to_string()).collect()
            } else {
                Vec::new()
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_entry_has_a_unique_composite_key() {
        let fs = fields();
        let keys: HashSet<&str> = fs.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys.len(), fs.len(), "duplicate composite key in the ARK catalog");
    }

    #[test]
    fn covers_the_curated_range() {
        let n = ENTRIES.len();
        assert!(n >= 100 && n <= 160, "catalog size {n} outside the intended curated range");
    }
}
