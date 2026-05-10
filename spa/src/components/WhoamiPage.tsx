import { useState, useRef } from "preact/hooks";
import { Link, Check, RefreshCw, X } from "lucide-react";
import { initLink, getLinkStatus, decodeJwt } from "../api";
import { QrCode } from "./QrCode";

export function WhoamiPage() {
  const claims = decodeJwt();
  const [linkToken, setLinkToken] = useState<string | null>(null);
  const [linked, setLinked] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  async function generateLink() {
    setError(null);
    setLinked(false);
    setLoading(true);
    if (pollRef.current) clearInterval(pollRef.current);
    try {
      const { token } = await initLink();
      setLinkToken(token);
      pollRef.current = setInterval(async () => {
        try {
          const s = await getLinkStatus(token);
          if (s.status === "confirmed") {
            clearInterval(pollRef.current!);
            setLinked(true);
          }
        } catch {}
      }, 2000);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  function cancelLink() {
    if (pollRef.current) clearInterval(pollRef.current);
    setLinkToken(null);
    setLinked(false);
  }

  const cmd = linkToken ? `jot link ${linkToken}` : "";

  return (
    <div>
      <div class="page-title"><h2>Profile</h2></div>

      {error && (
        <div class="error-msg">
          {error}<button class="btn-icon" onClick={() => setError(null)}><X size={14} /></button>
        </div>
      )}

      <div class="info-card">
        <div class="info-row">
          <span class="info-key">Identity</span>
          <span class="info-val">{claims?.identity_id ?? "—"}</span>
        </div>
        <div class="info-row">
          <span class="info-key">Device</span>
          <span class="info-val">{claims?.sub ?? "—"}</span>
        </div>
      </div>

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.5rem" }}>
        <button class="btn-primary" onClick={generateLink} disabled={loading}>
          {loading ? <RefreshCw size={14} style={{ animation: "spin 1s linear infinite" }} /> : <Link size={14} />}
          {linkToken ? "Regenerate link" : "Link a new device"}
        </button>
        {linkToken && !linked && (
          <button onClick={cancelLink}><X size={14} /> Cancel</button>
        )}
      </div>

      {linkToken && !linked && (
        <div class="qr-box">
          <QrCode text={cmd} />
          <div>
            <div class="qr-hint" style={{ marginBottom: "0.35rem" }}>Run in your terminal:</div>
            <div class="qr-cmd">{cmd}</div>
          </div>
          <div class="qr-hint">Waiting for confirmation… <RefreshCw size={11} style={{ display: "inline", animation: "spin 2s linear infinite" }} /></div>
        </div>
      )}

      {linked && (
        <div class="qr-box">
          <div class="qr-success"><Check size={16} /> Device linked successfully!</div>
          <button onClick={cancelLink}>Close</button>
        </div>
      )}

      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
