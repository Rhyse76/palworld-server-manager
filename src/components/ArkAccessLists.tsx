import { useEffect, useState } from "react";
import { api } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
}

// ARK: Survival Ascended's exclusive-join and admin lists are plain text files, one
// EOS/Steam ID per line — verified against real-world reports rather than guessed
// (see the punch-list entry this closes). Both editors below share one shape.
export default function ArkAccessLists({ notify }: Props) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16, marginTop: 4 }}>
      <IdListCard
        title="Exclusive join allow list"
        description="Only these IDs can join while Exclusive Join (above) is on — requires a server restart to take effect. Note: ARK's own enforcement of this has been reported unreliable at times; that's the game's behavior, not something this app controls."
        load={api.arkExclusiveJoinList}
        save={api.arkSetExclusiveJoinList}
        notify={notify}
      />
      <IdListCard
        title="Admin list"
        description="IDs granted admin console access. Saving here also points AdminListURL (above) at this file, unless it's already set to something else."
        load={api.arkAdminsList}
        save={api.arkSetAdminsList}
        notify={notify}
      />
    </div>
  );
}

function IdListCard({
  title,
  description,
  load,
  save,
  notify,
}: {
  title: string;
  description: string;
  load: () => Promise<string[]>;
  save: (ids: string[]) => Promise<void>;
  notify: (msg: string, error?: boolean) => void;
}) {
  const [text, setText] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    load()
      .then((ids) => setText(ids.join("\n")))
      .catch((e) => setError(String(e)))
      .finally(() => setLoaded(true));
    // Only load once on mount — this card owns its own save, so it doesn't need to
    // resync from anywhere else.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function doSave() {
    const ids = Array.from(
      new Set(
        text
          .split("\n")
          .map((s) => s.trim())
          .filter(Boolean),
      ),
    );
    setSaving(true);
    try {
      await save(ids);
      setText(ids.join("\n"));
      notify(`${title} saved (${ids.length} id${ids.length === 1 ? "" : "s"}).`);
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="card">
      <h2 style={{ marginTop: 0 }}>{title}</h2>
      <p style={{ color: "var(--text-dim)" }}>{description}</p>
      {error ? (
        <p style={{ color: "var(--text-dim)", marginBottom: 0 }}>{error}</p>
      ) : (
        <>
          <textarea
            className="search"
            style={{ width: "100%", minHeight: 120, fontFamily: "monospace", fontSize: 12 }}
            placeholder="One ID per line…"
            value={text}
            onChange={(e) => setText(e.target.value)}
            disabled={!loaded}
          />
          <div className="row" style={{ marginTop: 10 }}>
            <button className="btn primary" onClick={doSave} disabled={saving || !loaded}>
              {saving ? "Saving…" : "Save list"}
            </button>
          </div>
        </>
      )}
    </div>
  );
}
