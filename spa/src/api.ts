const BASE = "";

function token(): string {
  return localStorage.getItem("token") ?? "";
}

function authHeaders(): HeadersInit {
  return { Authorization: `Bearer ${token()}`, "Content-Type": "application/json" };
}

export interface Board { id: string; name: string; position: number }
export interface Note  { id: string; note_type: string; position: number }
export interface DeviceSummary { id: string; name: string; last_seen: string }
export type WsEvent = { event: string; [key: string]: unknown };

// ─── Boards ───────────────────────────────────────────────────────────────────

export async function fetchBoards(): Promise<Board[]> {
  const r = await fetch(`${BASE}/boards`, { headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function createBoard(name: string): Promise<Board> {
  const r = await fetch(`${BASE}/boards`, {
    method: "POST", headers: authHeaders(),
    body: JSON.stringify({ name, position: 0 }),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function renameBoard(id: string, name: string): Promise<void> {
  const r = await fetch(`${BASE}/boards/${id}`, {
    method: "PATCH", headers: authHeaders(),
    body: JSON.stringify({ name }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function deleteBoard(id: string): Promise<void> {
  const r = await fetch(`${BASE}/boards/${id}`, { method: "DELETE", headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
}

// ─── Notes ────────────────────────────────────────────────────────────────────

export async function fetchNotes(boardId: string): Promise<Note[]> {
  const r = await fetch(`${BASE}/notes?board_id=${boardId}`, { headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function fetchNoteContent(id: string): Promise<string> {
  const r = await fetch(`${BASE}/notes/${id}/blob`, { headers: authHeaders() });
  if (!r.ok) return "";
  return r.text();
}

export async function createNote(boardId: string, text: string): Promise<{ id: string }> {
  const r = await fetch(`${BASE}/notes`, {
    method: "POST", headers: authHeaders(),
    body: JSON.stringify({
      board_id: boardId, note_type: "text", color: null, position: 0,
      blob_key: crypto.randomUUID(),
      size: new TextEncoder().encode(text).length,
    }),
  });
  if (!r.ok) throw new Error(await r.text());
  const { id } = await r.json();
  await fetch(`${BASE}/notes/${id}/blob`, {
    method: "PUT",
    headers: { Authorization: `Bearer ${token()}`, "Content-Type": "text/plain" },
    body: text,
  });
  return { id };
}

export async function updateNoteContent(id: string, text: string): Promise<void> {
  const r = await fetch(`${BASE}/notes/${id}/blob`, {
    method: "PUT",
    headers: { Authorization: `Bearer ${token()}`, "Content-Type": "text/plain" },
    body: text,
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function deleteNote(id: string): Promise<void> {
  const r = await fetch(`${BASE}/notes/${id}`, { method: "DELETE", headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
}

// ─── Devices ──────────────────────────────────────────────────────────────────

export async function fetchDevices(): Promise<DeviceSummary[]> {
  const r = await fetch(`${BASE}/devices`, { headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function renameDevice(id: string, name: string): Promise<void> {
  const r = await fetch(`${BASE}/devices/${id}/rename`, {
    method: "POST", headers: authHeaders(),
    body: JSON.stringify({ name }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function deleteDevice(id: string): Promise<void> {
  const r = await fetch(`${BASE}/devices/${id}`, { method: "DELETE", headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
}

// ─── Link ─────────────────────────────────────────────────────────────────────

export async function initLink(): Promise<{ token: string; code: string; expires_at: string }> {
  const r = await fetch(`${BASE}/link/init`, { method: "POST" });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function getLinkStatus(linkToken: string): Promise<{ status: string; jwt?: string }> {
  const r = await fetch(`${BASE}/link/status/${linkToken}`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

// ─── Identity ─────────────────────────────────────────────────────────────────

export interface IdentityInfo { id: string; friendly_name: string }

export async function getIdentityMe(): Promise<IdentityInfo> {
  const r = await fetch(`${BASE}/identity/me`, { headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function updateIdentityName(friendly_name: string): Promise<IdentityInfo> {
  const r = await fetch(`${BASE}/identity/me`, {
    method: "PATCH", headers: authHeaders(),
    body: JSON.stringify({ friendly_name }),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function lookupIdentity(name: string): Promise<IdentityInfo | null> {
  const r = await fetch(`${BASE}/identity/lookup/${encodeURIComponent(name)}`, { headers: authHeaders() });
  if (r.status === 404) return null;
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

// ─── Shares ───────────────────────────────────────────────────────────────────

export interface ShareEntry { shared_with_id: string; shared_with_name: string | null; created_at: string }
export interface SharedNote  { note_id: string; note_type: string; board_id: string; owner_identity_id: string; owner_friendly_name: string | null }

export async function fetchShares(noteId: string): Promise<ShareEntry[]> {
  const r = await fetch(`${BASE}/notes/${noteId}/shares`, { headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function shareNote(noteId: string, target: string): Promise<void> {
  const r = await fetch(`${BASE}/notes/${noteId}/shares`, {
    method: "POST", headers: authHeaders(),
    body: JSON.stringify({ target }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function revokeShare(noteId: string, targetId: string): Promise<void> {
  const r = await fetch(`${BASE}/notes/${noteId}/shares/${targetId}`, {
    method: "DELETE", headers: authHeaders(),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function getSharedWithMe(): Promise<SharedNote[]> {
  const r = await fetch(`${BASE}/notes/shared`, { headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

// ─── WebSocket ────────────────────────────────────────────────────────────────

export function connectWs(onEvent: (e: WsEvent) => void): () => void {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  let ws: WebSocket;
  let delay = 1000;
  let stopped = false;

  function connect() {
    ws = new WebSocket(`${proto}://${location.host}/ws?token=${encodeURIComponent(token())}`);
    ws.onmessage = (e) => { try { onEvent(JSON.parse(e.data as string)); } catch {} };
    ws.onclose   = () => { if (!stopped) setTimeout(connect, Math.min((delay *= 2), 30000)); };
    ws.onerror   = () => ws.close();
  }
  connect();
  return () => { stopped = true; ws.close(); };
}

// ─── JWT helpers ──────────────────────────────────────────────────────────────

export function decodeJwt(t?: string): { sub: string; identity_id: string } | null {
  try {
    const raw = t ?? token();
    const payload = raw.split(".")[1];
    return JSON.parse(atob(payload.replace(/-/g, "+").replace(/_/g, "/")));
  } catch { return null; }
}
