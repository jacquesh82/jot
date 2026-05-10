const BASE = "";

function token(): string {
  return localStorage.getItem("token") ?? "";
}

function authHeaders(): HeadersInit {
  return { Authorization: `Bearer ${token()}`, "Content-Type": "application/json" };
}

export interface Board {
  id: string;
  name: string;
  position: number;
}

export interface Note {
  id: string;
  note_type: string;
  position: number;
}

export type WsEvent = { event: string; [key: string]: unknown };

export async function fetchBoards(): Promise<Board[]> {
  const r = await fetch(`${BASE}/boards`, { headers: authHeaders() });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function createBoard(name: string): Promise<Board> {
  const r = await fetch(`${BASE}/boards`, {
    method: "POST",
    headers: authHeaders(),
    body: JSON.stringify({ name, position: 0 }),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function renameBoard(id: string, name: string): Promise<void> {
  const r = await fetch(`${BASE}/boards/${id}`, {
    method: "PATCH",
    headers: authHeaders(),
    body: JSON.stringify({ name }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function deleteBoard(id: string): Promise<void> {
  const r = await fetch(`${BASE}/boards/${id}`, {
    method: "DELETE",
    headers: authHeaders(),
  });
  if (!r.ok) throw new Error(await r.text());
}

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
    method: "POST",
    headers: authHeaders(),
    body: JSON.stringify({
      board_id: boardId,
      note_type: "text",
      color: null,
      position: 0,
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

export function connectWs(onEvent: (e: WsEvent) => void): () => void {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  let ws: WebSocket;
  let delay = 1000;
  let stopped = false;

  function connect() {
    ws = new WebSocket(`${proto}://${location.host}/ws?token=${encodeURIComponent(token())}`);
    ws.onmessage = (e) => {
      try {
        onEvent(JSON.parse(e.data as string));
      } catch {
        // ignore malformed frames
      }
    };
    ws.onclose = () => {
      if (!stopped) setTimeout(connect, Math.min((delay *= 2), 30000));
    };
    ws.onerror = () => ws.close();
  }

  connect();
  return () => {
    stopped = true;
    ws.close();
  };
}
