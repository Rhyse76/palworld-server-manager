import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  api,
  onInstallLog,
  onInstallProgress,
  type DetectedInstall,
  type StatusInfo,
} from "../api";

interface Props {
  status: StatusInfo | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
}

export default function ServerPage({ status, refresh, notify }: Props) {
  const [log, setLog] = useState<string[]>([]);
  const [progress, setProgress] = useState<number | null>(null);
  const [installing, setInstalling] = useState(false);
  const [busy, setBusy] = useState(false);
  const [detected, setDetected] = useState<DetectedInstall[]>([]);
  const [scanning, setScanning] = useState(false);
  const consoleRef = useRef<HTMLDivElement>(null);

  async function scan() {
    setScanning(true);
    try {
      setDetected(await api.detectInstalls());
    } catch {
      /* ignore */
    } finally {
      setScanning(false);
    }
  }

  useEffect(() => {
    scan();
  }, []);

  async function use(path: string) {
    await api.setInstallDir(path);
    notify("Connected to existing server installation.");
    refresh();
  }

  useEffect(() => {
    const unlisteners = [
      onInstallLog((line) => setLog((l) => [...l.slice(-400), line])),
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

  async function changeDir() {
    const picked = await open({ directory: true, title: "Choose server install folder" });
    if (typeof picked === "string") {
      await api.setInstallDir(picked);
      refresh();
    }
  }

  async function control(action: "start" | "stop") {
    setBusy(true);
    try {
      if (action === "start") await api.startServer();
      else await api.stopServer();
      notify(action === "start" ? "Server started." : "Server stopped.");
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

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Server</h1>
          <p>Install, update, and control your Palworld dedicated server.</p>
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

      <div className="card">
        <h2>Install location</h2>
        <div className="row">
          <span className="path" title={status?.installDir}>
            {status?.installDir ?? "…"}
          </span>
          <button className="btn" onClick={changeDir} disabled={installing}>
            Change…
          </button>
        </div>
      </div>

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
              : "No existing Palworld server found. Use Install below, or pick a folder above."}
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
                    onClick={() => use(d.path)}
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
          Downloads SteamCMD (first time only) and installs or updates the Palworld
          Dedicated Server (Steam app 2394010). The full server is several GB.
        </p>
        <div className="row" style={{ marginBottom: progress !== null ? 14 : 0 }}>
          <button className="btn primary" onClick={install} disabled={installing}>
            {installing ? "Working…" : installed ? "Update server" : "Install server"}
          </button>
          {installing && progress !== null && (
            <span style={{ color: "var(--text-dim)" }}>{progress.toFixed(1)}%</span>
          )}
        </div>
        {progress !== null && (
          <div className="progress">
            <span style={{ width: `${progress}%` }} />
          </div>
        )}
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
            className="btn danger"
            onClick={() => control("stop")}
            disabled={!running || busy}
          >
            Stop server
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
