import { useEffect, useState } from "react";
import { api, onInstallLog, onInstallProgress, type GameInfo, type StatusInfo } from "../api";

interface Props {
  status: StatusInfo | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
  gameName: string;
  games: GameInfo[];
  activeGameId: string;
  activeProfileId?: string;
  profileCount: number;
  liveControl: "rest" | "rcon" | "none";
  onClose: () => void;
}

/** Guided first-run setup, shown when no server is installed yet. */
export default function FirstRunWizard({
  status,
  refresh,
  notify,
  gameName,
  games,
  activeGameId,
  activeProfileId,
  profileCount,
  liveControl,
  onClose,
}: Props) {
  const [step, setStep] = useState(0);
  const [working, setWorking] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [phase, setPhase] = useState("");
  const installed = status?.installed ?? false;
  // Only offer a game choice on a genuinely fresh setup (the single auto-created
  // profile, nothing installed yet) — a returning user with multiple profiles already
  // knows how to add one for another game from Settings.
  const showGameChoice = profileCount === 1 && !installed;

  useEffect(() => {
    const un = [
      onInstallProgress((p) => setProgress(p)),
      onInstallLog((l) => setPhase(l)),
    ];
    return () => un.forEach((p) => p.then((fn) => fn()));
  }, []);

  async function install() {
    setWorking(true);
    setProgress(0);
    setPhase("Preparing…");
    try {
      notify("Downloading the server (several GB) — this can take a few minutes.");
      await api.installServer();
      notify("Server installed.");
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setWorking(false);
      setProgress(null);
    }
  }

  async function detect() {
    setWorking(true);
    try {
      const found = await api.detectInstalls();
      if (found.length === 0) {
        notify("No existing server found on this PC.", true);
      } else {
        await api.addProfile(found[0].source, found[0].path, activeGameId);
        notify("Connected to your existing server.");
        refresh();
      }
    } catch (e) {
      notify(String(e), true);
    } finally {
      setWorking(false);
    }
  }

  async function chooseGame(gameId: string) {
    if (gameId === activeGameId || switching) return;
    setSwitching(true);
    try {
      const dir = await api.defaultInstallDir(gameId);
      await api.addProfile("Default", dir, gameId);
      // The pre-switch state was the sole pristine profile (that's the showGameChoice
      // gate) — safe to drop it now that the new one is active, so we don't leave an
      // empty never-installed profile behind every time someone picks a different game.
      if (profileCount === 1 && activeProfileId) {
        await api.deleteProfile(activeProfileId);
      }
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSwitching(false);
    }
  }

  async function enableRest() {
    setWorking(true);
    try {
      const res = await api.enableLiveControl();
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

  // Numbered steps auto-renumber based on which ones actually apply (e.g. no
  // live-control step for a game with none), so nothing goes "1 · ... 3 · ..." with a
  // gap.
  const numberedSteps = [
    {
      suffix: "Get a server",
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
          {progress !== null && (
            <div style={{ marginTop: 14 }}>
              <div style={{ fontSize: 12, color: "var(--text-dim)", marginBottom: 6 }}>
                {phase || "Preparing…"}
                {progress > 0 ? ` · ${progress.toFixed(0)}%` : ""}
              </div>
              <div className={`progress${progress > 0 ? "" : " indeterminate"}`}>
                <span style={progress > 0 ? { width: `${progress}%` } : undefined} />
              </div>
            </div>
          )}
        </>
      ),
    },
    ...(liveControl === "none"
      ? []
      : [
          {
            suffix: liveControl === "rcon" ? "Enable RCON" : "Enable the live dashboard",
            body: (
              <>
                <p>
                  {liveControl === "rcon"
                    ? "Turn on RCON so you get the live Dashboard (players, kick/ban, broadcast). This sets an admin password — restart the server afterward to apply."
                    : "Turn on the server's REST API so you get the live Dashboard (players, kick/ban, broadcast). This sets an admin password — restart the server afterward to apply."}
                </p>
                <button className="btn primary" onClick={enableRest} disabled={working}>
                  {working ? "Working…" : liveControl === "rcon" ? "Enable RCON" : "Enable REST API"}
                </button>
              </>
            ),
          },
        ]),
    {
      suffix: "You're set 🎉",
      body: (
        <p>
          Head to <strong>Server → Start</strong> to launch it. Use <strong>Configuration</strong>{" "}
          for the server name/password, <strong>Connect</strong> to share the join address, and{" "}
          <strong>Automation</strong> for backups &amp; restarts.
        </p>
      ),
    },
  ];

  const steps = [
    ...(showGameChoice
      ? [
          {
            title: "Choose your game",
            body: (
              <>
                <p>Which game are you setting up? You can add more later from Settings → Profiles.</p>
                <div className="row" style={{ flexWrap: "wrap", gap: 8 }}>
                  {games.map((g) => (
                    <button
                      key={g.id}
                      className={`btn ${g.id === activeGameId ? "primary" : ""}`}
                      onClick={() => chooseGame(g.id)}
                      disabled={switching}
                    >
                      {g.displayName}
                    </button>
                  ))}
                </div>
              </>
            ),
          },
        ]
      : []),
    {
      title: "Welcome 👋",
      body: (
        <p>
          Let's get your {gameName} dedicated server running in a few quick steps. You can skip
          this any time — everything's also available from the sidebar.
        </p>
      ),
    },
    ...numberedSteps.map((s, i) => ({ title: `${i + 1} · ${s.suffix}`, body: s.body })),
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
              <button className="btn" onClick={() => setStep(step - 1)} disabled={switching}>
                Back
              </button>
            )}
            <button
              className="btn primary"
              onClick={() => (last ? onClose() : setStep(step + 1))}
              disabled={switching}
            >
              {last ? "Finish" : "Next"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
