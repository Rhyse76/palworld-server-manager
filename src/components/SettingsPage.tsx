import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type AppConfig, type Discord } from "../api";

interface Props {
  config: AppConfig | null;
  refresh: () => void;
  notify: (msg: string, error?: boolean) => void;
}

const SUPPORT_URL = "https://ko-fi.com/rhyse76";

const DISCORD_DEFAULT: Discord = {
  enabled: false,
  webhookUrl: "",
  notifyServer: true,
  notifyPlayers: true,
  notifyBackups: true,
};

export default function SettingsPage({ config, refresh, notify }: Props) {
  const hide = config?.hideServerConsole ?? false;
  const [discord, setDiscordState] = useState<Discord>(config?.discord ?? DISCORD_DEFAULT);
  const [savingDiscord, setSaving] = useState(false);

  useEffect(() => {
    if (config?.discord) setDiscordState(config.discord);
  }, [config?.discord]);

  async function toggleConsole() {
    try {
      await api.setHideConsole(!hide);
      notify(`Server console will ${!hide ? "be hidden" : "be shown"} on next start.`);
      refresh();
    } catch (e) {
      notify(String(e), true);
    }
  }

  function setD<K extends keyof Discord>(key: K, value: Discord[K]) {
    setDiscordState((d) => ({ ...d, [key]: value }));
  }

  async function saveDiscord() {
    setSaving(true);
    try {
      await api.setDiscord(discord);
      notify("Discord settings saved.");
      refresh();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setSaving(false);
    }
  }

  async function testDiscord() {
    try {
      await api.setDiscord(discord); // ensure the latest URL is saved before testing
      await api.discordTest();
      notify("Test message sent — check your Discord channel.");
    } catch (e) {
      notify(String(e), true);
    }
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Settings</h1>
          <p>App preferences, notifications, and info.</p>
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
        <div className="row spread" style={{ marginBottom: 10 }}>
          <div>
            <h2 style={{ margin: 0 }}>🔔 Discord notifications</h2>
            <p style={{ color: "var(--text-dim)", margin: "6px 0 0" }}>
              Post server events to a Discord channel via a webhook (Channel → Edit → Integrations
              → Webhooks → New Webhook → Copy URL).
            </p>
          </div>
          <div
            className={`toggle ${discord.enabled ? "on" : ""}`}
            role="switch"
            aria-checked={discord.enabled}
            onClick={() => setD("enabled", !discord.enabled)}
          />
        </div>

        <input
          className="search"
          type="text"
          placeholder="https://discord.com/api/webhooks/…"
          value={discord.webhookUrl}
          onChange={(e) => setD("webhookUrl", e.target.value)}
          style={{ marginBottom: 14 }}
        />

        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <EventToggle
            label="Server started / stopped / crashed"
            on={discord.notifyServer}
            onChange={(v) => setD("notifyServer", v)}
          />
          <EventToggle
            label="Player joined / left"
            on={discord.notifyPlayers}
            onChange={(v) => setD("notifyPlayers", v)}
          />
          <EventToggle
            label="Backup created"
            on={discord.notifyBackups}
            onChange={(v) => setD("notifyBackups", v)}
          />
        </div>

        <div className="row" style={{ marginTop: 16 }}>
          <button className="btn primary" onClick={saveDiscord} disabled={savingDiscord}>
            {savingDiscord ? "Saving…" : "Save"}
          </button>
          <button className="btn" onClick={testDiscord} disabled={!discord.webhookUrl.trim()}>
            Send test message
          </button>
        </div>
      </div>

      <div className="card">
        <h2>About</h2>
        <p style={{ margin: "0 0 6px" }}>
          <strong>Palworld Server Manager</strong> · v0.2.0
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

function EventToggle({
  label,
  on,
  onChange,
}: {
  label: string;
  on: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="field">
      <label>{label}</label>
      <div
        className={`toggle ${on ? "on" : ""}`}
        role="switch"
        aria-checked={on}
        onClick={() => onChange(!on)}
      />
    </div>
  );
}
