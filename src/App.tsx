import { useCallback, useEffect, useState } from "react";
import { api, type StatusInfo } from "./api";
import ServerPage from "./components/ServerPage";
import ConfigPage from "./components/ConfigPage";

type Page = "server" | "config";

interface Toast {
  msg: string;
  error: boolean;
}

export default function App() {
  const [page, setPage] = useState<Page>("server");
  const [status, setStatus] = useState<StatusInfo | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);

  const refresh = useCallback(async () => {
    try {
      setStatus(await api.getStatus());
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
          <button className={page === "server" ? "active" : ""} onClick={() => setPage("server")}>
            🖥️ Server
          </button>
          <button className={page === "config" ? "active" : ""} onClick={() => setPage("config")}>
            ⚙️ Configuration
          </button>
        </nav>
        <div className="sidebar-footer">
          {status?.running ? "● Server online" : "○ Server offline"}
          <br />
          v0.1.0 · M1
        </div>
      </aside>

      <main className="content">
        {page === "server" && <ServerPage status={status} refresh={refresh} notify={notify} />}
        {page === "config" && <ConfigPage notify={notify} />}
      </main>

      {toast && <div className={`toast ${toast.error ? "error" : ""}`}>{toast.msg}</div>}
    </div>
  );
}
