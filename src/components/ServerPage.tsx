import { useEffect, useRef, useState } from "react";
import {
  api,
  onInstallLog,
  onInstallProgress,
  type AppConfig,
  type DetectedInstall,
  type StatusInfo,
} from "../api";
import ProfilesCard from "./ProfilesCard";

interface Props {
  status: StatusInfo | null;
  config: AppConfig | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
  gameName: string;
}

/** Well-documented flags only — kept short and per-game so nothing is guessed. */
const COMMON_FLAGS: Record<string, { flag: string; desc: string }[]> = {
  palworld: [
    { flag: "-useperfthreads", desc: "Improves CPU performance (Windows)." },
    { flag: "-NoAsyncLoadingThread", desc: "Loads assets synchronously — official perf tip." },
    { flag: "-UseMultithreadForDS", desc: "Enables multithreading for the dedicated server." },
    { flag: "-publiclobby", desc: "Required for the server to register on the in-game community server list — direct connect works without it, but it won't be listed." },
  ],
  "ark-sa": [
    { flag: "-NoBattlEye", desc: "Disables BattlEye anti-cheat." },
    { flag: "-servergamelog", desc: "Turns on extra server-side game logging." },
  ],
};

/// Map an install-log line to a friendly current-phase label, so the UI clearly
/// shows work is happening even before SteamCMD reports a numeric percentage.
function derivePhase(line: string): string | null {
  const l = line.toLowerCase();
  if (l.includes("downloading steamcmd")) return "Downloading SteamCMD…";
  if (l.includes("extracting")) return "Extracting SteamCMD…";
  if (l.includes("updated itself") || l.includes("running the install again"))
    return "Preparing SteamCMD…";
  if (l.includes("downloading, progress")) return "Downloading server files…";
  if (l.includes("verifying")) return "Verifying server files…";
  if (l.includes("fully installed") || l.includes("steamcmd finished")) return "Finishing up…";
  return null;
}

export default function ServerPage({ status, config, refresh, notify, gameName }: Props) {
  const [log, setLog] = useState<string[]>([]);
  const [progress, setProgress] = useState<number | null>(null);
  const [installing, setInstalling] = useState(false);
  const [phase, setPhase] = useState("");
  const [busy, setBusy] = useState(false);
  const [detected, setDetected] = useState<DetectedInstall[]>([]);
  const [scanning, setScanning] = useState(false);
  const consoleRef = useRef<HTMLDivElement>(null);

  async function scan(): Promise<boolean> {
    setScanning(true);
    try {
      setDetected(await api.detectInstalls());
      return true;
    } catch {
      return false; // backend not ready yet
    } finally {
      setScanning(false);
    }
  }

  // Retry the initial scan until it succeeds — on a fast startup the first call
  // can land before the Tauri IPC is ready, and it must not give up silently.
  useEffect(() => {
    let stopped = false;
    let timer: number;
    const attempt = async () => {
      const ok = await scan();
      if (!ok && !stopped) timer = window.setTimeout(attempt, 400);
    };
    attempt();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, []);

  async function use(path: string, source: string) {
    await api.addProfile(source, path, "palworld");
    notify("Connected — added as a server profile.");
    refresh();
  }

  useEffect(() => {
    const unlisteners = [
      onInstallLog((line) => {
        setLog((l) => [...l.slice(-400), line]);
        const p = derivePhase(line);
        if (p) setPhase(p);
      }),
      onInstallProgress((pct) => setProgress(pct)),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  useEffect(() => {
    consoleRef.current?.scrollTo(0, consoleRef.current.scrollHeight);
  }, [log]);

  async function install() {
    setInstalling(true);
    setProgress(0);
    setPhase("Preparing…");
    setLog((l) => [...l, "> Starting install/update..."]);
    try {
      await api.installServer();
      notify("Server install/update finished.");
    } catch (e) {
      notify(String(e), true);
      setLog((l) => [...l, `! ${e}`]);
    } finally {
      setInstalling(false);
      setProgress(null);
      refresh();
    }
  }

  async function control(action: "start" | "stop" | "restart") {
    setBusy(true);
    try {
      if (action === "start") await api.startServer();
      else if (action === "stop") await api.stopServer();
      else await api.restartServer();
      notify(
        action === "start"
          ? "Server started."
          : action === "stop"
            ? "Server stopped."
            : "Restarting — saving, warning players (10s), then starting back up.",
      );
    } catch (e) {
      notify(String(e), true);
    } finally {
      setTimeout(() => {
        setBusy(false);
        refresh();
      }, 800);
    }
  }

  const installed = status?.installed ?? false;
  const running = status?.running ?? false;

  const activeProfile = config?.profiles.find((p) => p.id === config.activeProfile) ?? null;
  const [launchArgs, setLaunchArgsState] = useState("");
  const [savingArgs, setSavingArgs] = useState(false);

  useEffect(() => {
    setLaunchArgsState(activeProfile?.extraLaunchArgs ?? "");
  }, [activeProfile?.id, activeProfile?.extraLaunchArgs]);

  function toggleFlag(flag: string) {
    const tokens = launchArgs.split(/\s+/).filter(Boolean);
    const has = tokens.includes(flag);
    setLaunchArgsState(has ? tokens.filter((t) => t !== flag).join(" ") : [...tokens, flag].join(" "));
  }

  async function saveLaunchArgs() {
    if (!activeProfile) return;
    setSavingArgs(true);
    try {
      await api.setLaunchArgs(activeProfile.id, launchArgs);
      notify("Launch arguments saved. Restart the server to apply.");
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSavingArgs(false);
    }
  }

  const activeTokens = launchArgs.split(/\s+/).filter(Boolean);
  const flags = activeProfile ? COMMON_FLAGS[activeProfile.game] ?? [] : [];

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Server</h1>
          <p>Install, update, and control your {gameName} dedicated server.</p>
        </div>
        <div className="row">
          <span className={`pill ${installed ? "ok" : "off"}`}>
            <span className="dot" /> {installed ? "Installed" : "Not installed"}
          </span>
          <span className={`pill ${running ? "ok" : "off"}`}>
            <span className="dot" /> {running ? "Running" : "Stopped"}
          </span>
        </div>
      </div>

      <ProfilesCard config={config} refresh={refresh} notify={notify} />

      <div className="card">
        <div className="row spread" style={{ marginBottom: 12 }}>
          <h2 style={{ margin: 0 }}>Existing installations</h2>
          <button className="btn" onClick={scan} disabled={scanning}>
            {scanning ? "Scanning…" : "Rescan"}
          </button>
        </div>
        {detected.length === 0 ? (
          <p style={{ color: "var(--text-dim)", margin: 0 }}>
            {scanning
              ? "Searching Steam libraries and app folders…"
              : `No existing ${gameName} server found. Use Install below, or pick a folder above.`}
          </p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {detected.map((d) => {
              const current = d.path === status?.installDir;
              return (
                <div className="field" key={d.path}>
                  <div style={{ overflow: "hidden" }}>
                    <div className="row" style={{ gap: 8 }}>
                      <span style={{ fontSize: 13 }}>{d.source}</span>
                      {d.hasConfig && (
                        <span className="pill ok" style={{ padding: "2px 8px" }}>
                          <span className="dot" /> config
                        </span>
                      )}
                      {current && (
                        <span className="pill" style={{ padding: "2px 8px" }}>
                          in use
                        </span>
                      )}
                    </div>
                    <div className="path" style={{ marginTop: 6, border: "none", padding: 0 }}>
                      {d.path}
                    </div>
                  </div>
                  <button
                    className="btn primary"
                    onClick={() => use(d.path, d.source)}
                    disabled={current || installing}
                  >
                    {current ? "Connected" : "Use this"}
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="card">
        <h2>Install / Update</h2>
        <p style={{ color: "var(--text-dim)", marginTop: 0 }}>
          Downloads SteamCMD (first time only) and installs or updates the {gameName}
          {" "}dedicated server via Steam. The full server is several GB.
        </p>
        <div className="row">
          <button className="btn primary" onClick={install} disabled={installing}>
            {installing ? "Working…" : installed ? "Update server" : "Install server"}
          </button>
        </div>
        {installing &&
          (() => {
            const pct = progress ?? 0;
            const determinate = pct > 0;
            return (
              <div style={{ marginTop: 14 }}>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    color: "var(--text-dim)",
                    fontSize: 13,
                    marginBottom: 6,
                  }}
                >
                  <span>{phase || "Preparing…"}</span>
                  {determinate && <span>{pct.toFixed(1)}%</span>}
                </div>
                <div className={`progress${determinate ? "" : " indeterminate"}`}>
                  <span style={determinate ? { width: `${pct}%` } : undefined} />
                </div>
                <p style={{ color: "var(--text-dim)", fontSize: 12, margin: "8px 0 0" }}>
                  The server is several GB — this can take a few minutes. Live progress shows in the
                  console below.
                </p>
              </div>
            );
          })()}
      </div>

      <div className="card">
        <h2>Controls</h2>
        <div className="row">
          <button
            className="btn primary"
            onClick={() => control("start")}
            disabled={!installed || running || busy || installing}
          >
            Start server
          </button>
          <button
            className="btn"
            onClick={() => control("restart")}
            disabled={!running || busy || installing}
          >
            Restart server
          </button>
          <button
            className="btn danger"
            onClick={() => control("stop")}
            disabled={!running || busy}
          >
            Stop server
          </button>
        </div>
        <p style={{ color: "var(--text-dim)", margin: "12px 0 0", fontSize: 13 }}>
          Restart gracefully saves the world and warns players (needs live control — the REST
          API or RCON — enabled); without it, it force-restarts.
        </p>
      </div>

      <div className="card">
        <h2>Launch arguments</h2>
        <p style={{ color: "var(--text-dim)", marginTop: 0 }}>
          Extra command-line flags appended after the app's own launch args. Takes effect on the
          next server start.
        </p>
        {flags.length > 0 && (
          <div className="row" style={{ flexWrap: "wrap", gap: 8, marginBottom: 12 }}>
            {flags.map((f) => {
              const on = activeTokens.includes(f.flag);
              return (
                <button
                  key={f.flag}
                  className={`btn ${on ? "primary" : ""}`}
                  title={f.desc}
                  onClick={() => toggleFlag(f.flag)}
                >
                  {f.flag}
                </button>
              );
            })}
          </div>
        )}
        <div className="row">
          <input
            type="text"
            placeholder="e.g. -useperfthreads -NoAsyncLoadingThread"
            value={launchArgs}
            onChange={(e) => setLaunchArgsState(e.target.value)}
            style={{ flex: 1 }}
          />
          <button className="btn primary" onClick={saveLaunchArgs} disabled={savingArgs || !activeProfile}>
            {savingArgs ? "Saving…" : "Save"}
          </button>
        </div>
      </div>

      <div className="card">
        <h2>Console</h2>
        <div className="console" ref={consoleRef}>
          {log.length === 0 ? (
            <span style={{ color: "var(--text-dim)" }}>SteamCMD output will appear here…</span>
          ) : (
            log.join("\n")
          )}
        </div>
      </div>
    </>
  );
}
