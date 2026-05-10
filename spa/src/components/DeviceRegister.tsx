import { useEffect, useRef, useState } from "preact/hooks";
import { BookOpen, RefreshCw, Link } from "lucide-react";
import { getLinkStatus } from "../api";
import { QrCode } from "./QrCode";

const BASE = "";

type Mode = "choice" | "loading" | "invite_required" | "registration_closed" | "link" | "done";

// Read invite token from URL hash: #/register?invite=<token>
function readHashInvite(): string | null {
  const h = location.hash.replace(/^#\/?register\??/, "");
  const m = h.match(/(?:^|&)invite=([^&]+)/);
  return m ? m[1] : null;
}

export function DeviceRegister() {
  const hashInvite = readHashInvite();
  // If URL carries an invite token, skip the choice screen and attempt register immediately
  const [mode, setMode] = useState<Mode>(hashInvite ? "loading" : "choice");
  const [inviteInput, setInviteInput] = useState("");
  const [inviteError, setInviteError] = useState<string | null>(null);
  const [linkToken, setLinkToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (hashInvite) attemptRegister(hashInvite);
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, []);

  async function attemptRegister(invite?: string) {
    setInviteError(null);
    try {
      const body: Record<string, string> = {};
      if (invite) body.invite_token = invite;

      const r = await fetch(`${BASE}/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (r.ok) {
        const { jwt } = await r.json();
        localStorage.setItem("token", jwt);
        location.hash = "#/";
        return;
      }

      const { error: reason } = await r.json().catch(() => ({ error: "unknown" }));

      if (r.status === 403) {
        if (reason === "invite_required") {
          setMode(invite ? "invite_required" : "invite_required");
          if (invite) setInviteError("Invite token invalide ou révoqué.");
        } else {
          setMode("registration_closed");
        }
      } else {
        setError(`Erreur: ${reason}`);
        setMode("invite_required");
      }
    } catch (e) {
      setError(String(e));
      setMode("invite_required");
    }
  }

  async function initLink() {
    setError(null);
    try {
      const r = await fetch(`${BASE}/link/init`, { method: "POST" });
      if (!r.ok) throw new Error(await r.text());
      const { token } = await r.json();
      setLinkToken(token);
      setMode("link");
      pollRef.current = setInterval(async () => {
        try {
          const s = await getLinkStatus(token);
          if (s.status === "confirmed" && s.jwt) {
            clearInterval(pollRef.current!);
            localStorage.setItem("token", s.jwt);
            location.hash = "#/";
          }
        } catch {}
      }, 2000);
    } catch (e) {
      setError(String(e));
    }
  }

  const cmd = linkToken ? `jot link ${linkToken}` : "";

  return (
    <div class="register-shell">
      <div class="register-card">
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1.25rem" }}>
          <BookOpen size={20} />
          <strong style={{ fontSize: "1.1rem" }}>jot</strong>
        </div>

        {mode === "choice" && (
          <>
            <h2 style={{ marginBottom: "0.5rem" }}>Welcome to jot</h2>
            <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", marginBottom: "1.25rem" }}>
              What would you like to do?
            </p>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem" }}>
              <button
                class="btn-primary"
                style={{ justifyContent: "center", padding: "0.6rem" }}
                onClick={() => { setMode("loading"); attemptRegister(); }}
              >
                Create a new account
              </button>
              <button
                style={{ justifyContent: "center", padding: "0.6rem" }}
                onClick={initLink}
              >
                <Link size={14} /> Link an existing account
              </button>
            </div>
          </>
        )}

        {mode === "loading" && (
          <p style={{ color: "var(--text-muted)" }}>
            <RefreshCw size={13} style={{ display: "inline", animation: "spin 1s linear infinite" }} /> Connecting…
          </p>
        )}

        {(mode === "invite_required" || mode === "registration_closed") && (
          <>
            <h2 style={{ marginBottom: "0.75rem" }}>Create an account</h2>

            {inviteError && (
              <p style={{ color: "var(--danger)", fontSize: "0.85rem", marginBottom: "0.5rem" }}>{inviteError}</p>
            )}
            {error && (
              <p style={{ color: "var(--danger)", fontSize: "0.85rem", marginBottom: "0.5rem" }}>{error}</p>
            )}

            {mode === "invite_required" && (
              <>
                <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", marginBottom: "0.75rem" }}>
                  Enter your invite token to register:
                </p>
                <div style={{ display: "flex", gap: "0.4rem", marginBottom: "0.75rem" }}>
                  <input
                    type="text"
                    placeholder="Invite token…"
                    value={inviteInput}
                    onInput={(e) => setInviteInput((e.target as HTMLInputElement).value)}
                    onKeyDown={(e) => { if (e.key === "Enter") attemptRegister(inviteInput.trim()); }}
                    style={{ flex: 1 }}
                  />
                  <button
                    class="btn-primary"
                    onClick={() => attemptRegister(inviteInput.trim())}
                    disabled={!inviteInput.trim()}
                  >
                    Join
                  </button>
                </div>
              </>
            )}

            {mode === "registration_closed" && (
              <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", marginBottom: "0.75rem" }}>
                Registration is closed on this server.
              </p>
            )}

            <button
              onClick={() => setMode("choice")}
              style={{ marginTop: "0.5rem", fontSize: "0.8rem" }}
            >
              ← Back
            </button>
          </>
        )}

        {mode === "link" && (
          <>
            <h2 style={{ marginBottom: "0.75rem" }}>Link this device</h2>
            {error ? (
              <p style={{ color: "var(--danger)" }}>{error}</p>
            ) : linkToken ? (
              <>
                <p style={{ marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--text-muted)" }}>
                  Run in your terminal:
                </p>
                <div class="register-cmd">{cmd}</div>
                <QrCode text={cmd} size={160} />
                <p class="register-hint" style={{ marginTop: "0.75rem" }}>
                  <RefreshCw size={11} style={{ display: "inline", animation: "spin 2s linear infinite" }} /> Waiting for confirmation…
                </p>
                <button
                  onClick={() => { clearInterval(pollRef.current!); setLinkToken(null); setMode("choice"); }}
                  style={{ marginTop: "0.75rem", fontSize: "0.8rem" }}
                >
                  ← Back
                </button>
              </>
            ) : (
              <p style={{ color: "var(--text-muted)" }}>Initialising…</p>
            )}
          </>
        )}
      </div>
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
