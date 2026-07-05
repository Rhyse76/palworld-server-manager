import { useCallback, useEffect, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { api, type Overview, type Player } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
}

function formatUptime(seconds: number): string {
  if (!seconds) return "—";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

export default function DashboardPage({ notify }: Props) {
  const [overview, setOverview] = useState<Overview | null>(null);
  const [players, setPlayers] = useState<Player[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [broadcast, setBroadcast] = useState("");

  const fetchAll = useCallback(async () => {
    try {
      const ov = await api.restOverview();
      setOverview(ov);
      setError(null);
      try {
        setPlayers(await api.restPlayers());
      } catch {
        setPlayers([]);
      }
    } catch (e) {
      setError(String(e));
      setOverview(null);
      setPlayers([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAll();
    const id = setInterval(fetchAll, 5000);
    return () => clearInterval(id);
  }, [fetchAll]);

  async function enable() {
    try {
      const res = await api.enableRestApi();
      const pw = res.generatedPassword
        ? `A new admin password was generated: ${res.adminPassword}`
        : "Using your existing admin password.";
      notify(`REST API enabled on port ${res.port}. ${pw} Restart the server to apply.`);
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function send(action: () => Promise<void>, ok: string) {
    try {
      await action();
      notify(ok);
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function doBroadcast() {
    if (!broadcast.trim()) return;
    await send(() => api.restAnnounce(broadcast.trim()), "Message broadcast.");
    setBroadcast("");
  }

  async function moderate(p: Player, kind: "kick" | "ban") {
    const yes = await ask(`${kind === "kick" ? "Kick" : "Ban"} ${p.name}?`, {
      title: `Confirm ${kind}`,
      kind: "warning",
    });
    if (!yes) return;
    const msg = kind === "kick" ? "You were kicked by an admin." : "You were banned by an admin.";
    await send(
      () => (kind === "kick" ? api.restKick(p.userId, msg) : api.restBan(p.userId, msg)),
      `${p.name} ${kind === "kick" ? "kicked" : "banned"}.`,
    );
    fetchAll();
  }

  async function gracefulShutdown() {
    const yes = await ask("Shut down the server in 30 seconds?", {
      title: "Confirm shutdown",
      kind: "warning",
    });
    if (!yes) return;
    await send(
      () => api.restShutdown(30, "Server shutting down in 30 seconds."),
      "Shutdown scheduled (30s).",
    );
  }

  if (loading) {
    return <div className="empty">Connecting to server…</div>;
  }

  if (error || !overview) {
    return (
      <>
        <div className="page-head">
          <div>
            <h1>Dashboard</h1>
            <p>Live control over your running server via the REST API.</p>
          </div>
        </div>
        <div className="card">
          <h2>Not connected</h2>
          <p style={{ color: "var(--text-dim)" }}>{error ?? "The REST API is unavailable."}</p>
          <div className="row">
            <button className="btn primary" onClick={enable}>
              Enable REST API
            </button>
            <button className="btn" onClick={fetchAll}>
              Retry
            </button>
          </div>
          <p style={{ color: "var(--text-dim)", fontSize: 12, marginBottom: 0 }}>
            The REST API needs to be enabled with an admin password, and the server must be
            running. After enabling, restart the server for changes to take effect.
          </p>
        </div>
      </>
    );
  }

  const m = overview.metrics;

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{overview.info.servername || "Dashboard"}</h1>
          <p>
            {overview.info.description || "Live server control"}
            {overview.info.version ? ` · v${overview.info.version}` : ""}
          </p>
        </div>
        <span className="pill ok">
          <span className="dot" /> Connected
        </span>
      </div>

      <div className="tiles">
        <Tile label="Players" value={`${m.currentplayernum} / ${m.maxplayernum}`} />
        <Tile label="Server FPS" value={`${m.serverfps}`} />
        <Tile label="Frame time" value={`${m.serverframetime.toFixed(1)} ms`} />
        <Tile label="Uptime" value={formatUptime(m.uptime)} />
      </div>

      <div className="card">
        <h2>Broadcast</h2>
        <div className="row">
          <input
            className="search"
            placeholder="Message to all players…"
            value={broadcast}
            onChange={(e) => setBroadcast(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && doBroadcast()}
          />
          <button className="btn primary" onClick={doBroadcast} disabled={!broadcast.trim()}>
            Send
          </button>
        </div>
        <div className="row" style={{ marginTop: 14 }}>
          <button className="btn" onClick={() => send(() => api.restSave(), "World saved.")}>
            Save world
          </button>
          <button className="btn danger" onClick={gracefulShutdown}>
            Graceful shutdown (30s)
          </button>
        </div>
      </div>

      <div className="card">
        <h2>Players online ({players.length})</h2>
        {players.length === 0 ? (
          <p style={{ color: "var(--text-dim)", margin: 0 }}>No players connected.</p>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Level</th>
                <th>Ping</th>
                <th style={{ textAlign: "right" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {players.map((p) => (
                <tr key={p.playerId || p.userId || p.name}>
                  <td>{p.name || "(unknown)"}</td>
                  <td>{p.level || "—"}</td>
                  <td>{p.ping ? `${p.ping.toFixed(0)} ms` : "—"}</td>
                  <td style={{ textAlign: "right" }}>
                    <button className="btn" onClick={() => moderate(p, "kick")}>
                      Kick
                    </button>
                    <button
                      className="btn danger"
                      style={{ marginLeft: 8 }}
                      onClick={() => moderate(p, "ban")}
                    >
                      Ban
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </>
  );
}

function Tile({ label, value }: { label: string; value: string }) {
  return (
    <div className="tile">
      <div className="tile-value">{value}</div>
      <div className="tile-label">{label}</div>
    </div>
  );
}
