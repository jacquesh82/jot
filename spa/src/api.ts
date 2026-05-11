import {
  encryptNoteOwner, decryptNoteOwner, decryptNoteAsMember, decryptNoteAsRecipient,
  encryptBekForRecipient, encryptDekForRecipient,
  getPublicKeyHex,
} from "./crypto";

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
export interface Note  { id: string; note_type: string; position: number; shared?: boolean; snippet?: string; encrypted: boolean; schema_version?: number }
export interface NoteMeta { id: string; board_id: string; note_type: string; blob_key: string; schema_version?: number }
export interface DeviceSummary { id: string; name: string; last_seen: string }
export type WsEvent = { event: string; [key: string]: unknown };

export async function registerPubkey(): Promise<void> {
  await authedFetch(`${BASE}/identity/me/pubkey`, {
    method: "PUT",
    body: JSON.stringify({ public_key_x25519: await getPublicKeyHex() }),
  });
}

export async function deleteAccount(): Promise<void> {
  const r = await authedFetch(`${BASE}/identity/me`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

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

export async function fetchNoteMeta(id: string): Promise<NoteMeta> {
  const r = await authedFetch(`${BASE}/notes/${id}`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function fetchNoteContent(id: string): Promise<{ content: string; encrypted: boolean }> {
  const [metaResp, blobResp] = await Promise.all([
    authedFetch(`${BASE}/notes/${id}`),
    authedFetch(`${BASE}/notes/${id}/blob`),
  ]);
  if (!blobResp.ok) return { content: "", encrypted: false };
  const blob = new Uint8Array(await blobResp.arrayBuffer());
  if (!metaResp.ok) {
    try {
      return { content: new TextDecoder("utf-8", { fatal: true }).decode(blob), encrypted: false };
    } catch {
      return { content: "[chiffré — métadonnées manquantes]", encrypted: true };
    }
  }
  const { board_id } = await metaResp.json() as NoteMeta;

  // 1. Try owner path: derive BEK → DEK locally.
  try {
    return { content: await decryptNoteOwner(blob, board_id, id), encrypted: true };
  } catch { /* not the owner or wrong key — fall through */ }

  // 2. Try board-member path: fetch encrypted BEK from the API.
  try {
    const bekResp = await authedFetch(`${BASE}/boards/${board_id}/key`);
    if (bekResp.ok) {
      const { encrypted_bek, owner_pubkey_x25519 } = await bekResp.json() as {
        encrypted_bek: string;
        owner_pubkey_x25519?: string;
      };
      if (owner_pubkey_x25519) {
        return { content: await decryptNoteAsMember(blob, encrypted_bek, owner_pubkey_x25519, id), encrypted: true };
      }
    }
  } catch { /* fall through */ }

  // 3. Fall back to individual note-level share (note_shares table).
  try {
    const dekResp = await authedFetch(`${BASE}/notes/${id}/dek`);
    if (dekResp.ok) {
      const { encrypted_dek, owner_pubkey_x25519 } = await dekResp.json() as {
        encrypted_dek: string;
        owner_pubkey_x25519?: string;
      };
      if (owner_pubkey_x25519) {
        return { content: await decryptNoteAsRecipient(blob, encrypted_dek, owner_pubkey_x25519), encrypted: true };
      }
    }
  } catch { /* fall through */ }

  return { content: "[chiffré — clé manquante]", encrypted: true };
}

export async function encryptExistingNote(id: string, plaintext: string): Promise<void> {
  const meta = await fetchNoteMeta(id);
  const ciphertext = await encryptNoteOwner(plaintext, meta.board_id, id);
  const snippet = plaintext.split("\n").find(l => l.trim()) ?? plaintext.slice(0, 80);
  const [blobResp, patchResp] = await Promise.all([
    authedFetch(`${BASE}/notes/${id}/blob`, {
      method: "PUT",
      headers: { "Content-Type": "application/octet-stream" },
      body: ciphertext,
    }),
    authedFetch(`${BASE}/notes/${id}`, {
      method: "PATCH",
      body: JSON.stringify({ snippet: snippet.slice(0, 80), size: ciphertext.length }),
    }),
  ]);
  if (!blobResp.ok) throw new Error(`Impossible de chiffrer (${blobResp.status})`);
  if (!patchResp.ok) throw new Error(`Impossible de mettre à jour (${patchResp.status})`);
}

export async function createNote(boardId: string, text: string): Promise<{ id: string }> {
  // Create note record first to obtain the note_id (required for deterministic DEK).
  const snippet = text.split("\n").find(l => l.trim()) ?? text.slice(0, 80);
  const r = await authedFetch(`${BASE}/notes`, {
    method: "POST",
    body: JSON.stringify({
      board_id: boardId, note_type: "text", color: null, position: 0,
      blob_key: crypto.randomUUID(),
      size: 0,
      snippet: snippet.slice(0, 80),
    }),
  });
  if (!r.ok) throw new Error(await r.text());
  const { id } = await r.json() as { id: string };

  // Derive DEK now that we have both board_id and note_id.
  const ciphertext = await encryptNoteOwner(text, boardId, id);

  await Promise.all([
    authedFetch(`${BASE}/notes/${id}/blob`, {
      method: "PUT",
      headers: { "Content-Type": "application/octet-stream" },
      body: ciphertext,
    }),
    authedFetch(`${BASE}/notes/${id}`, {
      method: "PATCH",
      body: JSON.stringify({ size: ciphertext.length }),
    }),
  ]);
  return { id };
}

export async function updateNoteContent(id: string, text: string): Promise<void> {
  const meta = await fetchNoteMeta(id);
  const ciphertext = await encryptNoteOwner(text, meta.board_id, id);
  const snippet = text.split("\n").find(l => l.trim()) ?? text.slice(0, 80);
  const [blobResp, patchResp] = await Promise.all([
    authedFetch(`${BASE}/notes/${id}/blob`, {
      method: "PUT",
      headers: { "Content-Type": "application/octet-stream" },
      body: ciphertext,
    }),
    authedFetch(`${BASE}/notes/${id}`, {
      method: "PATCH",
      body: JSON.stringify({ snippet: snippet.slice(0, 80) }),
    }),
  ]);
  if (!blobResp.ok) throw new Error(`Impossible de sauvegarder (${blobResp.status})`);
  if (!patchResp.ok) throw new Error(`Impossible de mettre à jour (${patchResp.status})`);
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
  const r = await authedFetch(`${BASE}/link/init`, { method: "POST" });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function confirmLink(linkToken: string): Promise<void> {
  const r = await authedFetch(`${BASE}/link/confirm`, {
    method: "POST",
    body: JSON.stringify({ token: linkToken }),
  });
  // "not pending" means the server already auto-confirmed (new server) — ignore
  if (!r.ok && r.status !== 400) throw new Error(await r.text());
}

export async function getLinkStatus(linkToken: string): Promise<{ status: string; jwt?: string }> {
  const r = await fetch(`${BASE}/link/status/${linkToken}`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

// ─── Identity ─────────────────────────────────────────────────────────────────

export interface IdentityInfo { id: string; friendly_name: string; public_key_x25519?: string }

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
  const identity = await lookupIdentity(target);
  if (!identity) throw new Error(`Identité "${target}" introuvable`);
  if (!identity.public_key_x25519) throw new Error(`"${target}" n'a pas de clé publique — ils doivent d'abord créer une note`);

  // Register our own public key so the recipient can look it up for BEK decryption.
  await authedFetch(`${BASE}/identity/me/pubkey`, {
    method: "PUT",
    body: JSON.stringify({ public_key_x25519: await getPublicKeyHex() }),
  });

  // Grant board access.
  const r = await authedFetch(`${BASE}/boards/${boardId}/shares`, {
    method: "POST", body: JSON.stringify({ target }),
  });
  if (!r.ok) throw new Error(await r.text());

  // Encrypt and upload the BEK for the recipient (one key, covers all board notes).
  const encryptedBek = await encryptBekForRecipient(boardId, identity.public_key_x25519);
  const keyResp = await authedFetch(`${BASE}/boards/${boardId}/keys/${identity.id}`, {
    method: "PUT",
    body: JSON.stringify({ encrypted_bek: encryptedBek }),
  });
  if (!keyResp.ok) throw new Error(await keyResp.text());
}

export async function revokeBoardShare(boardId: string, targetId: string): Promise<void> {
  // Delete board share and the member's BEK concurrently.
  await Promise.all([
    authedFetch(`${BASE}/boards/${boardId}/shares/${targetId}`, { method: "DELETE" }),
    authedFetch(`${BASE}/boards/${boardId}/keys/${targetId}`, { method: "DELETE" }),
  ]);
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

export interface ShareEntry { shared_with_id: string; shared_with_name: string | null; created_at: string; permission: string; public_key_x25519?: string }
export interface SharedNote  { note_id: string; note_type: string; board_id: string; owner_identity_id: string; owner_friendly_name: string | null; snippet?: string }

export async function fetchShares(noteId: string): Promise<ShareEntry[]> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/shares`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function shareNote(noteId: string, target: string, permission: "read" | "write" | "delete" = "read"): Promise<void> {
  const identity = await lookupIdentity(target);
  if (!identity) throw new Error(`Identité "${target}" introuvable`);
  if (!identity.public_key_x25519) throw new Error(`"${target}" n'a pas de clé publique enregistrée — ils doivent d'abord créer une note depuis ce compte`);

  // Register our public key so the recipient can derive the ECDH wrap key.
  await authedFetch(`${BASE}/identity/me/pubkey`, {
    method: "PUT",
    body: JSON.stringify({ public_key_x25519: await getPublicKeyHex() }),
  });

  // Get board_id to derive the DEK locally.
  const meta = await fetchNoteMeta(noteId);

  // Encrypt the derived DEK for the recipient.
  const encryptedDekForRecipient = await encryptDekForRecipient(meta.board_id, noteId, identity.public_key_x25519);

  const r = await authedFetch(`${BASE}/notes/${noteId}/shares`, {
    method: "POST",
    body: JSON.stringify({ target, encrypted_dek_for_recipient: encryptedDekForRecipient, permission }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function revokeShare(noteId: string, targetId: string): Promise<void> {
  // With deterministic BEK→DEK derivation, revocation removes the recipient's DEK entry.
  // No re-encryption needed (the owner's DEK is never stored).
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

// ─── Blocks ───────────────────────────────────────────────────────────────────

export interface BlockDto {
  id: string;
  note_id: string;
  parent_block_id: string | null;
  position: number;
  block_type: "text" | "heading" | "todo" | "quote" | "code" | "embed" | "divider";
  content: string;            // base64 ciphertext
  metadata: string | null;    // base64 ciphertext
  collapsed: boolean;
  created_at: string;
  updated_at: string;
}

export async function listBlocks(noteId: string): Promise<BlockDto[]> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/blocks`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function createBlock(
  noteId: string,
  input: { parent_id?: string | null; position?: number; block_type: string; content_b64: string; metadata_b64?: string | null }
): Promise<BlockDto> {
  const r = await authedFetch(`${BASE}/notes/${noteId}/blocks`, { method: "POST", body: JSON.stringify(input) });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function patchBlock(
  id: string,
  patch: { block_type?: string; content_b64?: string; metadata_b64?: string | null; collapsed?: boolean }
): Promise<BlockDto> {
  const r = await authedFetch(`${BASE}/blocks/${id}`, { method: "PATCH", body: JSON.stringify(patch) });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function deleteBlock(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}`, { method: "DELETE" });
  if (!r.ok) throw new Error(await r.text());
}

export async function moveBlock(id: string, new_parent_id: string | null, new_position: number): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/move`, {
    method: "POST", body: JSON.stringify({ new_parent_id, new_position }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function indentBlock(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/indent`, { method: "POST", body: "{}" });
  if (!r.ok) throw new Error(await r.text());
}
export async function outdentBlock(id: string): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/outdent`, { method: "POST", body: "{}" });
  if (!r.ok) throw new Error(await r.text());
}

export async function putBlockLinks(
  id: string,
  links: { target_kind: string; target_id: string; link_kind: string }[]
): Promise<void> {
  const r = await authedFetch(`${BASE}/blocks/${id}/links`, { method: "PUT", body: JSON.stringify({ links }) });
  if (!r.ok) throw new Error(await r.text());
}

export interface BackLinkRow {
  source_block_id: string;
  source_note_id: string;
  link_kind: string;
}

export async function blockBacklinks(id: string): Promise<BackLinkRow[]> {
  const r = await authedFetch(`${BASE}/blocks/${id}/backlinks`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function noteBacklinks(id: string): Promise<BackLinkRow[]> {
  const r = await authedFetch(`${BASE}/notes/${id}/backlinks`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export interface TagDto { name: string; color?: string | null }

export async function listTags(): Promise<TagDto[]> {
  const r = await authedFetch(`${BASE}/tags`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function putTag(name: string, color?: string | null): Promise<void> {
  const r = await authedFetch(`${BASE}/tags/${encodeURIComponent(name)}`, {
    method: "PUT", body: JSON.stringify({ color: color ?? null }),
  });
  if (!r.ok) throw new Error(await r.text());
}

export async function blocksWithTag(name: string): Promise<string[]> {
  const r = await authedFetch(`${BASE}/tags/${encodeURIComponent(name)}/blocks`);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}
