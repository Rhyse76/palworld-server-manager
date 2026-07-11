import { useState } from "react";
import { api, type StatusInfo } from "../api";

interface Props {
  status: StatusInfo | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
  gameName: string;
  onClose: () => void;
}

/** Guided first-run setup, shown when no server is installed yet. */
export default function FirstRunWizard({ status, refresh, notify, gameName, onClose }: Props) {
  const [step, setStep] = useState(0);
  const [working, setWorking] = useState(false);
  const installed = status?.installed ?? false;

  async function install() {
    setWorking(true);
    try {
      notify("Downloading the server (several GB) — this can take a few minutes.");
      await api.installServer();
      notify("Server installed.");
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setWorking(false);
    }
  }

  async function detect() {
    setWorking(true);
    try {
      const found = await api.detectInstalls();
      if (found.length === 0) {
        notify("No existing server found on this PC.", true);
      } else {
        await api.addProfile(found[0].source, found[0].path, "palworld");
        notify("Connected to your existing server.");
        refresh();
      }
    } catch (e) {
      notify(String(e), true);
    } finally {
      setWorking(false);
    }
  }

  async function enableRest() {
    setWorking(true);
    try {
      const res = await api.enableRestApi();
      notify(
        res.generatedPassword
          ? `Live dashboard enabled. Admin password: ${res.adminPassword} (restart the server to apply).`
          : "Live dashboard enabled. Restart the server to apply.",
      );
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setWorking(false);
    }
  }

  const steps = [
    {
      title: "Welcome 👋",
      body: (
        <p>
          Let's get your {gameName} dedicated server running in a few quick steps. You can skip
          this any time — everything's also available from the sidebar.
        </p>
      ),
    },
    {
      title: "1 · Get a server",
      body: (
        <>
          <p>
            {installed
              ? "✅ A server is already set up — you're good to go."
              : "Install a fresh server, or connect one you already have on this PC."}
          </p>
          {!installed && (
            <div className="row">
              <button className="btn primary" onClick={install} disabled={working}>
                {working ? "Working…" : "Install server"}
              </button>
              <button className="btn" onClick={detect} disabled={working}>
                Detect existing
              </button>
            </div>
          )}
        </>
      ),
    },
    {
      title: "2 · Enable the live dashboard",
      body: (
        <>
          <p>
            Turn on the server's REST API so you get the live Dashboard (players, kick/ban,
            broadcast). This sets an admin password — restart the server afterward to apply.
          </p>
          <button className="btn primary" onClick={enableRest} disabled={working}>
            {working ? "Working…" : "Enable REST API"}
          </button>
        </>
      ),
    },
    {
      title: "3 · You're set 🎉",
      body: (
        <p>
          Head to <strong>Server → Start</strong> to launch it. Use <strong>Configuration</strong>{" "}
          for the server name/password, <strong>Connect</strong> to share the join address, and{" "}
          <strong>Automation</strong> for backups &amp; restarts.
        </p>
      ),
    },
  ];

  const s = steps[step];
  const last = step === steps.length - 1;

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.7)",
        zIndex: 99999,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: 24,
      }}
    >
      <div className="card" style={{ maxWidth: 520, width: "100%", margin: 0 }}>
        <div className="row spread" style={{ marginBottom: 12 }}>
          <h2 style={{ margin: 0 }}>{s.title}</h2>
          <button className="btn" onClick={onClose} style={{ padding: "4px 12px" }}>
            Skip
          </button>
        </div>
        <div style={{ color: "var(--text-dim)", lineHeight: 1.6, minHeight: 90 }}>{s.body}</div>
        <div className="row spread" style={{ marginTop: 18 }}>
          <span style={{ color: "var(--text-dim)", fontSize: 12 }}>
            Step {step + 1} of {steps.length}
          </span>
          <div className="row" style={{ gap: 8 }}>
            {step > 0 && (
              <button className="btn" onClick={() => setStep(step - 1)}>
                Back
              </button>
            )}
            <button
              className="btn primary"
              onClick={() => (last ? onClose() : setStep(step + 1))}
            >
              {last ? "Finish" : "Next"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
