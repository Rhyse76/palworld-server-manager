import { useCallback, useEffect, useState } from "react";
import { api, type AppConfig, type StatusInfo } from "./api";
import ServerPage from "./components/ServerPage";
import ConfigPage from "./components/ConfigPage";
import DashboardPage from "./components/DashboardPage";
import BackupsPage from "./components/BackupsPage";
import AutomationPage from "./components/AutomationPage";
import LogsPage from "./components/LogsPage";

type Page = "server" | "dashboard" | "config" | "backups" | "automation" | "logs";

interface Toast {
  msg: string;
  error: boolean;
}

const NAV: { id: Page; label: string }[] = [
  { id: "server", label: "🖥️ Server" },
  { id: "dashboard", label: "📊 Dashboard" },
  { id: "config", label: "⚙️ Configuration" },
  { id: "backups", label: "💾 Backups" },
  { id: "automation", label: "⏱️ Automation" },
  { id: "logs", label: "📜 Server log" },
];

export default function App() {
  const [page, setPage] = useState<Page>("server");
  const [status, setStatus] = useState<StatusInfo | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [s, c] = await Promise.all([api.getStatus(), api.getAppConfig()]);
      setStatus(s);
      setConfig(c);
    } catch {
      /* backend not ready yet */
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 4000);
    return () => clearInterval(id);
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
          <div className="brand-mark">P</div>
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
          v0.1.0 · M3
        </div>
      </aside>

      <main className="content">
        {page === "server" && (
          <ServerPage status={status} config={config} refresh={refresh} notify={notify} />
        )}
        {page === "dashboard" && <DashboardPage notify={notify} />}
        {page === "config" && <ConfigPage notify={notify} />}
        {page === "backups" && <BackupsPage notify={notify} />}
        {page === "automation" && (
          <AutomationPage config={config} refresh={refresh} notify={notify} />
        )}
        {page === "logs" && <LogsPage />}
      </main>

      {toast && <div className={`toast ${toast.error ? "error" : ""}`}>{toast.msg}</div>}
    </div>
  );
}
