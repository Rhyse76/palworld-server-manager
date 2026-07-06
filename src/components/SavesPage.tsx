import { useState } from "react";
import { api, type SaveInfo } from "../api";

interface Props {
  notify: (msg: string, error?: boolean) => void;
}

function mb(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export default function SavesPage({ notify }: Props) {
  const [info, setInfo] = useState<SaveInfo | null>(null);
  const [busy, setBusy] = useState(false);

  async function inspect() {
    setBusy(true);
    try {
      setInfo(await api.inspectSave());
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
          <h1>Save tools <span className="pill">experimental</span></h1>
          <p>Read your world save. Player/pal/inventory viewing and editing build on this.</p>
        </div>
        <button className="btn primary" onClick={inspect} disabled={busy}>
          {busy ? "Reading…" : "Read world save"}
        </button>
      </div>

      <div className="card">
        {!info ? (
          <p style={{ color: "var(--text-dim)", margin: 0 }}>
            Click “Read world save” to decompress and inspect <code>Level.sav</code>.
          </p>
        ) : (
          <table className="table">
            <tbody>
              <tr>
                <td>File</td>
                <td style={{ wordBreak: "break-all" }}>{info.path}</td>
              </tr>
              <tr>
                <td>On disk (compressed)</td>
                <td>{mb(info.compressedSize)}</td>
              </tr>
              <tr>
                <td>Decompressed</td>
                <td>{mb(info.decompressedSize)}</td>
              </tr>
              <tr>
                <td>Compression</td>
                <td>{info.saveType === 0x32 ? "double zlib" : "single zlib"}</td>
              </tr>
              <tr>
                <td>Valid GVAS save data</td>
                <td>
                  <span className={`pill ${info.isGvas ? "ok" : "bad"}`}>
                    <span className="dot" /> {info.isGvas ? "Yes" : "No"}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <h2>Coming next</h2>
        <p style={{ color: "var(--text-dim)", margin: 0 }}>
          This confirms the save can be read. Next: parse the GVAS to list players, pals, guilds,
          and inventories (read-only), then careful editing (give items, levels) with a forced
          backup and the server stopped first.
        </p>
      </div>
    </>
  );
}
