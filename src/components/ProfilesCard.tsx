import { useEffect, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { api, type AppConfig, type GameInfo } from "../api";

interface Props {
  config: AppConfig | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
}

export default function ProfilesCard({ config, refresh, notify }: Props) {
  const [editing, setEditing] = useState<string | null>(null);
  const [nameEdit, setNameEdit] = useState("");
  const [games, setGames] = useState<GameInfo[]>([]);
  const [newGame, setNewGame] = useState("palworld");

  useEffect(() => {
    api.gamesList().then(setGames).catch(() => {});
  }, []);

  const gameName = (id: string) => games.find((g) => g.id === id)?.displayName ?? id;

  if (!config) return null;
  const active = config.activeProfile;

  async function add() {
    const picked = await open({ directory: true, title: `Choose a folder for the ${gameName(newGame)} server` });
    if (typeof picked !== "string") return;
    const name = picked.split(/[\\/]/).filter(Boolean).pop() || "Server";
    await api.addProfile(name, picked, newGame);
    notify(`${gameName(newGame)} profile added and activated.`);
    refresh();
  }

  async function switchTo(id: string) {
    await api.setActiveProfile(id);
    refresh();
  }

  async function del(id: string) {
    const yes = await ask("Remove this profile? (Server files on disk are NOT deleted.)", {
      title: "Remove profile",
      kind: "warning",
    });
    if (!yes) return;
    try {
      await api.deleteProfile(id);
      notify("Profile removed.");
      refresh();
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function saveRename(id: string) {
    if (nameEdit.trim()) await api.renameProfile(id, nameEdit.trim());
    setEditing(null);
    refresh();
  }

  return (
    <div className="card">
      <div className="row spread" style={{ marginBottom: 12 }}>
        <h2 style={{ margin: 0 }}>Server profiles</h2>
        <div className="row" style={{ gap: 8 }}>
          {games.length > 1 && (
            <select
              className="search"
              style={{ maxWidth: 190 }}
              value={newGame}
              onChange={(e) => setNewGame(e.target.value)}
              title="Game for the new profile"
            >
              {games.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.displayName}
                </option>
              ))}
            </select>
          )}
          <button className="btn" onClick={add}>
            Add profile…
          </button>
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {config.profiles.map((p) => (
          <div className="field" key={p.id}>
            <div style={{ overflow: "hidden", flex: 1 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
                <input
                  type="radio"
                  checked={p.id === active}
                  onChange={() => switchTo(p.id)}
                />
                {editing === p.id ? (
                  <input
                    className="search"
                    style={{ maxWidth: 220 }}
                    value={nameEdit}
                    autoFocus
                    onChange={(e) => setNameEdit(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && saveRename(p.id)}
                  />
                ) : (
                  <span style={{ fontWeight: 600 }}>
                    {p.name}
                    <span
                      className="pill"
                      style={{ marginLeft: 8, padding: "2px 8px", fontWeight: 400 }}
                    >
                      {gameName(p.game)}
                    </span>
                    {p.id === active && (
                      <span className="pill ok" style={{ marginLeft: 8, padding: "2px 8px" }}>
                        active
                      </span>
                    )}
                  </span>
                )}
              </label>
              <div className="path" style={{ marginTop: 6, border: "none", padding: 0 }}>
                {p.installDir}
              </div>
            </div>
            <div className="row" style={{ gap: 6 }}>
              {editing === p.id ? (
                <button className="btn" onClick={() => saveRename(p.id)}>
                  Save
                </button>
              ) : (
                <button
                  className="btn"
                  onClick={() => {
                    setEditing(p.id);
                    setNameEdit(p.name);
                  }}
                >
                  Rename
                </button>
              )}
              <button
                className="btn danger"
                onClick={() => del(p.id)}
                disabled={config.profiles.length <= 1}
              >
                Remove
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
