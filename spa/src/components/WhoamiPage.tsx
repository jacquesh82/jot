import { useEffect, useRef, useState } from "preact/hooks";
import { Link, Check, RefreshCw, X, Pencil, Save, Plus, Trash2, Copy, Shuffle, Download } from "lucide-react";
import { initLink, getLinkStatus, decodeJwt, getIdentityMe, updateIdentityName, generateRandomName, exportData, createInvite, listInvites, revokeInvite, type IdentityInfo, type InviteToken } from "../api";
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
  const [invites, setInvites] = useState<InviteToken[]>([]);
  const [inviteLabel, setInviteLabel] = useState("");
  const [inviteError, setInviteError] = useState<string | null>(null);
  const [copiedToken, setCopiedToken] = useState<string | null>(null);
  const [exportPassword, setExportPassword] = useState("");
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const nameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { loadIdentity(); loadInvites(); }, []);
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

  async function loadInvites() {
    try { setInvites(await listInvites()); } catch {}
  }

  async function handleCreateInvite(e: Event) {
    e.preventDefault();
    setInviteError(null);
    try {
      await createInvite(inviteLabel.trim());
      setInviteLabel("");
      await loadInvites();
    } catch (e) { setInviteError(String(e)); }
  }

  async function handleRevokeInvite(token: string) {
    try { await revokeInvite(token); await loadInvites(); } catch {}
  }

  function copyInviteUrl(token: string) {
    const url = `${location.origin}/#/register?invite=${token}`;
    navigator.clipboard.writeText(url).then(() => {
      setCopiedToken(token);
      setTimeout(() => setCopiedToken(null), 2000);
    });
  }

  async function handleExport(e: Event) {
    e.preventDefault();
    setExportError(null);
    setExporting(true);
    try {
      const data = await exportData();
      const json = JSON.stringify(data, null, 2);
      let blob: Blob;
      let filename: string;

      if (exportPassword.trim()) {
        const enc = new TextEncoder();
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const iv = crypto.getRandomValues(new Uint8Array(12));
        const keyMaterial = await crypto.subtle.importKey("raw", enc.encode(exportPassword), "PBKDF2", false, ["deriveKey"]);
        const key = await crypto.subtle.deriveKey(
          { name: "PBKDF2", salt, iterations: 200_000, hash: "SHA-256" },
          keyMaterial,
          { name: "AES-GCM", length: 256 },
          false,
          ["encrypt"],
        );
        const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, enc.encode(json)));
        // Format: magic(4) + salt(16) + iv(12) + ciphertext
        const magic = new Uint8Array([0x6a, 0x6f, 0x74, 0x65]); // "jote"
        const out = new Uint8Array(magic.length + salt.length + iv.length + ciphertext.length);
        out.set(magic, 0);
        out.set(salt, 4);
        out.set(iv, 20);
        out.set(ciphertext, 32);
        blob = new Blob([out], { type: "application/octet-stream" });
        filename = "jot-export.jote";
      } else {
        blob = new Blob([json], { type: "application/json" });
        filename = "jot-export.json";
      }

      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setExportError(String(e));
    } finally {
      setExporting(false);
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
              <button class="btn-primary" style={{ padding: "0.25rem 0.4rem" }} onClick={saveName} title="Save"><Save size={13} /></button>
              <button class="btn-icon" onClick={() => setEditingName(false)} title="Cancel"><X size={13} /></button>
            </div>
          ) : (
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flex: 1 }}>
              <span class="info-val">{identity?.friendly_name ?? <em style={{ color: "var(--text-muted)" }}>not set</em>}</span>
              <button class="btn-icon" onClick={() => { setNameValue(identity?.friendly_name ?? ""); setEditingName(true); }} title="Edit name">
                <Pencil size={13} />
              </button>
              <button class="btn-icon" onClick={() => { setNameValue(generateRandomName()); setEditingName(true); }} title="Generate random name">
                <Shuffle size={13} />
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

      {/* ── Invitations ── */}
      <div style={{ marginTop: "1.5rem" }}>
        <div class="page-title" style={{ marginBottom: "0.75rem" }}>
          <h2 style={{ fontSize: "1rem" }}>Invitations</h2>
        </div>

        {invites.filter(i => !i.revoked_at).length > 0 && (
          <ul class="item-list" style={{ marginBottom: "0.75rem" }}>
            {invites.filter(i => !i.revoked_at).map((inv) => (
              <li key={inv.token} class="item-row">
                <div class="item-row-header">
                  <div style={{ flex: 1, overflow: "hidden" }}>
                    <div style={{ fontFamily: "monospace", fontSize: "0.78rem", color: "var(--text-muted)" }}>
                      {inv.token.slice(0, 8)}…
                    </div>
                    {inv.label && (
                      <div style={{ fontSize: "0.78rem", color: "var(--text)" }}>{inv.label}</div>
                    )}
                  </div>
                  <div class="item-actions" style={{ opacity: 1, display: "flex", gap: "0.3rem" }}>
                    <button
                      class="btn-icon"
                      title={copiedToken === inv.token ? "Copied!" : "Copy invite URL"}
                      onClick={() => copyInviteUrl(inv.token)}
                    >
                      {copiedToken === inv.token ? <Check size={13} /> : <Copy size={13} />}
                    </button>
                    <button class="btn-icon btn-danger" title="Revoke" onClick={() => handleRevokeInvite(inv.token)}>
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}

        {invites.filter(i => !i.revoked_at).length === 0 && (
          <p class="empty-msg" style={{ padding: "0.5rem 0", textAlign: "left" }}>No active invitations.</p>
        )}

        <form style={{ display: "flex", gap: "0.4rem" }} onSubmit={handleCreateInvite}>
          <input
            type="text"
            placeholder="Label (optional)…"
            value={inviteLabel}
            onInput={(e) => setInviteLabel((e.target as HTMLInputElement).value)}
            style={{ flex: 1 }}
          />
          <button class="btn-primary" type="submit"><Plus size={13} /> New invite</button>
        </form>
        {inviteError && <p style={{ color: "var(--danger)", fontSize: "0.8rem", marginTop: "0.3rem" }}>{inviteError}</p>}
      </div>

      {/* ── Export ── */}
      <div style={{ marginTop: "1.5rem" }}>
        <div class="page-title" style={{ marginBottom: "0.75rem" }}>
          <h2 style={{ fontSize: "1rem" }}>Export data</h2>
        </div>
        <form style={{ display: "flex", gap: "0.4rem", alignItems: "center" }} onSubmit={handleExport}>
          <input
            type="password"
            placeholder="Encryption password (optional)…"
            value={exportPassword}
            onInput={(e) => setExportPassword((e.target as HTMLInputElement).value)}
            style={{ flex: 1 }}
          />
          <button class="btn-primary" type="submit" disabled={exporting}>
            {exporting
              ? <RefreshCw size={14} style={{ animation: "spin 1s linear infinite" }} />
              : <Download size={14} />}
            {exportPassword.trim() ? "Export encrypted" : "Export JSON"}
          </button>
        </form>
        {exportPassword.trim() && (
          <p style={{ fontSize: "0.78rem", color: "var(--text-muted)", marginTop: "0.3rem" }}>
            File saved as <code>.jote</code> — AES-256-GCM + PBKDF2 (200k rounds).
          </p>
        )}
        {exportError && <p style={{ color: "var(--danger)", fontSize: "0.8rem", marginTop: "0.3rem" }}>{exportError}</p>}
      </div>

      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
