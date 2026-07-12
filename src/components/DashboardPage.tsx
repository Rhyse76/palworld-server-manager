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
  const [history, setHistory] = useState<{ players: number; fps: number }[]>([]);
  const [bans, setBans] = useState<string[]>([]);
  const [unbanId, setUnbanId] = useState("");
  // Live-control mechanism of the active game: "rest" (Palworld), "rcon" (ARK), or "none".
  const [live, setLive] = useState<"rest" | "rcon" | "none" | null>(null);

  useEffect(() => {
    api.gameInfo().then((g) => setLive(g.liveControl)).catch(() => setLive("rest"));
  }, []);

  async function loadBans() {
    try {
      setBans(await api.bansList());
    } catch {
      /* ignore */
    }
  }
  async function unban(id: string) {
    if (!id.trim()) return;
    try {
      await api.restUnban(id.trim());
      notify(`Unbanned ${id.trim()}.`);
      setUnbanId("");
      setTimeout(loadBans, 500);
    } catch (e) {
      notify(String(e), true);
    }
  }

  const fetchAll = useCallback(async () => {
    if (live === null) return;
    try {
      if (live === "rest") {
        const ov = await api.restOverview();
        setOverview(ov);
        setHistory((h) => [
          ...h.slice(-89),
          { players: ov.metrics.currentplayernum, fps: ov.metrics.serverfps },
        ]);
        try {
          setPlayers(await api.restPlayers());
        } catch {
          setPlayers([]);
        }
        loadBans();
      } else if (live === "rcon") {
        // No REST metrics for RCON games — just the player list + actions.
        setPlayers(await api.restPlayers());
      } else {
        throw new Error("This game doesn't support live control while running.");
      }
      setError(null);
    } catch (e) {
      setError(String(e));
      setOverview(null);
      setPlayers([]);
    } finally {
      setLoading(false);
    }
  }, [live]);

  useEffect(() => {
    fetchAll();
    const id = setInterval(fetchAll, 5000);
    return () => clearInterval(id);
  }, [fetchAll]);

  async function enable() {
    try {
      const res = await api.enableLiveControl();
      const label = live === "rcon" ? "RCON" : "REST API";
      const pw = res.generatedPassword
        ? `A new admin password was generated: ${res.adminPassword}`
        : "Using your existing admin password.";
      const restart = live === "rcon" ? "Start" : "Restart";
      notify(`${label} enabled on port ${res.port}. ${pw} ${restart} the server to apply.`);
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

  if (error) {
    return (
      <>
        <div className="page-head">
          <div>
            <h1>Dashboard</h1>
            <p>Live control over your running server.</p>
          </div>
        </div>
        <div className="card">
          <h2>Not connected</h2>
          <p style={{ color: "var(--text-dim)" }}>{error}</p>
          <div className="row">
            {(live === "rest" || live === "rcon") && (
              <button className="btn primary" onClick={enable}>
                {live === "rcon" ? "Enable RCON" : "Enable REST API"}
              </button>
            )}
            <button className="btn" onClick={fetchAll}>
              Retry
            </button>
          </div>
          <p style={{ color: "var(--text-dim)", fontSize: 12, marginBottom: 0 }}>
            {live === "rest"
              ? "The REST API needs to be enabled with an admin password, and the server must be running. After enabling, restart the server for changes to take effect."
              : live === "rcon"
                ? "RCON must be enabled in the server config (RCONEnabled=True with an admin password) and the server must be running."
                : "This game doesn't offer live control while the server is running."}
          </p>
        </div>
      </>
    );
  }

  const m = overview?.metrics;

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{overview?.info.servername || "Dashboard"}</h1>
          <p>
            {overview?.info.description || "Live server control"}
            {overview?.info.version ? ` · v${overview.info.version}` : ""}
          </p>
        </div>
        <span className="pill ok">
          <span className="dot" /> Connected
        </span>
      </div>

      {live === "rest" && m && (
        <>
          <div className="tiles">
            <Tile label="Players" value={`${m.currentplayernum} / ${m.maxplayernum}`} />
            <Tile label="Server FPS" value={`${m.serverfps}`} />
            <Tile label="Frame time" value={`${m.serverframetime.toFixed(1)} ms`} />
            <Tile label="Uptime" value={formatUptime(m.uptime)} />
          </div>

          {history.length > 1 && (
            <div className="card">
              <h2>Recent activity ({history.length} samples · ~5s each)</h2>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}>
                <Spark label="Players" color="#22c55e" values={history.map((h) => h.players)} />
                <Spark label="Server FPS" color="#3b82f6" values={history.map((h) => h.fps)} />
              </div>
            </div>
          )}
        </>
      )}

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
                {live === "rest" && <th>Level</th>}
                {live === "rest" && <th>Ping</th>}
                <th>Steam ID</th>
                {live === "rest" && <th>Player UID</th>}
                <th style={{ textAlign: "right" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {players.map((p) => (
                <tr key={p.playerId || p.userId || p.name}>
                  <td>{p.name || "(unknown)"}</td>
                  {live === "rest" && <td>{p.level || "—"}</td>}
                  {live === "rest" && <td>{p.ping ? `${p.ping.toFixed(0)} ms` : "—"}</td>}
                  <td>
                    <IdCell value={p.userId} label="Steam ID" notify={notify} />
                  </td>
                  {live === "rest" && (
                    <td>
                      <IdCell value={p.playerId} label="Player UID" notify={notify} />
                    </td>
                  )}
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

      {live === "rest" && (
        <div className="card">
          <h2>Banned players ({bans.length})</h2>
          <div className="row" style={{ marginBottom: bans.length ? 14 : 0 }}>
            <input
              className="search"
              placeholder="Unban by user id (e.g. steam_7656…)"
              value={unbanId}
              onChange={(e) => setUnbanId(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && unban(unbanId)}
            />
            <button className="btn primary" onClick={() => unban(unbanId)} disabled={!unbanId.trim()}>
              Unban
            </button>
          </div>
          {bans.length === 0 ? (
            <p style={{ color: "var(--text-dim)", margin: 0 }}>No banned players.</p>
          ) : (
            <table className="table">
              <tbody>
                {bans.map((id) => (
                  <tr key={id}>
                    <td style={{ fontFamily: "monospace" }}>{id}</td>
                    <td style={{ textAlign: "right" }}>
                      <button className="btn" onClick={() => unban(id)}>
                        Unban
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </>
  );
}

function Spark({ label, color, values }: { label: string; color: string; values: number[] }) {
  const w = 300;
  const h = 70;
  const max = Math.max(...values, 1);
  const min = Math.min(...values, 0);
  const range = max - min || 1;
  const pts = values
    .map((v, i) => {
      const x = (i / Math.max(values.length - 1, 1)) * w;
      const y = h - ((v - min) / range) * h;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  const current = values[values.length - 1] ?? 0;
  return (
    <div>
      <div className="row spread" style={{ marginBottom: 6 }}>
        <span className="tile-label">{label}</span>
        <span style={{ color, fontWeight: 700 }}>{current}</span>
      </div>
      <svg
        viewBox={`0 0 ${w} ${h}`}
        preserveAspectRatio="none"
        style={{
          width: "100%",
          height: 70,
          background: "var(--bg)",
          borderRadius: 6,
          border: "1px solid var(--border)",
        }}
      >
        <polyline
          points={pts}
          fill="none"
          stroke={color}
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      <div className="row spread" style={{ marginTop: 4 }}>
        <span style={{ color: "var(--text-dim)", fontSize: 11 }}>min {min}</span>
        <span style={{ color: "var(--text-dim)", fontSize: 11 }}>max {max}</span>
      </div>
    </div>
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

// A player identifier (Steam ID / player UID): monospace, truncated with the full
// value on hover, and click-to-copy since admins routinely need to paste these.
function IdCell({
  value,
  label,
  notify,
}: {
  value: string;
  label: string;
  notify: (msg: string, error?: boolean) => void;
}) {
  if (!value) return <span style={{ color: "var(--text-dim)" }}>—</span>;
  return (
    <code
      title={`${value} — click to copy`}
      onClick={() => {
        navigator.clipboard.writeText(value);
        notify(`${label} copied.`);
      }}
      style={{
        cursor: "pointer",
        fontSize: 12,
        color: "var(--text-dim)",
        maxWidth: 150,
        display: "inline-block",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        verticalAlign: "bottom",
      }}
    >
      {value}
    </code>
  );
}
