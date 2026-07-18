import { useCallback, useEffect, useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import logo from "./assets/logo.png";
import { api, type AppConfig, type GameInfo, type StatusInfo } from "./api";
import ServerPage from "./components/ServerPage";
import ConfigPage from "./components/ConfigPage";
import DashboardPage from "./components/DashboardPage";
import BackupsPage from "./components/BackupsPage";
import AutomationPage from "./components/AutomationPage";
import LogsPage from "./components/LogsPage";
import SettingsPage from "./components/SettingsPage";
import ConnectPage from "./components/ConnectPage";
import SavesPage from "./components/SavesPage";
import ModsPage from "./components/ModsPage";
import FirstRunWizard from "./components/FirstRunWizard";

type Page =
  | "server"
  | "dashboard"
  | "connect"
  | "config"
  | "backups"
  | "automation"
  | "mods"
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
  { id: "mods", label: "🧩 Mods" },
  { id: "saves", label: "🧬 Save tools" },
  { id: "logs", label: "📜 Activity" },
  { id: "settings", label: "🔧 Settings" },
];

export default function App() {
  const [page, setPage] = useState<Page>("server");
  const [status, setStatus] = useState<StatusInfo | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);
  const [wizardOpen, setWizardOpen] = useState(true);
  const [games, setGames] = useState<GameInfo[]>([]);

  useEffect(() => {
    api.gamesList().then(setGames).catch(() => {});
  }, []);

  // If the active game hides the current page (e.g. switching to Enshrouded while on
  // Mods/Save tools), fall back to the Server page.
  useEffect(() => {
    const gameId = config?.profiles.find((p) => p.id === config.activeProfile)?.game ?? "palworld";
    const g = games.find((x) => x.id === gameId);
    if (
      (page === "mods" && g?.modsKind === "none") ||
      (page === "saves" && gameId !== "palworld")
    ) {
      setPage("server");
    }
  }, [config, games, page]);

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

  // Quiet check for a new app version on launch.
  useEffect(() => {
    check()
      .then((u) => {
        if (u) notify(`App update v${u.version} available — Settings → Check for app updates.`);
      })
      .catch(() => {});
  }, [notify]);

  const activeProfileObj = config?.profiles.find((p) => p.id === config.activeProfile);
  const activeName = activeProfileObj?.name;
  const activeGame = games.find((g) => g.id === activeProfileObj?.game);
  const activeGameId = activeProfileObj?.game ?? "palworld";
  const activeGameName = activeGame?.displayName ?? "Palworld";

  // Hide pages that don't apply to the active game: Mods (no mod support, e.g.
  // Enshrouded) and Save tools (Palworld's GVAS format only). Dashboard always
  // applies — it shows host performance even for games with no live-control protocol.
  const visibleNav = NAV.filter((n) => {
    if (n.id === "mods") return activeGame?.modsKind !== "none";
    if (n.id === "saves") return activeGameId === "palworld";
    return true;
  });

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img className="brand-logo" src={logo} alt="Rhyse Gaming" />
          <div className="brand-sub">{activeGameName}</div>
        </div>
        <nav className="nav">
          {visibleNav.map((n) => (
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
          v0.4.10
        </div>
      </aside>

      <main className="content">
        {page === "server" && (
          <ServerPage
            status={status}
            config={config}
            refresh={refresh}
            notify={notify}
            gameName={activeGameName}
          />
        )}
        {page === "dashboard" && <DashboardPage notify={notify} />}
        {page === "connect" && <ConnectPage notify={notify} gameName={activeGameName} />}
        {page === "config" && <ConfigPage notify={notify} status={status} />}
        {page === "backups" && <BackupsPage config={config} notify={notify} />}
        {page === "automation" && (
          <AutomationPage
            config={config}
            refresh={refresh}
            notify={notify}
            gameName={activeGameName}
          />
        )}
        {page === "mods" && <ModsPage notify={notify} status={status} />}
        {page === "saves" && <SavesPage notify={notify} />}
        {page === "logs" && <LogsPage />}
        {page === "settings" && (
          <SettingsPage config={config} refresh={refresh} notify={notify} />
        )}
      </main>

      {toast && <div className={`toast ${toast.error ? "error" : ""}`}>{toast.msg}</div>}

      {wizardOpen && status && !status.installed && (
        <FirstRunWizard
          status={status}
          refresh={refresh}
          notify={notify}
          gameName={activeGameName}
          games={games}
          activeGameId={activeGameId}
          activeProfileId={activeProfileObj?.id}
          profileCount={config?.profiles.length ?? 1}
          liveControl={activeGame?.liveControl ?? "rest"}
          onClose={() => setWizardOpen(false)}
        />
      )}
    </div>
  );
}
