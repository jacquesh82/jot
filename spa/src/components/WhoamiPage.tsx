import { useEffect, useRef, useState } from "preact/hooks";
import { Link, Check, RefreshCw, X, Pencil, Save } from "lucide-react";
import { initLink, getLinkStatus, decodeJwt, getIdentityMe, updateIdentityName, type IdentityInfo } from "../api";
import { QrCode } from "./QrCode";

export function WhoamiPage() {
  const claims = decodeJwt();
  const [identity, setIdentity] = useState<IdentityInfo | null>(null);
  const [editingName, setEditingName] = useState(false);
  const [nameValue, setNameValue] = useState("");
  const [nameError, setNameError] = useState<string | null>(null);
  const [linkToken, setLinkToken] = useState<string | null>(null);
  const [linked, setLinked] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const nameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { loadIdentity(); }, []);
  useEffect(() => { if (editingName) nameInputRef.current?.focus(); }, [editingName]);

  async function loadIdentity() {
    try { setIdentity(await getIdentityMe()); }
    catch { /* identity may not exist yet */ }
  }

  async function saveName() {
    if (!nameValue.trim()) return;
    setNameError(null);
    try {
      const updated = await updateIdentityName(nameValue.trim());
      setIdentity(updated);
      setEditingName(false);
    } catch (e) {
      setNameError(String(e));
    }
  }

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
          if (s.status === "confirmed") { clearInterval(pollRef.current!); setLinked(true); }
        } catch {}
      }, 2000);
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
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

      {error && <div class="error-msg">{error}<button class="btn-icon" onClick={() => setError(null)}><X size={14} /></button></div>}

      {/* ── Identity info ── */}
      <div class="info-card">
        <div class="info-row">
          <span class="info-key">Identity</span>
          <span class="info-val">{claims?.identity_id ?? "—"}</span>
        </div>
        <div class="info-row">
          <span class="info-key">Device</span>
          <span class="info-val">{claims?.sub ?? "—"}</span>
        </div>
        <div class="info-row" style={{ alignItems: "center" }}>
          <span class="info-key">Name</span>
          {editingName ? (
            <div style={{ display: "flex", gap: "0.4rem", flex: 1, alignItems: "center" }}>
              <input
                ref={nameInputRef}
                type="text"
                value={nameValue}
                onInput={(e) => setNameValue((e.target as HTMLInputElement).value)}
                onKeyDown={(e) => { if (e.key === "Enter") saveName(); if (e.key === "Escape") setEditingName(false); }}
                placeholder="your-unique-name"
                style={{ flex: 1, padding: "0.2rem 0.4rem" }}
              />
              <button class="btn-primary btn-icon" onClick={saveName} title="Save"><Save size={13} /></button>
              <button class="btn-icon" onClick={() => setEditingName(false)} title="Cancel"><X size={13} /></button>
            </div>
          ) : (
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flex: 1 }}>
              <span class="info-val">{identity?.friendly_name ?? <em style={{ color: "var(--text-muted)" }}>not set</em>}</span>
              <button class="btn-icon" onClick={() => { setNameValue(identity?.friendly_name ?? ""); setEditingName(true); }} title="Edit name">
                <Pencil size={13} />
              </button>
            </div>
          )}
        </div>
        {nameError && <p style={{ color: "var(--danger)", fontSize: "0.8rem", marginTop: "0.25rem" }}>{nameError}</p>}
      </div>

      {/* ── Link new device ── */}
      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.5rem" }}>
        <button class="btn-primary" onClick={generateLink} disabled={loading}>
          {loading ? <RefreshCw size={14} style={{ animation: "spin 1s linear infinite" }} /> : <Link size={14} />}
          {linkToken ? "Regenerate link" : "Link a new device"}
        </button>
        {linkToken && !linked && <button onClick={cancelLink}><X size={14} /> Cancel</button>}
      </div>

      {linkToken && !linked && (
        <div class="qr-box">
          <QrCode text={cmd} />
          <div>
            <div class="qr-hint" style={{ marginBottom: "0.35rem" }}>Run in your terminal:</div>
            <div class="qr-cmd">{cmd}</div>
          </div>
          <div class="qr-hint">
            <RefreshCw size={11} style={{ display: "inline", animation: "spin 2s linear infinite" }} /> Waiting for confirmation…
          </div>
        </div>
      )}

      {linked && (
        <div class="qr-box">
          <div class="qr-success"><Check size={16} /> Device linked!</div>
          <button onClick={cancelLink}>Close</button>
        </div>
      )}

      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
