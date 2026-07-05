import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface StatusInfo {
  installDir: string;
  installed: boolean;
  running: boolean;
  steamcmdReady: boolean;
}

export interface ServerProfile {
  id: string;
  name: string;
  installDir: string;
}

export interface Automation {
  autoRestartEnabled: boolean;
  restartIntervalHours: number;
  autoBackupEnabled: boolean;
  backupIntervalHours: number;
  keepBackups: number;
  autoRestartOnCrash: boolean;
}

export interface AppConfig {
  activeProfile: string | null;
  profiles: ServerProfile[];
  automation: Automation;
  hideServerConsole: boolean;
}

export type FieldKind = "bool" | "int" | "float" | "string" | "enum";

export interface ConfigField {
  key: string;
  value: string;
  kind: FieldKind;
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

export const api = {
  getStatus: () => invoke<StatusInfo>("get_status"),
  getAppConfig: () => invoke<AppConfig>("get_app_config"),
  setInstallDir: (path: string) => invoke<void>("set_install_dir", { path }),
  installServer: () => invoke<void>("install_server"),
  startServer: () => invoke<void>("start_server"),
  stopServer: () => invoke<void>("stop_server"),
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
  restSave: () => invoke<void>("rest_save"),
  restShutdown: (seconds: number, message: string) =>
    invoke<void>("rest_shutdown", { seconds, message }),
  enableRestApi: () => invoke<EnableResult>("enable_rest_api"),

  // Backups
  backupCreate: () => invoke<string>("backup_create"),
  backupList: () => invoke<BackupInfo[]>("backup_list"),
  backupRestore: (name: string) => invoke<void>("backup_restore", { name }),
  backupDelete: (name: string) => invoke<void>("backup_delete", { name }),
  backupOpenFolder: () => invoke<void>("backup_open_folder"),

  // Profiles
  addProfile: (name: string, path: string) => invoke<string>("add_profile", { name, path }),
  setActiveProfile: (id: string) => invoke<void>("set_active_profile", { id }),
  renameProfile: (id: string, name: string) => invoke<void>("rename_profile", { id, name }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),

  // Automation
  setAutomation: (automation: Automation) => invoke<void>("set_automation", { automation }),
  setHideConsole: (hide: boolean) => invoke<void>("set_hide_console", { hide }),

  // Activity log
  readActivityLog: () => invoke<string>("read_activity_log"),
};

export function onActivityLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>("activity-log", (e) => cb(e.payload));
}

export function onInstallLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>("install-log", (e) => cb(e.payload));
}

export function onInstallProgress(cb: (pct: number) => void): Promise<UnlistenFn> {
  return listen<number>("install-progress", (e) => cb(e.payload));
}
