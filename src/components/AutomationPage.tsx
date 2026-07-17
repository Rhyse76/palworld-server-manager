import { useEffect, useState } from "react";
import {
  api,
  onActivityLog,
  type AppConfig,
  type Announcement,
  type Automation,
  type UpdateStatus,
} from "../api";

interface Props {
  config: AppConfig | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
  gameName: string;
}

const DEFAULTS: Automation = {
  autoRestartEnabled: false,
  restartIntervalHours: 6,
  autoBackupEnabled: false,
  backupIntervalHours: 2,
  keepBackups: 10,
  autoRestartOnCrash: true,
  smartRestart: false,
  autoUpdateEnabled: false,
  autoUpdateIntervalHours: 6,
};

export default function AutomationPage({ config, refresh, notify, gameName }: Props) {
  const activeProfile = config?.profiles.find((p) => p.id === config.activeProfile) ?? null;
  const [form, setForm] = useState<Automation>(activeProfile?.automation ?? DEFAULTS);
  const [formDirty, setFormDirty] = useState(false);
  const [announcements, setAnnouncements] = useState<Announcement[]>(config?.announcements ?? []);
  const [announcementsDirty, setAnnouncementsDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [activity, setActivity] = useState<string[]>([]);
  const [update, setUpdate] = useState<UpdateStatus | null>(null);
  const [checking, setChecking] = useState(false);

  // Profile actually changed (or first load): always reload, discarding any unsaved edits
  // for the profile we just left.
  useEffect(() => {
    setForm(activeProfile?.automation ?? DEFAULTS);
    setFormDirty(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeProfile?.id]);

  // Background poll refreshed the same profile's data: only take it if the user isn't
  // mid-edit, so a 4s poll tick can't clobber an in-progress toggle change.
  useEffect(() => {
    if (activeProfile?.automation && !formDirty) setForm(activeProfile.automation);
  }, [activeProfile?.automation, formDirty]);

  useEffect(() => {
    if (config?.announcements && !announcementsDirty) setAnnouncements(config.announcements);
  }, [config?.announcements, announcementsDirty]);

  useEffect(() => {
    const un = onActivityLog((line) => setActivity((a) => [...a.slice(-200), line]));
    return () => {
      un.then((fn) => fn());
    };
  }, []);

  function set<K extends keyof Automation>(key: K, value: Automation[K]) {
    setForm((f) => ({ ...f, [key]: value }));
    setFormDirty(true);
  }

  async function save() {
    setSaving(true);
    try {
      await api.setAutomation(form);
      await api.setAnnouncements(announcements);
      setFormDirty(false);
      setAnnouncementsDirty(false);
      notify("Automation settings saved.");
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSaving(false);
    }
  }

  async function checkNow() {
    setChecking(true);
    try {
      setUpdate(await api.checkUpdate());
    } catch (e) {
      notify(String(e), true);
    } finally {
      setChecking(false);
    }
  }

  function addAnnouncement() {
    setAnnouncements((a) => [
      ...a,
      { id: crypto.randomUUID(), message: "", intervalMinutes: 30, enabled: true },
    ]);
    setAnnouncementsDirty(true);
  }
  function updateAnnouncement(id: string, patch: Partial<Announcement>) {
    setAnnouncements((a) => a.map((x) => (x.id === id ? { ...x, ...patch } : x)));
    setAnnouncementsDirty(true);
  }
  function removeAnnouncement(id: string) {
    setAnnouncements((a) => a.filter((x) => x.id !== id));
    setAnnouncementsDirty(true);
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Automation</h1>
          <p>Scheduled tasks for the active server. Runs while the app is open.</p>
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
          Watchdog: if the server dies unexpectedly while it should be running, automatically
          start it again. Checked every 60 seconds.
        </p>
      </div>

      <div className="card">
        <div className="row spread">
          <h2 style={{ margin: 0 }}>Scheduled restarts</h2>
          <Toggle on={form.autoRestartEnabled} onChange={(v) => set("autoRestartEnabled", v)} />
        </div>
        <p style={{ color: "var(--text-dim)" }}>
          Warns players, saves, and gracefully restarts on an interval.
        </p>
        <div className="row" style={{ gap: 24 }}>
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
          <div className="row">
            <Toggle on={form.smartRestart} onChange={(v) => set("smartRestart", v)} />
            <label title="Wait for the server to be empty before restarting">
              Only when empty
            </label>
          </div>
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
        <div className="row spread">
          <h2 style={{ margin: 0 }}>Server auto-update</h2>
          <Toggle on={form.autoUpdateEnabled} onChange={(v) => set("autoUpdateEnabled", v)} />
        </div>
        <p style={{ color: "var(--text-dim)" }}>
          Keeps the <strong>{gameName} dedicated server (the game)</strong> — not this app — on the
          latest build. When a new version drops it warns players, saves, updates, and restarts.
        </p>
        <div className="row" style={{ gap: 16 }}>
          <div className="row">
            <label>Check every</label>
            <input
              type="number"
              className="num"
              min={1}
              step={1}
              value={form.autoUpdateIntervalHours}
              disabled={!form.autoUpdateEnabled}
              onChange={(e) => set("autoUpdateIntervalHours", Number(e.target.value))}
            />
            <label>hours</label>
          </div>
          <button className="btn" onClick={checkNow} disabled={checking}>
            {checking ? "Checking…" : "Check now"}
          </button>
          {update &&
            (!update.checked ? (
              <span style={{ color: "var(--text-dim)" }}>Couldn't check (server installed?).</span>
            ) : update.updateAvailable ? (
              <span className="pill bad">
                <span className="dot" /> Update available ({update.installedBuild} →{" "}
                {update.latestBuild})
              </span>
            ) : (
              <span className="pill ok">
                <span className="dot" /> Up to date
              </span>
            ))}
        </div>
      </div>

      <div className="card">
        <div className="row spread" style={{ marginBottom: 10 }}>
          <div>
            <h2 style={{ margin: 0 }}>Scheduled announcements</h2>
            <p style={{ color: "var(--text-dim)", margin: "6px 0 0" }}>
              Recurring in-game broadcasts (rules, Discord link, restart reminders). Sent only
              while the server is running.
            </p>
          </div>
          <button className="btn" onClick={addAnnouncement}>
            Add
          </button>
        </div>
        {announcements.length === 0 ? (
          <p style={{ color: "var(--text-dim)", margin: 0 }}>No announcements yet.</p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {announcements.map((a) => (
              <div className="field" key={a.id} style={{ gap: 10 }}>
                <Toggle on={a.enabled} onChange={(v) => updateAnnouncement(a.id, { enabled: v })} />
                <input
                  className="search"
                  style={{ flex: 1 }}
                  placeholder="Message to broadcast…"
                  value={a.message}
                  onChange={(e) => updateAnnouncement(a.id, { message: e.target.value })}
                />
                <input
                  type="number"
                  className="num"
                  min={1}
                  step={1}
                  value={a.intervalMinutes}
                  onChange={(e) =>
                    updateAnnouncement(a.id, { intervalMinutes: Number(e.target.value) })
                  }
                />
                <label>min</label>
                <button className="btn danger" onClick={() => removeAnnouncement(a.id)}>
                  ✕
                </button>
              </div>
            ))}
          </div>
        )}
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
