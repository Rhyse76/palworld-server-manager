import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";

export default function LogsPage() {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [live, setLive] = useState(true);
  const consoleRef = useRef<HTMLDivElement>(null);

  async function load() {
    try {
      setText(await api.readServerLog());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    load();
    if (!live) return;
    const id = setInterval(load, 3000);
    return () => clearInterval(id);
  }, [live]);

  const lines = useMemo(() => {
    const all = text.split(/\r?\n/);
    const q = filter.trim().toLowerCase();
    return q ? all.filter((l) => l.toLowerCase().includes(q)) : all;
  }, [text, filter]);

  useEffect(() => {
    if (live && !filter) consoleRef.current?.scrollTo(0, consoleRef.current.scrollHeight);
  }, [lines, live, filter]);

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Server log</h1>
          <p>Live tail of Pal.log for the active server.</p>
        </div>
        <div className="row">
          <button className={`btn ${live ? "primary" : ""}`} onClick={() => setLive((v) => !v)}>
            {live ? "● Live" : "Paused"}
          </button>
          <button className="btn" onClick={load}>
            Refresh
          </button>
        </div>
      </div>

      <div className="toolbar">
        <input
          className="search"
          placeholder="Filter log lines…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
      </div>

      <div className="card" style={{ padding: 0 }}>
        <div className="console" ref={consoleRef} style={{ height: 460, border: "none" }}>
          {error ? (
            <span style={{ color: "var(--text-dim)" }}>{error}</span>
          ) : (
            lines.join("\n")
          )}
        </div>
      </div>
    </>
  );
}
