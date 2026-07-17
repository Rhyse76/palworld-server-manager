import { useEffect, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type CurseForgeMod, type GameInfo, type ModInfo, type StatusInfo } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
  status: StatusInfo | null;
}

// Same ARK: SA behavior noted in ConfigPage.tsx: it rewrites GameUserSettings.ini
// (which ActiveMods lives in) from memory on shutdown, discarding edits made while
// the server was running.
const ARK_LIVE_EDIT_WARNING =
  "The server is running. ARK: Survival Ascended rewrites its config file from memory when it shuts down, which silently discards mod changes made here. Stop the server first, make your changes, then start it again.";

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export default function ModsPage({ notify, status }: Props) {
  const [gameInfo, setGameInfo] = useState<GameInfo | null>(null);

  useEffect(() => {
    api.gameInfo().then(setGameInfo).catch(() => {});
  }, []);

  if (gameInfo?.modsKind === "curseforge-ids") {
    return <CurseForgeIdMods notify={notify} running={!!status?.running} />;
  }
  return <LocalFileMods notify={notify} />;
}

function LocalFileMods({ notify }: { notify: (msg: string, error?: boolean) => void }) {
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

function CurseForgeIdMods({
  notify,
  running,
}: {
  notify: (msg: string, error?: boolean) => void;
  running: boolean;
}) {
  const [ids, setIds] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CurseForgeMod[]>([]);
  const [searching, setSearching] = useState(false);
  const [searched, setSearched] = useState(false);

  async function load() {
    try {
      setIds(await api.modIdsList());
    } catch (e) {
      notify(String(e), true);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function search() {
    if (!query.trim()) return;
    setSearching(true);
    try {
      setResults(await api.curseforgeSearch(query.trim()));
      setSearched(true);
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSearching(false);
    }
  }

  async function addFromSearch(mod: CurseForgeMod) {
    if (running) {
      notify(ARK_LIVE_EDIT_WARNING, true);
      return;
    }
    try {
      await api.modIdAdd(String(mod.id));
      notify(`Added ${mod.name}. Restart the server so it can download it.`);
      load();
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function add() {
    if (running) {
      notify(ARK_LIVE_EDIT_WARNING, true);
      return;
    }
    const id = input.trim();
    if (!/^\d+$/.test(id)) {
      notify("Mod id must be numeric — copy it from the mod's CurseForge project page.", true);
      return;
    }
    setBusy(true);
    try {
      await api.modIdAdd(id);
      setInput("");
      notify(`Added mod ${id}. Restart the server so it can download it.`);
      load();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: string) {
    if (running) {
      notify(ARK_LIVE_EDIT_WARNING, true);
      return;
    }
    const yes = await ask(`Remove mod ${id} from the active list? Any files ARK already downloaded for it are left in place.`, {
      title: "Remove mod",
      kind: "warning",
    });
    if (!yes) return;
    try {
      await api.modIdRemove(id);
      notify("Mod removed from the active list.");
      load();
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function deleteFiles(id: string) {
    if (running) {
      notify(ARK_LIVE_EDIT_WARNING, true);
      return;
    }
    const yes = await ask(
      `Delete mod ${id}? This removes it from the active list AND deletes any files ARK has downloaded for it. This cannot be undone.`,
      { title: "Delete mod files", kind: "warning" },
    );
    if (!yes) return;
    try {
      await api.modIdDeleteFiles(id);
      notify("Mod removed and its downloaded files deleted.");
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
          <p>
            CurseForge mod ids the server loads via -mods=. The server downloads and updates the
            mod content itself on next launch — this just manages which ids are active.
          </p>
        </div>
        <button className="btn" onClick={load}>
          Refresh
        </button>
      </div>

      {running && (
        <div className="card" style={{ borderColor: "var(--warn)" }}>
          <p style={{ margin: 0 }}>⚠️ {ARK_LIVE_EDIT_WARNING}</p>
        </div>
      )}

      <div className="card">
        <h2 style={{ marginTop: 0 }}>Search CurseForge</h2>
        <div className="row">
          <input
            type="text"
            placeholder="Search mods…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && search()}
          />
          <button className="btn primary" onClick={search} disabled={searching || !query.trim()}>
            {searching ? "Searching…" : "Search"}
          </button>
        </div>

        {searched && results.length === 0 && !searching && (
          <p style={{ color: "var(--text-dim)", marginTop: 12 }}>No mods found.</p>
        )}

        {results.length > 0 && (
          <table className="table" style={{ marginTop: 14 }}>
            <thead>
              <tr>
                <th>Mod</th>
                <th>Downloads</th>
                <th style={{ textAlign: "right" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {results.map((m) => {
                const active = ids.includes(String(m.id));
                return (
                  <tr key={m.id}>
                    <td>
                      <div style={{ fontWeight: 600 }}>
                        {m.websiteUrl ? (
                          <a
                            href="#"
                            onClick={(e) => {
                              e.preventDefault();
                              openUrl(m.websiteUrl!);
                            }}
                          >
                            {m.name}
                          </a>
                        ) : (
                          m.name
                        )}
                      </div>
                      <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{m.summary}</div>
                    </td>
                    <td>{m.downloadCount.toLocaleString()}</td>
                    <td style={{ textAlign: "right" }}>
                      <button
                        className="btn"
                        onClick={() => addFromSearch(m)}
                        disabled={active || running}
                        title={running ? ARK_LIVE_EDIT_WARNING : undefined}
                      >
                        {active ? "Added" : "Add"}
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <h2>Add by id</h2>
        <p style={{ color: "var(--text-dim)", marginTop: 0 }}>
          Or paste a mod's numeric CurseForge project id directly.
        </p>
        <div className="row">
          <input
            type="text"
            placeholder="e.g. 940975"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && add()}
          />
          <button
            className="btn primary"
            onClick={add}
            disabled={busy || !input.trim() || running}
            title={running ? ARK_LIVE_EDIT_WARNING : undefined}
          >
            {busy ? "Adding…" : "Add"}
          </button>
        </div>
      </div>

      <div className="card">
        <h2>Active mods ({ids.length})</h2>
        {ids.length === 0 ? (
          <p style={{ color: "var(--text-dim)", margin: 0 }}>No mods added yet.</p>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>CurseForge id</th>
                <th style={{ textAlign: "right" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {ids.map((id) => (
                <tr key={id}>
                  <td>{id}</td>
                  <td style={{ textAlign: "right" }}>
                    <button
                      className="btn"
                      onClick={() => remove(id)}
                      disabled={running}
                      title={running ? ARK_LIVE_EDIT_WARNING : undefined}
                      style={{ marginRight: 8 }}
                    >
                      Remove
                    </button>
                    <button
                      className="btn danger"
                      onClick={() => deleteFiles(id)}
                      disabled={running}
                      title={running ? ARK_LIVE_EDIT_WARNING : undefined}
                    >
                      Delete files
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="note">
        <strong>Remove</strong> just drops the mod from the active list — any files ARK already
        downloaded for it stay on disk, so re-adding it later won't re-download.{" "}
        <strong>Delete files</strong> does that and also clears its downloaded content to reclaim
        disk space. Restart the server after changes to apply them.
      </div>
    </>
  );
}
