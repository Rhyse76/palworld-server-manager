import { useCallback, useEffect, useState } from "react";
import { api, type AppConfig, type StatusInfo } from "./api";
import ServerPage from "./components/ServerPage";
import ConfigPage from "./components/ConfigPage";
import DashboardPage from "./components/DashboardPage";
import BackupsPage from "./components/BackupsPage";
import AutomationPage from "./components/AutomationPage";
import LogsPage from "./components/LogsPage";
import SettingsPage from "./components/SettingsPage";
import ConnectPage from "./components/ConnectPage";
import SavesPage from "./components/SavesPage";

type Page =
  | "server"
  | "dashboard"
  | "connect"
  | "config"
  | "backups"
  | "automation"
  | "saves"
  | "logs"
  | "settings";

interface Toast {
  msg: string;
  error: boolean;
}

const NAV: { id: Page; label: string }[] = [
  { id: "server", label: "🖥️ Server" },
  { id: "dashboard", label: "📊 Dashboard" },
  { id: "connect", label: "🌐 Connect" },
  { id: "config", label: "⚙️ Configuration" },
  { id: "backups", label: "💾 Backups" },
  { id: "automation", label: "⏱️ Automation" },
  { id: "saves", label: "🧬 Save tools" },
  { id: "logs", label: "📜 Activity" },
  { id: "settings", label: "🔧 Settings" },
];

export default function App() {
  const [page, setPage] = useState<Page>("server");
  const [status, setStatus] = useState<StatusInfo | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);

  // Load status and config independently so a transient failure in one doesn't
  // discard the other (previously a single Promise.all could drop a good status).
  // Returns true only when both succeeded.
  const refresh = useCallback(async (): Promise<boolean> => {
    let ok = true;
    try {
      setStatus(await api.getStatus());
    } catch {
      ok = false;
    }
    try {
      setConfig(await api.getAppConfig());
    } catch {
      ok = false;
    }
    return ok;
  }, []);

  useEffect(() => {
    // On a fast startup the Tauri IPC may not be ready for the very first calls,
    // so retry quickly until both load, then settle into a slower poll cadence.
    let stopped = false;
    let timer: number;
    let settled = false;
    const tick = async () => {
      const ok = await refresh();
      if (ok) settled = true;
      if (stopped) return;
      timer = window.setTimeout(tick, settled ? 4000 : 400);
    };
    tick();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, [refresh]);

  const notify = useCallback((msg: string, error = false) => {
    setToast({ msg, error });
    setTimeout(() => setToast(null), 4000);
  }, []);

  const activeName = config?.profiles.find((p) => p.id === config.activeProfile)?.name;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="3" y="4.5" width="18" height="4" rx="1.4" fill="#08110d" />
              <rect x="3" y="10" width="18" height="4" rx="1.4" fill="#08110d" />
              <rect x="3" y="15.5" width="18" height="4" rx="1.4" fill="#08110d" />
            </svg>
          </div>
          <div className="brand-title">
            Palworld
            <small>Server Manager</small>
          </div>
        </div>
        <nav className="nav">
          {NAV.map((n) => (
            <button
              key={n.id}
              className={page === n.id ? "active" : ""}
              onClick={() => setPage(n.id)}
            >
              {n.label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          {activeName && (
            <>
              Profile: {activeName}
              <br />
            </>
          )}
          {status?.running ? "● Server online" : "○ Server offline"}
          <br />
          v0.2.0
        </div>
      </aside>

      <main className="content">
        {page === "server" && (
          <ServerPage status={status} config={config} refresh={refresh} notify={notify} />
        )}
        {page === "dashboard" && <DashboardPage notify={notify} />}
        {page === "connect" && <ConnectPage notify={notify} />}
        {page === "config" && <ConfigPage notify={notify} />}
        {page === "backups" && <BackupsPage notify={notify} />}
        {page === "automation" && (
          <AutomationPage config={config} refresh={refresh} notify={notify} />
        )}
        {page === "saves" && <SavesPage notify={notify} />}
        {page === "logs" && <LogsPage />}
        {page === "settings" && (
          <SettingsPage config={config} refresh={refresh} notify={notify} />
        )}
      </main>

      {toast && <div className={`toast ${toast.error ? "error" : ""}`}>{toast.msg}</div>}
    </div>
  );
}
