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
    ("gus", "[ServerSettings]", "EnablePvPGamma", "bool", "false", "Difficulty & PvP"),
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
    ("gus", "[ServerSettings]", "AllowFlyerCarryPvE", "bool", "false", "Dinos & Taming"),
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
    ("gus", "[ServerSettings]", "ExclusiveJoin", "bool", "false", "Access & Whitelist"),
    ("gus", "[ServerSettings]", "AdminListURL", "string", "", "Access & Whitelist"),
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

    // ---- 2026-07-24: cross-referenced against a real, currently-running server's
    // GameUserSettings.ini/Game.ini (not guessed) to close the gap from ~128 to the
    // fuller set of settings ARK actually writes/accepts. Values are that server's
    // own proven-working ones, except ClusterDirectoryName/ClusterID which use an
    // empty default (those two are inherently machine/server-specific, not something
    // to default from one particular install). ActiveMods and the complex
    // ConfigAddNPCSpawnEntriesContainer struct were deliberately left out (owned by
    // the dedicated Mods feature, and too advanced/rare for a simple catalog entry,
    // respectively) -- both still show up fine via the live-file overlay if present.
    ("gus", "[ServerSettings]", "StructureDamageMultiplier", "int", "6", "Structures"),
    ("gus", "[ServerSettings]", "StartTimeHour", "int", "-1", "World & Start Time"),
    ("gus", "[ServerSettings]", "StartTimeOverride", "bool", "false", "World & Start Time"),
    ("gus", "[ServerSettings]", "PreventSpawnAnimations", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "OnlyAllowSpecifiedEngrams", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "AllowHideDamageSourceFromLogs", "bool", "true", "Player & Tribe"),
    ("gus", "[ServerSettings]", "RaidDinoCharacterFoodDrainMultiplier", "int", "1", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "AllowRaidDinoFeeding", "bool", "false", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "DisableWeatherFog", "bool", "false", "Misc"),
    ("gus", "[ServerSettings]", "AdminLogging", "bool", "false", "Misc"),
    ("gus", "[ServerSettings]", "MaxTributeItems", "int", "50", "Tribute & Transfers"),
    ("gus", "[ServerSettings]", "OxygenSwimSpeedStatMultiplier", "float", "1.00", "Misc"),
    ("gus", "[ServerSettings]", "StructurePickupHoldDuration", "float", "0.5", "Structures"),
    ("gus", "[ServerSettings]", "PreventMateBoost", "bool", "false", "Misc"),
    ("gus", "[ServerSettings]", "StructurePickupTimeAfterPlacement", "int", "30", "Structures"),
    ("gus", "[ServerSettings]", "AlwaysAllowStructurePickup", "bool", "true", "Structures"),
    ("gus", "[ServerSettings]", "ForceAllStructureLocking", "bool", "false", "Structures"),
    ("gus", "[ServerSettings]", "DisableStructureDecayPvE", "bool", "true", "Structures"),
    ("gus", "[ServerSettings]", "OverrideStructurePlatformPrevention", "bool", "false", "Structures"),
    ("gus", "[ServerSettings]", "TheMaxStructuresInRange", "int", "10500", "Structures"),
    ("gus", "[ServerSettings]", "DisableCryopodEnemyCheck", "bool", "false", "Cryo"),
    ("gus", "[ServerSettings]", "DisablePvEGamma", "bool", "false", "Difficulty & PvP"),
    ("gus", "[ServerSettings]", "DontAlwaysNotifyPlayerJoined", "bool", "true", "Player & Tribe"),
    ("gus", "[ServerSettings]", "GlobalVoiceChat", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "ProximityChat", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "PreventDiseases", "bool", "true", "Player & Tribe"),
    ("gus", "[ServerSettings]", "NonPermanentDiseases", "bool", "true", "Player & Tribe"),
    ("gus", "[ServerSettings]", "PreventTribeAlliances", "bool", "false", "Player & Tribe"),
    ("gus", "[ServerSettings]", "Port", "int", "7777", "Network & Console"),
    ("gus", "[ServerSettings]", "QueryPort", "int", "27025", "Network & Console"),
    ("gus", "[ServerSettings]", "RCONPort", "int", "27020", "Network & Console"),
    ("gus", "[ServerSettings]", "RCONServerGameLogBuffer", "int", "600", "Network & Console"),
    ("gus", "[ServerSettings]", "PlatformSaddleBuildAreaBoundsMultiplier", "int", "1", "Misc"),
    ("gus", "[ServerSettings]", "MaxTamedDinos_SoftTameLimit", "int", "5000", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "MaxTamedDinos_SoftTameLimit_CountdownForDeletionDuration", "int", "604800", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "DestroyTamesOverTheSoftTameLimit", "bool", "false", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "DisableBattleEye", "bool", "true", "Network & Console"),
    ("gus", "[ServerSettings]", "RCONEnabled", "bool", "true", "Network & Console"),
    ("gus", "[ServerSettings]", "EnableCrossPlay", "bool", "true", "Crossplay & Platform"),
    // Not in the reference file (this server doesn't restrict platforms), but this is
    // the exact key from the original bug report: real format is an unquoted
    // parenthesized list, e.g. "(Steam,Xbox,PS5,Mac)" -- "enum" kind is deliberate
    // here (not "string", which would wrongly wrap it in quotes on write for a
    // brand-new key); empty default means "no restriction" until the user sets one.
    ("gus", "[ServerSettings]", "CrossplayPlatforms", "enum", "", "Crossplay & Platform"),
    ("gus", "[ServerSettings]", "ConsoleAccess", "bool", "true", "Network & Console"),
    ("gus", "[ServerSettings]", "ForceAllowCaveFlyers", "bool", "true", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "DisableDinoDecayPvE", "bool", "false", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "PvPDinoDecay", "bool", "false", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "ImplantSuicideCD", "int", "28800", "Misc"),
    ("gus", "[ServerSettings]", "PvEAllowStructuresAtSupplyDrops", "bool", "true", "Structures"),
    ("gus", "[ServerSettings]", "MaxTributeDinos", "int", "20", "Tribute & Transfers"),
    ("gus", "[ServerSettings]", "AllowCryoFridgeOnSaddle", "bool", "false", "Cryo"),
    ("gus", "[ServerSettings]", "DisableCryopodFridgeRequirement", "bool", "false", "Cryo"),
    ("gus", "[ServerSettings]", "OverrideSecondsUntilBuriedTreasureAutoReveals", "int", "1209600", "Misc"),
    ("gus", "[ServerSettings]", "EnableOldConsole", "bool", "false", "Network & Console"),
    ("gus", "[ServerSettings]", "ClusterDirectoryName", "enum", "", "Clustering"),
    ("gus", "[ServerSettings]", "ClusterID", "enum", "", "Clustering"),
    ("gus", "[ServerSettings]", "CrossARKAllowForeignDinoDownloads", "bool", "false", "Clustering"),
    ("gus", "[ServerSettings]", "ConvertToStore", "bool", "false", "Crossplay & Platform"),
    ("gus", "[ServerSettings]", "UseStore", "bool", "false", "Crossplay & Platform"),
    ("gus", "[ServerSettings]", "DoCustomCosmeticValidation", "bool", "false", "Misc"),
    ("gus", "[ServerSettings]", "WorldBossKingKaijuSpawnTime", "enum", "19:00:00", "World & Start Time"),
    ("gus", "[ServerSettings]", "ForceGachaUnhappyInCaves", "bool", "false", "World & Start Time"),
    ("gus", "[ServerSettings]", "ArmadoggoDeathCooldown", "int", "3600", "World & Start Time"),
    ("gus", "[ServerSettings]", "IgnorePVPMountedWeaponryRestrictions", "bool", "true", "Misc"),
    ("gus", "[ServerSettings]", "AllowTeslaCoilCaveBuildingPVP", "bool", "true", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "MaxCosmoWeaponAmmo", "int", "100", "Misc"),
    ("gus", "[ServerSettings]", "CosmoWeaponAmmoReloadAmount", "int", "10", "Misc"),
    ("gus", "[ServerSettings]", "CosmeticWhitelistOverride", "enum", "", "Misc"),
    ("gus", "[ServerSettings]", "TamedDinoDamageMultiplier", "int", "5", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "TamedDinoResistanceMultiplier", "int", "5", "Dinos & Taming"),
    ("gus", "[ServerSettings]", "LimitBunkersPerTribe", "bool", "true", "Bunkers"),
    ("gus", "[ServerSettings]", "LimitBunkersPerTribeNum", "int", "3", "Bunkers"),
    ("gus", "[ServerSettings]", "AllowBunkersInPreventionZones", "bool", "false", "Bunkers"),
    ("gus", "[ServerSettings]", "AllowRidingDinosInsideBunkers", "bool", "true", "Bunkers"),
    ("gus", "[ServerSettings]", "AllowBunkerModulesAboveGround", "bool", "false", "Bunkers"),
    ("gus", "[ServerSettings]", "AllowDinoAIInsideBunkers", "bool", "true", "Bunkers"),
    ("gus", "[ServerSettings]", "AllowBunkerModulesInPreventionZones", "bool", "false", "Bunkers"),
    ("gus", "[ServerSettings]", "MinDistanceBetweenBunkers", "float", "3000.0", "Bunkers"),
    ("gus", "[ServerSettings]", "EnemyAccessBunkerHPThreshold", "float", "0.25", "Bunkers"),
    ("gus", "[ServerSettings]", "BunkerUnderHPThresholdDmgMultiplier", "float", "0.05", "Bunkers"),
    ("gus", "[ServerSettings]", "CryoHospitalHoursToRegenHP", "float", "1.0", "Cryo"),
    ("gus", "[ServerSettings]", "CryoHospitalHoursToRegenFood", "float", "24.0", "Cryo"),
    ("gus", "[ServerSettings]", "CryoHospitalHoursToDrainTorpor", "float", "1.0", "Cryo"),
    ("gus", "[ServerSettings]", "CryoHospitalMatingCooldownReduction", "float", "2.0", "Cryo"),
    ("gus", "[ServerSettings]", "BloodforgeReinforceExtraDurability", "float", "0.3", "Misc"),
    ("gus", "[ServerSettings]", "BloodforgeReinforceResourceCostMultiplier", "float", "3.0", "Misc"),
    ("gus", "[ServerSettings]", "BloodforgeReinforceSpeedMultiplier", "float", "0.1", "Misc"),
    ("gus", "[ServerSettings]", "MaxActiveOutposts", "int", "10", "Outposts"),
    ("gus", "[ServerSettings]", "MaxActiveResourceCaches", "int", "5", "Outposts"),
    ("gus", "[ServerSettings]", "MaxActiveCityOutposts", "int", "3", "Outposts"),
    ("gus", "[ServerSettings]", "ServerPlatform", "enum", "ALL", "Crossplay & Platform"),
    ("gus", "[ServerSettings]", "ConsoleArguments", "enum", "", "Network & Console"),
    ("gus", "[ServerSettings]", "OutpostSigilRewardMultiplier", "int", "1", "Outposts"),
    ("gus", "[ServerSettings]", "NeedsPowerToActivateAquaticCompartments", "bool", "true", "Misc"),
    ("gus", "[OmegaTeleporters]", "TeleportCooldown", "int", "10", "Misc"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BabyImprintingStatScaleMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BabyCuddleIntervalMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BabyCuddleGracePeriodMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BabyCuddleLoseImprintQualitySpeedMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "GlobalSpoilingTimeMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "GlobalItemDecompositionTimeMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "GlobalCorpseDecompositionTimeMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "PvPZoneStructureDamageMultiplier", "float", "6.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "StructureDamageRepairCooldown", "int", "180", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "IncreasePvPRespawnIntervalCheckPeriod", "float", "300.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "IncreasePvPRespawnIntervalMultiplier", "float", "2.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "IncreasePvPRespawnIntervalBaseAmount", "float", "60.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "ResourceNoReplenishRadiusPlayers", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "ResourceNoReplenishRadiusStructures", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "CropGrowthSpeedMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "LayEggIntervalMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "PoopIntervalMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "CropDecaySpeedMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "MatingIntervalMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "EggHatchSpeedMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BabyMatureSpeedMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BabyFoodConsumptionSpeedMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "DinoTurretDamageMultiplier", "float", "1.0", "Structures"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "DinoHarvestingDamageMultiplier", "float", "3.2", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "PlayerHarvestingDamageMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "CustomRecipeEffectivenessMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "CustomRecipeSkillMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "FuelConsumptionIntervalMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "PhotoModeRangeLimit", "int", "3000", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bDisablePhotoMode", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bIncreasePvPRespawnInterval", "bool", "true", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bFlyerPlatformAllowUnalignedDinoBasing", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bDisableLootCrates", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bAllowCustomRecipes", "bool", "true", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bPassiveDefensesDamageRiderlessDinos", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bUseCorpseLocator", "bool", "true", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bShowCreativeMode", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bHardLimitTurretsInRange", "bool", "false", "Structures"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bLimitTurretsInRange", "bool", "false", "Structures"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "LimitTurretsRange", "int", "10000", "Structures"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "LimitTurretsNum", "int", "100", "Structures"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bDisableStructurePlacementCollision", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bAllowPlatformSaddleMultiFloors", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bAllowUnlimitedRespecs", "bool", "true", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "OverrideMaxExperiencePointsPlayer", "int", "0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "OverrideMaxExperiencePointsDino", "int", "0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "MaxNumberOfPlayersInTribe", "int", "70", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "ExplorerNoteXPMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BossKillXPMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "AlphaKillXPMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "WildKillXPMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "CaveKillXPMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "TamedKillXPMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "UnclaimedKillXPMultiplier", "int", "5", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "SupplyCrateLootQualityMultiplier", "int", "1", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "FishingLootQualityMultiplier", "int", "1", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "CraftingSkillBonusMultiplier", "int", "1", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "DestroyTamesOverLevelClamp", "int", "0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bUseDinoLevelUpAnimations", "bool", "true", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "BabyImprintAmountMultiplier", "float", "1.0", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bPvEDisableFriendlyFire", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "bAllowFlyerSpeedLeveling", "bool", "false", "Gameplay Rules"),
    ("game", "[/Script/ShooterGame.ShooterGameMode]", "MatingSpeedMultiplier", "int", "1", "Gameplay Rules"),
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
        assert!(n >= 250 && n <= 350, "catalog size {n} outside the intended curated range");
    }
}
