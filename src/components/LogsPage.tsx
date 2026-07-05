import { useEffect, useMemo, useRef, useState } from "react";
import { api, onActivityLog } from "../api";

export default function LogsPage() {
  const [text, setText] = useState("");
  const [filter, setFilter] = useState("");
  const consoleRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.readActivityLog().then(setText).catch(() => {});
    const un = onActivityLog((line) => setText((t) => (t ? `${t}\n${line}` : line)));
    return () => {
      un.then((fn) => fn());
    };
  }, []);

  const lines = useMemo(() => {
    const all = text.split(/\r?\n/).filter(Boolean);
    const q = filter.trim().toLowerCase();
    return q ? all.filter((l) => l.toLowerCase().includes(q)) : all;
  }, [text, filter]);

  useEffect(() => {
    if (!filter) consoleRef.current?.scrollTo(0, consoleRef.current.scrollHeight);
  }, [lines, filter]);

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Activity log</h1>
          <p>Everything the manager does — starts, stops, backups, moderation, and crashes.</p>
        </div>
        <button className="btn" onClick={() => api.readActivityLog().then(setText)}>
          Refresh
        </button>
      </div>

      <div className="toolbar">
        <input
          className="search"
          placeholder="Filter activity…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
      </div>

      <div className="card" style={{ padding: 0 }}>
        <div className="console" ref={consoleRef} style={{ height: 480, border: "none" }}>
          {lines.length === 0 ? (
            <span style={{ color: "var(--text-dim)" }}>
              No activity yet. Start the server or run an action to see entries here.
            </span>
          ) : (
            lines.join("\n")
          )}
        </div>
      </div>
    </>
  );
}
