import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type AppConfig } from "../api";

interface Props {
  config: AppConfig | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
}

// TODO: set this to your real donation page (Ko-fi / GitHub Sponsors / Buy Me a Coffee).
const SUPPORT_URL = "https://ko-fi.com/";

export default function SettingsPage({ config, refresh, notify }: Props) {
  const hide = config?.hideServerConsole ?? false;

  async function toggleConsole() {
    try {
      await api.setHideConsole(!hide);
      notify(`Server console will ${!hide ? "be hidden" : "be shown"} on next start.`);
      refresh();
    } catch (e) {
      notify(String(e), true);
    }
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Settings</h1>
          <p>App preferences and info.</p>
        </div>
      </div>

      <div className="card">
        <div className="row spread">
          <div>
            <h2 style={{ margin: 0 }}>Hide server console window</h2>
            <p style={{ color: "var(--text-dim)", margin: "6px 0 0" }}>
              When on, the server runs without its black console window. Takes effect the next
              time the server starts.
            </p>
          </div>
          <div
            className={`toggle ${hide ? "on" : ""}`}
            role="switch"
            aria-checked={hide}
            onClick={toggleConsole}
          />
        </div>
      </div>

      <div className="card">
        <h2>About</h2>
        <p style={{ margin: "0 0 6px" }}>
          <strong>Palworld Server Manager</strong> · v0.1.0
        </p>
        <p style={{ color: "var(--text-dim)", marginTop: 0 }}>
          An unofficial, community-made tool for running a Palworld dedicated server. Not
          affiliated with or endorsed by Pocketpair, Inc. “Palworld” is a trademark of its
          respective owner.
        </p>
        <button className="btn primary" onClick={() => openUrl(SUPPORT_URL)}>
          ♥ Support development
        </button>
      </div>
    </>
  );
}
