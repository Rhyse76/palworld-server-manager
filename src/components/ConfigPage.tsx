import { useEffect, useMemo, useState } from "react";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, type ConfigField, type StatusInfo } from "../api";
import ArkAccessLists from "./ArkAccessLists";

interface Props {
  notify: (msg: string, error?: boolean) => void;
  status: StatusInfo | null;
}

// ARK: SA keeps its loaded settings in memory and rewrites GameUserSettings.ini/
// Game.ini with that in-memory snapshot when the server shuts down — silently
// discarding any config edit made while it was running (community-confirmed
// behavior, not an app bug). Safe order: stop the server, edit, then start again.
const ARK_LIVE_EDIT_WARNING =
  "The server is running. ARK: Survival Ascended rewrites its config file from memory when it shuts down, which silently discards any changes made here. Stop the server first, make your changes, then start it again.";

export default function ConfigPage({ notify, status }: Props) {
  const [fields, setFields] = useState<ConfigField[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [saving, setSaving] = useState(false);
  const [configFile, setConfigFile] = useState("the config file");
  const [gameId, setGameId] = useState("server");
  const arkLiveEditRisk = gameId === "ark-sa" && !!status?.running;

  useEffect(() => {
    api.gameInfo().then((g) => {
      setConfigFile(g.configFile);
      setGameId(g.id);
    }).catch(() => {});
  }, []);

  async function load() {
    try {
      const f = await api.readConfig();
      setFields(f);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoaded(true);
    }
  }

  useEffect(() => {
    load();
  }, []);

  function update(key: string, value: string) {
    setFields((fs) => fs.map((f) => (f.key === key ? { ...f, value } : f)));
  }

  async function save() {
    setSaving(true);
    try {
      await api.writeConfig(fields);
      notify("Config saved. Restart the server to apply.");
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSaving(false);
    }
  }

  async function exportPreset() {
    if (!fields.length) return;
    const dest = await saveDialog({
      title: "Export config preset",
      defaultPath: `${gameId}-config.json`,
      filters: [{ name: "Config preset", extensions: ["json"] }],
    });
    if (!dest) return;
    try {
      await api.exportConfig(fields, dest);
      notify("Config preset exported.");
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function importPreset() {
    const src = await open({
      title: `Import config preset or ${configFile}`,
      filters: [
        { name: "Config", extensions: ["json", "ini"] },
        { name: "All files", extensions: ["*"] },
      ],
    });
    if (typeof src !== "string") return;
    try {
      const imported = await api.importConfig(src);
      setFields(imported);
      setError(null);
      notify(`Imported ${imported.length} settings. Review, then Save to apply.`);
    } catch (e) {
      notify(String(e), true);
    }
  }

  const shown = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return fields;
    return fields.filter(
      (f) => f.key.toLowerCase().includes(q) || (f.label ?? "").toLowerCase().includes(q),
    );
  }, [fields, filter]);

  // Group the visible fields by `group`, preserving first-seen order.
  const groupOrder = useMemo(() => {
    const map = new Map<string, ConfigField[]>();
    for (const f of shown) {
      const g = f.group ?? "";
      if (!map.has(g)) map.set(g, []);
      map.get(g)!.push(f);
    }
    return Array.from(map.entries());
  }, [shown]);

  // Stable tab list = distinct groups across ALL fields (not affected by search).
  const allGroups = useMemo(() => {
    const seen = new Set<string>();
    const order: string[] = [];
    for (const f of fields) {
      const g = f.group ?? "";
      if (!seen.has(g)) {
        seen.add(g);
        order.push(g);
      }
    }
    return order;
  }, [fields]);

  const searching = filter.trim().length > 0;
  // Tab the config when there are real groups and we're not searching; otherwise
  // fall back to a flat, section-headed list (search results / single-group games).
  const tabbed = allGroups.length > 1 && !searching;
  const [tab, setTab] = useState("");
  useEffect(() => {
    if (!allGroups.includes(tab)) setTab(allGroups[0] ?? "");
  }, [allGroups, tab]);

  if (loaded && error) {
    return (
      <>
        <div className="page-head">
          <div>
            <h1>Configuration</h1>
            <p>Edit every setting in {configFile}.</p>
          </div>
        </div>
        <div className="card">
          <div className="empty">
            {error}
            <div className="row" style={{ marginTop: 16, justifyContent: "center" }}>
              <button className="btn" onClick={load}>
                Retry
              </button>
              <button className="btn" onClick={importPreset}>
                Import preset…
              </button>
            </div>
          </div>
        </div>
      </>
    );
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Configuration</h1>
          <p>
            {fields.length} settings from {configFile} · changes apply on next
            server restart.
          </p>
        </div>
        <button
          className="btn primary"
          onClick={save}
          disabled={saving || !fields.length || arkLiveEditRisk}
          title={arkLiveEditRisk ? ARK_LIVE_EDIT_WARNING : undefined}
        >
          {saving ? "Saving…" : "Save changes"}
        </button>
      </div>

      {arkLiveEditRisk && (
        <div className="card" style={{ borderColor: "var(--warn)" }}>
          <p style={{ margin: 0 }}>⚠️ {ARK_LIVE_EDIT_WARNING}</p>
        </div>
      )}

      <div className="toolbar">
        <input
          className="search"
          placeholder="Search settings… (e.g. exp, capture, pvp)"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <button className="btn" onClick={load}>
          Reload
        </button>
        <button className="btn" onClick={importPreset}>
          Import…
        </button>
        <button className="btn" onClick={exportPreset} disabled={!fields.length}>
          Export…
        </button>
      </div>

      {tabbed ? (
        <>
          <div
            style={{
              display: "flex",
              gap: 4,
              overflowX: "auto",
              borderBottom: "1px solid var(--border)",
              margin: "4px 0 16px",
            }}
          >
            {allGroups.map((g) => (
              <button
                key={g}
                onClick={() => setTab(g)}
                style={{
                  background: "transparent",
                  border: "none",
                  borderBottom: `2px solid ${g === tab ? "var(--accent)" : "transparent"}`,
                  color: g === tab ? "var(--text)" : "var(--text-dim)",
                  padding: "8px 14px",
                  fontSize: 13,
                  fontWeight: g === tab ? 600 : 400,
                  cursor: "pointer",
                  whiteSpace: "nowrap",
                }}
              >
                {g || "General"}
              </button>
            ))}
          </div>
          <div className="fields">
            {shown
              .filter((f) => (f.group ?? "") === tab)
              .map((f) => (
                <Field key={f.key} field={f} onChange={(v) => update(f.key, v)} />
              ))}
          </div>
          {tab === "Access & Whitelist" && gameId === "ark-sa" && (
            <ArkAccessLists notify={notify} />
          )}
        </>
      ) : (
        groupOrder.map(([group, fs]) => (
          <div key={group || "_ungrouped"}>
            {group && (
              <h3
                style={{
                  margin: "20px 0 8px",
                  fontSize: 14,
                  color: "var(--accent)",
                  textTransform: "uppercase",
                  letterSpacing: 0.5,
                }}
              >
                {group}
              </h3>
            )}
            <div className="fields">
              {fs.map((f) => (
                <Field key={f.key} field={f} onChange={(v) => update(f.key, v)} />
              ))}
            </div>
          </div>
        ))
      )}
      {loaded && shown.length === 0 && <div className="empty">No settings match “{filter}”.</div>}
    </>
  );
}

function Field({ field, onChange }: { field: ConfigField; onChange: (v: string) => void }) {
  return (
    <div className="field">
      <label title={field.key}>
        {field.label || field.key}
        <span className="kind" style={{ marginLeft: 8 }}>
          {field.kind}
        </span>
      </label>
      {field.kind === "bool" ? (
        <div
          className={`toggle ${field.value === "true" ? "on" : ""}`}
          onClick={() => onChange(field.value === "true" ? "false" : "true")}
          role="switch"
          aria-checked={field.value === "true"}
        />
      ) : field.kind === "int" || field.kind === "float" ? (
        <input
          type="number"
          step={field.kind === "float" ? "any" : "1"}
          value={field.value}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : field.kind === "enum" && field.options && field.options.length > 0 ? (
        <select value={field.value} onChange={(e) => onChange(e.target.value)}>
          {!field.options.includes(field.value) && (
            <option value={field.value}>{field.value} (unrecognized)</option>
          )}
          {field.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      ) : (
        <input type="text" value={field.value} onChange={(e) => onChange(e.target.value)} />
      )}
    </div>
  );
}
