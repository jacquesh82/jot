// Per-block E2E crypto.
//
// Blocks reuse the same per-note DEK as the note body (HKDF-SHA256 from BEK
// derived from identity privkey + board UUID, then from BEK + note UUID).
// Each block ciphertext is an independent AES-256-GCM blob with a random
// 12-byte nonce prefix, base64-encoded for transport (matches BlockDto.content).
//
// We need the boardId in addition to noteId because the DEK is derived via
// BEK(boardId) → DEK(boardId, noteId). Callers typically have both in scope
// (BlockEditor receives the note meta which includes board_id).

import { encryptNoteOwner, decryptNoteOwner } from "../crypto";

function bytesToB64(u8: Uint8Array): string {
  let s = "";
  for (let i = 0; i < u8.length; i++) s += String.fromCharCode(u8[i]);
  return btoa(s);
}

function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Encrypt a block payload using the per-note DEK; returns base64 ciphertext. */
export async function encryptBlock(
  boardId: string,
  noteId: string,
  plaintext: string,
): Promise<string> {
  const blob = await encryptNoteOwner(plaintext, boardId, noteId);
  return bytesToB64(blob);
}

/** Decrypt a base64 block ciphertext using the per-note DEK. */
export async function decryptBlock(
  boardId: string,
  noteId: string,
  ciphertextB64: string,
): Promise<string> {
  if (!ciphertextB64) return "";
  const blob = b64ToBytes(ciphertextB64);
  return decryptNoteOwner(blob, boardId, noteId);
}
