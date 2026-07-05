import { useEffect, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { api, type BackupInfo } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function formatDate(unixSecs: number): string {
  if (!unixSecs) return "—";
  return new Date(unixSecs * 1000).toLocaleString();
}

export default function BackupsPage({ notify }: Props) {
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [busy, setBusy] = useState(false);

  async function load() {
    try {
      setBackups(await api.backupList());
    } catch (e) {
      notify(String(e), true);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function create() {
    setBusy(true);
    try {
      const name = await api.backupCreate();
      notify(`Backup created: ${name}`);
      load();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy(false);
    }
  }

  async function restore(name: string) {
    const yes = await ask(
      `Restore ${name}? This overwrites current save files. The server must be stopped.`,
      { title: "Confirm restore", kind: "warning" },
    );
    if (!yes) return;
    try {
      await api.backupRestore(name);
      notify("Backup restored.");
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function remove(name: string) {
    const yes = await ask(`Delete ${name}? This cannot be undone.`, {
      title: "Confirm delete",
      kind: "warning",
    });
    if (!yes) return;
    try {
      await api.backupDelete(name);
      notify("Backup deleted.");
      load();
    } catch (e) {
      notify(String(e), true);
    }
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Backups</h1>
          <p>Snapshot and restore your world save folder (SaveGames).</p>
        </div>
        <div className="row">
          <button className="btn" onClick={() => api.backupOpenFolder()}>
            Open folder
          </button>
          <button className="btn primary" onClick={create} disabled={busy}>
            {busy ? "Backing up…" : "Create backup"}
          </button>
        </div>
      </div>

      <div className="card">
        <h2>Saved backups ({backups.length})</h2>
        {backups.length === 0 ? (
          <p style={{ color: "var(--text-dim)", margin: 0 }}>
            No backups yet. Click “Create backup” to snapshot the current world.
          </p>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Size</th>
                <th>Created</th>
                <th style={{ textAlign: "right" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {backups.map((b) => (
                <tr key={b.name}>
                  <td>{b.name}</td>
                  <td>{formatSize(b.sizeBytes)}</td>
                  <td>{formatDate(b.modified)}</td>
                  <td style={{ textAlign: "right" }}>
                    <button className="btn" onClick={() => restore(b.name)}>
                      Restore
                    </button>
                    <button
                      className="btn danger"
                      style={{ marginLeft: 8 }}
                      onClick={() => remove(b.name)}
                    >
                      Delete
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
