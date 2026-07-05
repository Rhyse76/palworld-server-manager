import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface StatusInfo {
  installDir: string;
  installed: boolean;
  running: boolean;
  steamcmdReady: boolean;
}

export interface AppConfig {
  installDir: string | null;
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

export const api = {
  getStatus: () => invoke<StatusInfo>("get_status"),
  getAppConfig: () => invoke<AppConfig>("get_app_config"),
  setInstallDir: (path: string | null) => invoke<void>("set_install_dir", { path }),
  installServer: () => invoke<void>("install_server"),
  startServer: () => invoke<void>("start_server"),
  stopServer: () => invoke<void>("stop_server"),
  readConfig: () => invoke<ConfigField[]>("read_config"),
  writeConfig: (fields: ConfigField[]) => invoke<void>("write_config", { fields }),
  detectInstalls: () => invoke<DetectedInstall[]>("detect_installs"),
  exportConfig: (fields: ConfigField[], dest: string) =>
    invoke<void>("export_config", { fields, dest }),
  importConfig: (path: string) => invoke<ConfigField[]>("import_config", { path }),
};

export function onInstallLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>("install-log", (e) => cb(e.payload));
}

export function onInstallProgress(cb: (pct: number) => void): Promise<UnlistenFn> {
  return listen<number>("install-progress", (e) => cb(e.payload));
}
