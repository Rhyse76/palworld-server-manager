import { useEffect, useMemo, useState } from "react";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, type ConfigField } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
}

export default function ConfigPage({ notify }: Props) {
  const [fields, setFields] = useState<ConfigField[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [saving, setSaving] = useState(false);

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
      defaultPath: "palworld-config.json",
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
      title: "Import config preset or PalWorldSettings.ini",
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
    return q ? fields.filter((f) => f.key.toLowerCase().includes(q)) : fields;
  }, [fields, filter]);

  if (loaded && error) {
    return (
      <>
        <div className="page-head">
          <div>
            <h1>Configuration</h1>
            <p>Edit every setting in PalWorldSettings.ini.</p>
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
            {fields.length} settings from PalWorldSettings.ini · changes apply on next
            server restart.
          </p>
        </div>
        <button className="btn primary" onClick={save} disabled={saving || !fields.length}>
          {saving ? "Saving…" : "Save changes"}
        </button>
      </div>

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

      <div className="fields">
        {shown.map((f) => (
          <Field key={f.key} field={f} onChange={(v) => update(f.key, v)} />
        ))}
      </div>
      {loaded && shown.length === 0 && <div className="empty">No settings match “{filter}”.</div>}
    </>
  );
}

function Field({ field, onChange }: { field: ConfigField; onChange: (v: string) => void }) {
  return (
    <div className="field">
      <label title={field.key}>
        {field.key}
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
      ) : (
        <input type="text" value={field.value} onChange={(e) => onChange(e.target.value)} />
      )}
    </div>
  );
}
