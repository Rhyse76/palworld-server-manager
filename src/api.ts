import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface StatusInfo {
  installDir: string;
  installed: boolean;
  running: boolean;
  steamcmdReady: boolean;
}

export interface GameInfo {
  id: string;
  displayName: string;
  /** Base name of the config file, e.g. "PalWorldSettings.ini". */
  configFile: string;
  /** "local-files" = drop-in mod files (Palworld); "curseforge-ids" = an id list
   * the server downloads/updates itself (ARK: SA); "none" = no mod support. */
  modsKind: "local-files" | "curseforge-ids" | "none";
  liveControl: "rest" | "rcon" | "none";
}

export interface ServerProfile {
  id: string;
  name: string;
  installDir: string;
  /** Game id this profile manages (e.g. "palworld", "ark-sa"). */
  game: string;
  /** Freeform extra command-line args, appended after the game's own auto-generated ones. */
  extraLaunchArgs: string;
  /** This profile's own auto-restart/auto-backup/crash-watchdog settings. */
  automation: Automation;
}

export interface Automation {
  autoRestartEnabled: boolean;
  restartIntervalHours: number;
  autoBackupEnabled: boolean;
  backupIntervalHours: number;
  keepBackups: number;
  autoRestartOnCrash: boolean;
  smartRestart: boolean;
  autoUpdateEnabled: boolean;
  autoUpdateIntervalHours: number;
}

export interface Announcement {
  id: string;
  message: string;
  intervalMinutes: number;
  enabled: boolean;
}

export interface UpdateStatus {
  installedBuild: string;
  latestBuild: string;
  updateAvailable: boolean;
  checked: boolean;
}

export interface Discord {
  enabled: boolean;
  /** Legacy single webhook URL, no longer edited from the UI. */
  webhookUrl: string;
  notifyServer: boolean;
  notifyPlayers: boolean;
  notifyBackups: boolean;
  /** Per-game webhook URLs, keyed by game id (e.g. "palworld", "ark-sa"). */
  webhooks: Record<string, string>;
}

export interface AppConfig {
  activeProfile: string | null;
  profiles: ServerProfile[];
  discord: Discord;
  announcements: Announcement[];
  backupMirrorDir: string;
  hideServerConsole: boolean;
  curseforgeApiKey: string;
}

export type FieldKind = "bool" | "int" | "float" | "string" | "enum";

export interface ConfigField {
  key: string;
  value: string;
  kind: FieldKind;
  /** Friendly display name; UI falls back to `key` when empty. */
  label?: string;
  /** Group/section heading; empty renders ungrouped. */
  group?: string;
  /** Known-good values for an "enum" field, rendered as a dropdown. Empty = free text. */
  options?: string[];
}

export interface DetectedInstall {
  path: string;
  source: string;
  hasConfig: boolean;
}

export interface ServerInfo {
  version: string;
  servername: string;
  description: string;
}

export interface Metrics {
  serverfps: number;
  currentplayernum: number;
  maxplayernum: number;
  serverframetime: number;
  uptime: number;
}

export interface Overview {
  info: ServerInfo;
  metrics: Metrics;
}

export interface Player {
  name: string;
  playerId: string;
  userId: string;
  ping: number;
  level: number;
}

export interface BackupInfo {
  name: string;
  sizeBytes: number;
  modified: number;
}

export interface EnableResult {
  port: number;
  adminPassword: string;
  generatedPassword: boolean;
}

export interface HostStats {
  cpuPercent: number;
  memUsedMb: number;
  memTotalMb: number;
  memPercent: number;
  serverCpuPercent: number | null;
  serverMemMb: number | null;
}

export const api = {
  getStatus: () => invoke<StatusInfo>("get_status"),
  getAppConfig: () => invoke<AppConfig>("get_app_config"),
  gameInfo: () => invoke<GameInfo>("game_info"),
  hostStats: () => invoke<HostStats>("host_stats"),
  setInstallDir: (path: string) => invoke<void>("set_install_dir", { path }),
  installServer: () => invoke<void>("install_server"),
  startServer: () => invoke<void>("start_server"),
  stopServer: () => invoke<void>("stop_server"),
  restartServer: () => invoke<void>("restart_server"),
  readConfig: () => invoke<ConfigField[]>("read_config"),
  writeConfig: (fields: ConfigField[]) => invoke<void>("write_config", { fields }),
  detectInstalls: () => invoke<DetectedInstall[]>("detect_installs"),
  exportConfig: (fields: ConfigField[], dest: string) =>
    invoke<void>("export_config", { fields, dest }),
  importConfig: (path: string) => invoke<ConfigField[]>("import_config", { path }),

  // REST live dashboard
  restOverview: () => invoke<Overview>("rest_overview"),
  restPlayers: () => invoke<Player[]>("rest_players"),
  restAnnounce: (message: string) => invoke<void>("rest_announce", { message }),
  restKick: (userid: string, message: string) => invoke<void>("rest_kick", { userid, message }),
  restBan: (userid: string, message: string) => invoke<void>("rest_ban", { userid, message }),
  restUnban: (userid: string) => invoke<void>("rest_unban", { userid }),
  bansList: () => invoke<string[]>("bans_list"),

  // ARK: SA player access lists
  arkExclusiveJoinList: () => invoke<string[]>("ark_exclusive_join_list"),
  arkSetExclusiveJoinList: (ids: string[]) => invoke<void>("ark_set_exclusive_join_list", { ids }),
  arkAdminsList: () => invoke<string[]>("ark_admins_list"),
  arkSetAdminsList: (ids: string[]) => invoke<void>("ark_set_admins_list", { ids }),
  restSave: () => invoke<void>("rest_save"),
  restShutdown: (seconds: number, message: string) =>
    invoke<void>("rest_shutdown", { seconds, message }),
  enableLiveControl: () => invoke<EnableResult>("enable_live_control"),

  // Backups
  backupCreate: () => invoke<string>("backup_create"),
  backupList: () => invoke<BackupInfo[]>("backup_list"),
  backupRestore: (name: string) => invoke<void>("backup_restore", { name }),
  backupDelete: (name: string) => invoke<void>("backup_delete", { name }),
  backupOpenFolder: () => invoke<void>("backup_open_folder"),
  setBackupMirror: (dir: string) => invoke<void>("set_backup_mirror", { dir }),

  // Profiles
  defaultInstallDir: (game: string) => invoke<string>("default_install_dir", { game }),
  addProfile: (name: string, path: string, game: string) =>
    invoke<string>("add_profile", { name, path, game }),
  gamesList: () => invoke<GameInfo[]>("games_list"),
  setActiveProfile: (id: string) => invoke<void>("set_active_profile", { id }),
  renameProfile: (id: string, name: string) => invoke<void>("rename_profile", { id, name }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),
  setLaunchArgs: (id: string, args: string) => invoke<void>("set_launch_args", { id, args }),

  // Automation
  setAutomation: (automation: Automation) => invoke<void>("set_automation", { automation }),
  setAnnouncements: (announcements: Announcement[]) =>
    invoke<void>("set_announcements", { announcements }),
  checkUpdate: () => invoke<UpdateStatus>("check_update"),
  setHideConsole: (hide: boolean) => invoke<void>("set_hide_console", { hide }),
  setDiscord: (discord: Discord) => invoke<void>("set_discord", { discord }),
  discordTest: () => invoke<void>("discord_test"),

  // Activity log
  readActivityLog: () => invoke<string>("read_activity_log"),

  // Mods (local files — Palworld)
  modsList: () => invoke<ModInfo[]>("mods_list"),
  modSetEnabled: (name: string, enabled: boolean) =>
    invoke<void>("mod_set_enabled", { name, enabled }),
  modInstall: (path: string) => invoke<string>("mod_install", { path }),
  modRemove: (name: string) => invoke<void>("mod_remove", { name }),

  // Mods (CurseForge id list — ARK: SA)
  modIdsList: () => invoke<string[]>("mods_id_list"),
  modIdAdd: (id: string) => invoke<void>("mod_id_add", { id }),
  modIdRemove: (id: string) => invoke<void>("mod_id_remove", { id }),
  modIdDeleteFiles: (id: string) => invoke<void>("mod_id_delete_files", { id }),
  setCurseforgeKey: (key: string) => invoke<void>("set_curseforge_key", { key }),
  curseforgeSearch: (query: string) => invoke<CurseForgeMod[]>("curseforge_search", { query }),

  // Saves (M4)
  inspectSave: () => invoke<SaveInfo>("inspect_save"),

  // Connectivity
  networkInfo: () => invoke<NetworkInfo>("network_info"),
  networkForward: () => invoke<string>("network_forward"),
  networkUnforward: () => invoke<string>("network_unforward"),
  networkReachability: () => invoke<Reachability>("network_reachability"),
};

export interface NetworkInfo {
  publicIp: string;
  localIp: string;
  port: number;
  portListening: boolean;
  /** The server's configured PublicIP (e.g. a Tailscale IP or domain), if set. */
  configuredIp: string;
}

export interface Reachability {
  serverRunning: boolean;
  usingOverlay: boolean;
  routerForwarding: boolean | null;
  verdict: "ready" | "not_ready" | "unknown";
  message: string;
}

export interface ModInfo {
  name: string;
  enabled: boolean;
  sizeBytes: number;
}

export interface CurseForgeMod {
  id: number;
  name: string;
  summary: string;
  downloadCount: number;
  thumbnailUrl: string | null;
  websiteUrl: string | null;
}

export interface SaveInfo {
  path: string;
  compressedSize: number;
  decompressedSize: number;
  isGvas: boolean;
  saveType: number;
}

export function onActivityLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>("activity-log", (e) => cb(e.payload));
}

export function onInstallLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>("install-log", (e) => cb(e.payload));
}

export function onInstallProgress(cb: (pct: number) => void): Promise<UnlistenFn> {
  return listen<number>("install-progress", (e) => cb(e.payload));
}
