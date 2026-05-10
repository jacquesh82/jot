import { useEffect, useRef, useState } from "preact/hooks";
import { BookOpen, RefreshCw } from "lucide-react";
import { getLinkStatus } from "../api";
import { QrCode } from "./QrCode";

const BASE = "";

export function DeviceRegister() {
  const [linkToken, setLinkToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    initLink();
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, []);

  async function initLink() {
    try {
      const r = await fetch(`${BASE}/link/init`, { method: "POST" });
      if (!r.ok) throw new Error(await r.text());
      const { token } = await r.json();
      setLinkToken(token);
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
    <div class="register-shell" data-theme={document.documentElement.getAttribute("data-theme") ?? "light"}>
      <div class="register-card">
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1.25rem" }}>
          <BookOpen size={20} />
          <strong style={{ fontSize: "1.1rem" }}>jot</strong>
        </div>

        <h2>Link this device</h2>

        {error ? (
          <p style={{ color: "var(--danger)", marginTop: "0.75rem" }}>{error}</p>
        ) : linkToken ? (
          <>
            <p style={{ marginTop: "0.75rem", marginBottom: "0.5rem", fontSize: "0.85rem", color: "var(--text-muted)" }}>
              Run in your terminal:
            </p>
            <div class="register-cmd">{cmd}</div>
            <QrCode text={cmd} size={160} />
            <p class="register-hint" style={{ marginTop: "0.75rem" }}>
              <RefreshCw size={11} style={{ display: "inline", animation: "spin 2s linear infinite" }} /> Waiting for confirmation…
            </p>
          </>
        ) : (
          <p style={{ color: "var(--text-muted)", marginTop: "0.75rem" }}>Initialising…</p>
        )}
      </div>
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
