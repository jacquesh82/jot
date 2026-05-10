const BASE = "";

function token(): string {
  return localStorage.getItem("token") ?? "";
}

function authHeaders(): HeadersInit {
  return { Authorization: `Bearer ${token()}`, "Content-Type": "application/json" };
}

async function authedFetch(input: string, init: RequestInit = {}): Promise<Response> {
  const r = await fetch(input, {
    ...init,
    headers: { ...authHeaders(), ...(init.headers ?? {}) },
  });
  if (r.status === 401) {
    localStorage.removeItem("token");
    location.hash = "#/register";
  }
  return r;
}

export interface Board { id: string; name: string; position: number }
export interface Note  { id: string; note_type: string; position: number; shared?: boolean }
export interface DeviceSummary { id: string; name: string; last_seen: string }
export type WsEvent = { event: string; [key: string]: unknown };

// ─── Boards ───────────────────────────────────────────────────────────────────

export async function fetchBoards(): Promise<Board[]> {
  const r = await authedFetch(`${BASE}/boards`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function createBoard(name: string): Promise<Board> {
  const r = await authedFetch(`${BASE}/boards`, {
    method: "POST", body: JSON.stringify({ name, position: 0 }),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function renameBoard(id: string, name: string): Promise<void> {
  const r = await authedFetch(`${BASE}/boards/${id}`, {
    method: "PATCH", body: JSON.stringify({ name }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function deleteBoard(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/boards/${id}`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

// ─── Notes ────────────────────────────────────────────────────────────────────

export async function fetchNotes(boardId: string): Promise<Note[]> {
  const r = await authedFetch(`${BASE}/notes?board_id=${boardId}`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function fetchNoteContent(id: string): Promise<string> {
  const r = await authedFetch(`${BASE}/notes/${id}/blob`);
  if (!r.ok) return "";
  return r.text();
}

export async function createNote(boardId: string, text: string): Promise<{ id: string }> {
  const r = await authedFetch(`${BASE}/notes`, {
    method: "POST",
    body: JSON.stringify({
      board_id: boardId, note_type: "text", color: null, position: 0,
      blob_key: crypto.randomUUID(),
      size: new TextEncoder().encode(text).length,
    }),
  });
  if (!r.ok) throw new Error(await r.text());
  const { id } = await r.json();
  await authedFetch(`${BASE}/notes/${id}/blob`, {
    method: "PUT",
    headers: { "Content-Type": "text/plain" },
    body: text,
  });
  return { id };
}

export async function updateNoteContent(id: string, text: string): Promise<void> {
  const r = await authedFetch(`${BASE}/notes/${id}/blob`, {
    method: "PUT",
    headers: { "Content-Type": "text/plain" },
    body: text,
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function deleteNote(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/notes/${id}`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

// ─── Devices ──────────────────────────────────────────────────────────────────

export async function fetchDevices(): Promise<DeviceSummary[]> {
  const r = await authedFetch(`${BASE}/devices`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function renameDevice(id: string, name: string): Promise<void> {
  const r = await authedFetch(`${BASE}/devices/${id}/rename`, {
    method: "POST", body: JSON.stringify({ name }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function deleteDevice(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/devices/${id}`, { method: "DELETE" });
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
  const r = await authedFetch(`${BASE}/identity/me`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function updateIdentityName(friendly_name: string): Promise<IdentityInfo> {
  const r = await authedFetch(`${BASE}/identity/me`, {
    method: "PATCH", body: JSON.stringify({ friendly_name }),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function lookupIdentity(name: string): Promise<IdentityInfo | null> {
  const r = await authedFetch(`${BASE}/identity/lookup/${encodeURIComponent(name)}`);
  if (r.status === 404) return null;
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function getRecentContacts(): Promise<IdentityInfo[]> {
  const r = await authedFetch(`${BASE}/identity/contacts`);
  if (!r.ok) return [];
  return r.json();
}

const _ADJ = ["swift","bold","calm","dark","free","glad","keen","mild","neat","pure","rare","safe","tame","warm","wise","bright","crisp","deep","fair","gray","high","just","long","open","rich","slow","tall","true","vast","wild"];
const _NOUN = ["alder","birch","cedar","daisy","elder","fern","grove","hazel","iris","larch","maple","oak","pine","reed","rose","sage","stone","thorn","vale","wave","brook","cliff","creek","dune","gale","mist","moon","peak","rain","star"];

export async function exportData(): Promise<unknown> {
  const r = await authedFetch(`${BASE}/export`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export function generateRandomName(): string {
  const adj = _ADJ[Math.floor(Math.random() * _ADJ.length)];
  const noun = _NOUN[Math.floor(Math.random() * _NOUN.length)];
  const num = Math.floor(Math.random() * 900) + 100;
  return `${adj}-${noun}-${num}`;
}

// ─── Board shares ─────────────────────────────────────────────────────────────

export interface BoardShareEntry { shared_with_id: string; shared_with_name: string | null; created_at: string }
export interface SharedBoard { board_id: string; board_name: string; owner_identity_id: string; owner_friendly_name: string | null }

export async function fetchSharedBoards(): Promise<SharedBoard[]> {
  const r = await authedFetch(`${BASE}/boards/shared`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function fetchBoardShares(boardId: string): Promise<BoardShareEntry[]> {
  const r = await authedFetch(`${BASE}/boards/${boardId}/shares`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function shareBoardWith(boardId: string, target: string): Promise<void> {
  const r = await authedFetch(`${BASE}/boards/${boardId}/shares`, {
    method: "POST", body: JSON.stringify({ target }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function revokeBoardShare(boardId: string, targetId: string): Promise<void> {
  const r = await authedFetch(`${BASE}/boards/${boardId}/shares/${targetId}`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

// ─── Invites ──────────────────────────────────────────────────────────────────

export interface InviteToken { token: string; label: string; created_at: string; revoked_at: string | null }

export async function createInvite(label: string): Promise<InviteToken> {
  const r = await authedFetch(`${BASE}/invites`, {
    method: "POST", body: JSON.stringify({ label }),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function listInvites(): Promise<InviteToken[]> {
  const r = await authedFetch(`${BASE}/invites`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function revokeInvite(token: string): Promise<void> {
  const r = await authedFetch(`${BASE}/invites/${token}`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

// ─── Shares ───────────────────────────────────────────────────────────────────

export interface ShareEntry { shared_with_id: string; shared_with_name: string | null; created_at: string }
export interface SharedNote  { note_id: string; note_type: string; board_id: string; owner_identity_id: string; owner_friendly_name: string | null }

export async function fetchShares(noteId: string): Promise<ShareEntry[]> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/shares`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function shareNote(noteId: string, target: string): Promise<void> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/shares`, {
    method: "POST", body: JSON.stringify({ target }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function revokeShare(noteId: string, targetId: string): Promise<void> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/shares/${targetId}`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

export async function getSharedWithMe(): Promise<SharedNote[]> {
  const r = await authedFetch(`${BASE}/notes/shared`);
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
