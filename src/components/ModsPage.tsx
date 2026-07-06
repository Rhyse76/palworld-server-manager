import { useEffect, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { api, type ModInfo } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export default function ModsPage({ notify }: Props) {
  const [mods, setMods] = useState<ModInfo[]>([]);
  const [busy, setBusy] = useState(false);

  async function load() {
    try {
      setMods(await api.modsList());
    } catch (e) {
      notify(String(e), true);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function toggle(m: ModInfo) {
    try {
      await api.modSetEnabled(m.name, !m.enabled);
      load();
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function install() {
    const picked = await open({
      title: "Choose a .pak mod",
      filters: [{ name: "Pak mod", extensions: ["pak"] }],
    });
    if (typeof picked !== "string") return;
    setBusy(true);
    try {
      const name = await api.modInstall(picked);
      notify(`Installed ${name}. Restart the server to apply.`);
      load();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy(false);
    }
  }

  async function remove(m: ModInfo) {
    const yes = await ask(`Remove ${m.name}? This deletes the file.`, {
      title: "Remove mod",
      kind: "warning",
    });
    if (!yes) return;
    try {
      await api.modRemove(m.name);
      notify("Mod removed.");
      load();
    } catch (e) {
      notify(String(e), true);
    }
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Mods</h1>
          <p>Manage .pak mods in Pal/Content/Paks/~mods. Changes apply on server restart.</p>
        </div>
        <div className="row">
          <button className="btn" onClick={load}>
            Refresh
          </button>
          <button className="btn primary" onClick={install} disabled={busy}>
            {busy ? "Installing…" : "Install .pak…"}
          </button>
        </div>
      </div>

      <div className="card">
        <h2>Installed mods ({mods.length})</h2>
        {mods.length === 0 ? (
          <p style={{ color: "var(--text-dim)", margin: 0 }}>
            No mods yet. Click “Install .pak…” to add one.
          </p>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Mod</th>
                <th>Size</th>
                <th>Enabled</th>
                <th style={{ textAlign: "right" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {mods.map((m) => (
                <tr key={m.name}>
                  <td>{m.name}</td>
                  <td>{formatSize(m.sizeBytes)}</td>
                  <td>
                    <div
                      className={`toggle ${m.enabled ? "on" : ""}`}
                      role="switch"
                      aria-checked={m.enabled}
                      onClick={() => toggle(m)}
                    />
                  </td>
                  <td style={{ textAlign: "right" }}>
                    <button className="btn danger" onClick={() => remove(m)}>
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="note">
        This manages the mod <em>files</em> — enable/disable/install/remove. Whether a mod works
        on a dedicated server is up to the mod itself. Always keep a backup before adding mods.
      </div>
    </>
  );
}
