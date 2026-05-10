import { useEffect, useRef, useState } from "preact/hooks";

const BASE = "/api";

export function DeviceRegister() {
  const [linkToken, setLinkToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    initLink();
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, []);

  async function initLink() {
    try {
      const r = await fetch(`${BASE}/link/init`, { method: "POST" });
      if (!r.ok) throw new Error(await r.text());
      const { token } = await r.json();
      setLinkToken(token);
      startPolling(token);
    } catch (e) {
      setError(String(e));
    }
  }

  function startPolling(token: string) {
    pollRef.current = setInterval(async () => {
      try {
        const r = await fetch(`${BASE}/link/status/${token}`);
        if (!r.ok) return;
        const data = await r.json();
        if (data.status === "confirmed" && data.jwt) {
          clearInterval(pollRef.current!);
          localStorage.setItem("token", data.jwt);
          location.hash = "#/";
        }
      } catch {
        // ignore poll errors
      }
    }, 2000);
  }

  if (error) return <div class="error">Error: {error}</div>;

  return (
    <div class="register">
      <h2>Link this device</h2>
      {linkToken ? (
        <>
          <p>Run this command in your terminal to link:</p>
          <pre>jot link {linkToken}</pre>
          <p class="hint">Waiting for confirmation…</p>
        </>
      ) : (
        <p>Initialising…</p>
      )}
    </div>
  );
}
