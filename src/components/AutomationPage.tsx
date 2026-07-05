import { useEffect, useState } from "react";
import { api, onActivityLog, type AppConfig, type Automation } from "../api";

interface Props {
  config: AppConfig | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
}

const DEFAULTS: Automation = {
  autoRestartEnabled: false,
  restartIntervalHours: 6,
  autoBackupEnabled: false,
  backupIntervalHours: 2,
  keepBackups: 10,
  autoRestartOnCrash: true,
};

export default function AutomationPage({ config, refresh, notify }: Props) {
  const [form, setForm] = useState<Automation>(config?.automation ?? DEFAULTS);
  const [saving, setSaving] = useState(false);
  const [activity, setActivity] = useState<string[]>([]);

  // Sync when config first loads.
  useEffect(() => {
    if (config?.automation) setForm(config.automation);
  }, [config?.automation]);

  useEffect(() => {
    const un = onActivityLog((line) => setActivity((a) => [...a.slice(-200), line]));
    return () => {
      un.then((fn) => fn());
    };
  }, []);

  function set<K extends keyof Automation>(key: K, value: Automation[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  async function save() {
    setSaving(true);
    try {
      await api.setAutomation(form);
      notify("Automation settings saved.");
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Automation</h1>
          <p>Scheduled restarts and backups for the active server. Runs while the app is open.</p>
        </div>
        <button className="btn primary" onClick={save} disabled={saving}>
          {saving ? "Saving…" : "Save settings"}
        </button>
      </div>

      <div className="card">
        <div className="row spread">
          <h2 style={{ margin: 0 }}>Crash recovery</h2>
          <Toggle on={form.autoRestartOnCrash} onChange={(v) => set("autoRestartOnCrash", v)} />
        </div>
        <p style={{ color: "var(--text-dim)", marginBottom: 0 }}>
          Watchdog: if the server dies unexpectedly (e.g. a game crash) while it should be
          running, automatically start it again. Checked every 60 seconds.
        </p>
      </div>

      <div className="card">
        <div className="row spread">
          <h2 style={{ margin: 0 }}>Scheduled restarts</h2>
          <Toggle on={form.autoRestartEnabled} onChange={(v) => set("autoRestartEnabled", v)} />
        </div>
        <p style={{ color: "var(--text-dim)" }}>
          Warns players, saves, and gracefully restarts on an interval. Only fires while the
          server is running.
        </p>
        <div className="row">
          <label>Every</label>
          <input
            type="number"
            className="num"
            min={0.5}
            step={0.5}
            value={form.restartIntervalHours}
            disabled={!form.autoRestartEnabled}
            onChange={(e) => set("restartIntervalHours", Number(e.target.value))}
          />
          <label>hours</label>
        </div>
      </div>

      <div className="card">
        <div className="row spread">
          <h2 style={{ margin: 0 }}>Scheduled backups</h2>
          <Toggle on={form.autoBackupEnabled} onChange={(v) => set("autoBackupEnabled", v)} />
        </div>
        <p style={{ color: "var(--text-dim)" }}>
          Snapshots the world save on an interval and prunes old backups.
        </p>
        <div className="row" style={{ gap: 24 }}>
          <div className="row">
            <label>Every</label>
            <input
              type="number"
              className="num"
              min={0.25}
              step={0.25}
              value={form.backupIntervalHours}
              disabled={!form.autoBackupEnabled}
              onChange={(e) => set("backupIntervalHours", Number(e.target.value))}
            />
            <label>hours</label>
          </div>
          <div className="row">
            <label>Keep last</label>
            <input
              type="number"
              className="num"
              min={0}
              step={1}
              value={form.keepBackups}
              disabled={!form.autoBackupEnabled}
              onChange={(e) => set("keepBackups", Number(e.target.value))}
            />
            <label>backups</label>
          </div>
        </div>
      </div>

      <div className="card">
        <h2>Automation activity</h2>
        <div className="console" style={{ height: 160 }}>
          {activity.length === 0 ? (
            <span style={{ color: "var(--text-dim)" }}>
              Scheduled actions will be logged here while the app runs…
            </span>
          ) : (
            activity.join("\n")
          )}
        </div>
      </div>
    </>
  );
}

function Toggle({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <div
      className={`toggle ${on ? "on" : ""}`}
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
    />
  );
}
