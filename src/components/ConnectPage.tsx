import { useEffect, useState } from "react";
import { api, type NetworkInfo } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
}

export default function ConnectPage({ notify }: Props) {
  const [info, setInfo] = useState<NetworkInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  async function load() {
    setLoading(true);
    try {
      setInfo(await api.networkInfo());
    } catch (e) {
      notify(String(e), true);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  const connectAddr = info ? `${info.publicIp}:${info.port}` : "";

  function copy() {
    navigator.clipboard.writeText(connectAddr);
    notify("Connect address copied.");
  }

  async function forward(open: boolean) {
    setBusy(true);
    try {
      const msg = open ? await api.networkForward() : await api.networkUnforward();
      notify(msg);
      load();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Connectivity</h1>
          <p>How your friends connect — and the fix when they can't.</p>
        </div>
        <button className="btn" onClick={load} disabled={loading}>
          {loading ? "Checking…" : "Refresh"}
        </button>
      </div>

      <div className="card">
        <h2>Share this with your friends</h2>
        <div className="row">
          <span className="path" style={{ fontSize: 15 }}>
            {loading ? "…" : connectAddr}
          </span>
          <button className="btn primary" onClick={copy} disabled={!info || info.publicIp === "unavailable"}>
            Copy
          </button>
        </div>
        <p style={{ color: "var(--text-dim)", marginBottom: 0 }}>
          In Palworld: <strong>Join Multiplayer Server → type the address above</strong>.
        </p>
      </div>

      <div className="card">
        <h2>Status</h2>
        <table className="table">
          <tbody>
            <tr>
              <td>Public IP</td>
              <td>{info?.publicIp ?? "…"}</td>
            </tr>
            <tr>
              <td>This PC (LAN IP)</td>
              <td>{info?.localIp ?? "…"}</td>
            </tr>
            <tr>
              <td>Game port (UDP)</td>
              <td>{info?.port ?? "…"}</td>
            </tr>
            <tr>
              <td>Server listening on this PC</td>
              <td>
                <span className={`pill ${info?.portListening ? "ok" : "off"}`}>
                  <span className="dot" /> {info?.portListening ? "Yes" : "No (start the server)"}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div className="card">
        <h2>Can't your friends connect?</h2>
        <p style={{ color: "var(--text-dim)", marginTop: 0 }}>
          The usual cause is that your router isn't forwarding the game port. Try opening it
          automatically (works if your router has UPnP enabled):
        </p>
        <div className="row">
          <button className="btn primary" onClick={() => forward(true)} disabled={busy}>
            {busy ? "Working…" : "Open port automatically (UPnP)"}
          </button>
          <button className="btn" onClick={() => forward(false)} disabled={busy}>
            Close port
          </button>
        </div>
        <div className="note" style={{ marginTop: 16 }}>
          <strong>If UPnP fails</strong>, forward it manually in your router: forward{" "}
          <strong>UDP port {info?.port ?? 8211}</strong> to this PC's LAN IP{" "}
          <strong>{info?.localIp ?? "…"}</strong>. (Router admin is usually{" "}
          <code>192.168.1.1</code> or <code>192.168.0.1</code>.)
        </div>
      </div>
    </>
  );
}
