// E2E crypto for jot SPA — mirrors the Rust crypto in crates/core/src/crypto/
// Algorithms: X25519 ECDH + HKDF-SHA256 + AES-256-GCM
// Key hierarchy: identity_privkey → BEK (per board) → DEK (per note)
// Key source: fetched from GET /identity/me/privkey (the server's identity.key)
// so the SPA and CLI share the same key pair.

// ── helpers ──────────────────────────────────────────────────────────────────

function hexEnc(u8: Uint8Array): string {
  return Array.from(u8).map(b => b.toString(16).padStart(2, "0")).join("");
}
function hexDec(hex: string): Uint8Array {
  const m = hex.match(/.{2}/g);
  if (!m) throw new Error("invalid hex");
  return Uint8Array.from(m.map(b => parseInt(b, 16)));
}

// UUID string "xxxxxxxx-xxxx-..." → 16 raw bytes
function uuidToBytes(uuid: string): Uint8Array {
  return hexDec(uuid.replace(/-/g, ""));
}

// X25519 SPKI = 12-byte DER header + 32-byte raw key
function pubKeyRaw(spki: ArrayBuffer): Uint8Array {
  return new Uint8Array(spki).slice(12);
}

// ── key pair — sourced from the server's identity.key ────────────────────────

let _keyPairCache: CryptoKeyPair | null = null;
let _rawPrivKeyCache: Uint8Array | null = null;

async function getKeyPair(): Promise<CryptoKeyPair> {
  if (_keyPairCache) return _keyPairCache;

  const tok = localStorage.getItem("token") ?? "";
  const r = await fetch("/identity/me/privkey", {
    headers: { Authorization: `Bearer ${tok}` },
  });
  if (!r.ok) throw new Error("cannot fetch identity key from server");
  const { private_key_x25519 } = await r.json() as { private_key_x25519: string };
  const rawPriv = hexDec(private_key_x25519);
  _rawPrivKeyCache = rawPriv;

  // X25519 raw private key → import as pkcs8 wrapper
  const pkcs8 = buildX25519Pkcs8(rawPriv);
  const privKey = await crypto.subtle.importKey("pkcs8", pkcs8, { name: "X25519" }, true, ["deriveBits"]);
  const pubKeySpki = await deriveX25519PublicKey(privKey);
  const pubKey = await crypto.subtle.importKey("spki", pubKeySpki, { name: "X25519" }, true, []);

  _keyPairCache = { privateKey: privKey, publicKey: pubKey };
  return _keyPairCache;
}

// Return raw 32-byte private key scalar (needed as HKDF IKM for BEK derivation).
async function getRawPrivateKey(): Promise<Uint8Array> {
  if (_rawPrivKeyCache) return _rawPrivKeyCache;
  await getKeyPair(); // populates cache
  return _rawPrivKeyCache!;
}

function buildX25519Pkcs8(raw: Uint8Array): Uint8Array {
  const alg = new Uint8Array([0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e]);
  const innerOctet = new Uint8Array([0x04, 0x20, ...raw]);
  const outerOctet = new Uint8Array([0x04, 0x22, ...innerOctet]);
  const version = new Uint8Array([0x02, 0x01, 0x00]);
  const body = new Uint8Array([...version, ...alg, ...outerOctet]);
  return new Uint8Array([0x30, body.length, ...body]);
}

async function deriveX25519PublicKey(privKey: CryptoKey): Promise<ArrayBuffer> {
  const jwk = await crypto.subtle.exportKey("jwk", privKey) as JsonWebKey & { x?: string };
  if (!jwk.x) throw new Error("no public key in JWK");
  const pubRaw = Uint8Array.from(atob(jwk.x.replace(/-/g,"+").replace(/_/g,"/")), c => c.charCodeAt(0));
  return new Uint8Array([
    0x30, 0x2a,
    0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e,
    0x03, 0x21, 0x00,
    ...pubRaw,
  ]).buffer;
}

// Return hex-encoded raw 32-byte public key (for PUT /identity/me/pubkey)
export async function getPublicKeyHex(): Promise<string> {
  const { publicKey } = await getKeyPair();
  const spki = await crypto.subtle.exportKey("spki", publicKey);
  return hexEnc(pubKeyRaw(spki));
}

// ── ECDH + HKDF wrap key (for individual note sharing) ───────────────────────
// Cross-ECDH: ECDH(my_priv, peer_pub) → HKDF("jot-share-v1") → AES-GCM key.
// By ECDH symmetry: ECDH(owner_priv, recipient_pub) == ECDH(recipient_priv, owner_pub).

async function wrapKeyFor(peerPubKeyHex?: string): Promise<CryptoKey> {
  const pair = await getKeyPair();
  let peerPub: CryptoKey;
  if (peerPubKeyHex) {
    const raw = hexDec(peerPubKeyHex);
    const spki = new Uint8Array([0x30,0x2a,0x30,0x05,0x06,0x03,0x2b,0x65,0x6e,0x03,0x21,0x00,...raw]);
    peerPub = await crypto.subtle.importKey("spki", spki, { name: "X25519" }, false, []);
  } else {
    peerPub = pair.publicKey;
  }
  const shared = await crypto.subtle.deriveBits(
    { name: "X25519", public: peerPub } as EcdhKeyDeriveParams,
    pair.privateKey,
    256,
  );
  const hkdfKey = await crypto.subtle.importKey("raw", shared, "HKDF", false, ["deriveKey"]);
  return crypto.subtle.deriveKey(
    { name: "HKDF", hash: "SHA-256", salt: new Uint8Array(32), info: new TextEncoder().encode("jot-share-v1") },
    hkdfKey,
    { name: "AES-GCM", length: 256 },
    true,
    ["encrypt", "decrypt"],
  );
}

// ── AES-256-GCM helpers ──────────────────────────────────────────────────────
// Blob format: [12-byte nonce] || [ciphertext + 16-byte GCM tag]

async function aesgcmEncrypt(key: CryptoKey, plaintext: Uint8Array): Promise<Uint8Array> {
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const ct = await crypto.subtle.encrypt({ name: "AES-GCM", iv: nonce }, key, plaintext);
  const out = new Uint8Array(12 + ct.byteLength);
  out.set(nonce);
  out.set(new Uint8Array(ct), 12);
  return out;
}

async function aesgcmDecrypt(key: CryptoKey, blob: Uint8Array): Promise<Uint8Array> {
  if (blob.length < 12) throw new Error("blob too short");
  const pt = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: blob.slice(0, 12) },
    key,
    blob.slice(12),
  );
  return new Uint8Array(pt);
}

// ── Hierarchical key derivation ───────────────────────────────────────────────
// BEK = HKDF-SHA256(ikm=privkey, salt=zero32, info="jot-board-v1" || board_id_bytes)
// DEK = HKDF-SHA256(ikm=bek,     salt=zero32, info="jot-note-v1"  || note_id_bytes)
// Mirrors crates/core/src/crypto/kdf.rs::derive_bek / derive_dek

async function hkdfDerive(ikm: Uint8Array, info: Uint8Array): Promise<Uint8Array> {
  const hkdfKey = await crypto.subtle.importKey("raw", ikm, "HKDF", false, ["deriveBits"]);
  const bits = await crypto.subtle.deriveBits(
    { name: "HKDF", hash: "SHA-256", salt: new Uint8Array(32), info },
    hkdfKey,
    256,
  );
  return new Uint8Array(bits);
}

// Derive BEK from our own identity private key + board UUID.
export async function deriveBek(boardId: string): Promise<Uint8Array> {
  const priv = await getRawPrivateKey();
  const info = new Uint8Array(12 + 16);
  info.set(new TextEncoder().encode("jot-board-v1"), 0);
  info.set(uuidToBytes(boardId), 12);
  return hkdfDerive(priv, info);
}

// Derive DEK from a BEK + note UUID.
export async function deriveDek(bek: Uint8Array, noteId: string): Promise<Uint8Array> {
  const info = new Uint8Array(11 + 16);
  info.set(new TextEncoder().encode("jot-note-v1"), 0);
  info.set(uuidToBytes(noteId), 11);
  return hkdfDerive(bek, info);
}

// ── AES-GCM key from raw bytes ────────────────────────────────────────────────

async function importDek(dek: Uint8Array, usage: KeyUsage[]): Promise<CryptoKey> {
  return crypto.subtle.importKey("raw", dek, { name: "AES-GCM", length: 256 }, false, usage);
}

// ── Owner encrypt / decrypt ───────────────────────────────────────────────────

// Encrypt plaintext using the deterministically derived DEK for this note.
export async function encryptNoteOwner(
  plaintext: string,
  boardId: string,
  noteId: string,
): Promise<Uint8Array> {
  const bek = await deriveBek(boardId);
  const dek = await deriveDek(bek, noteId);
  const dekKey = await importDek(dek, ["encrypt"]);
  return aesgcmEncrypt(dekKey, new TextEncoder().encode(plaintext));
}

// Decrypt blob using the deterministically derived DEK for this note (owner path).
export async function decryptNoteOwner(
  blob: Uint8Array,
  boardId: string,
  noteId: string,
): Promise<string> {
  const bek = await deriveBek(boardId);
  const dek = await deriveDek(bek, noteId);
  const dekKey = await importDek(dek, ["decrypt"]);
  const pt = await aesgcmDecrypt(dekKey, blob);
  return new TextDecoder().decode(pt);
}

// ── Board member decrypt ──────────────────────────────────────────────────────

// Decrypt the encrypted BEK received from the board owner, then derive DEK.
// Uses cross-ECDH: ECDH(member_priv, owner_pub) → wrap key → AES-GCM decrypt BEK.
export async function decryptNoteAsMember(
  blob: Uint8Array,
  encryptedBekHex: string,
  ownerPubKeyHex: string,
  noteId: string,
): Promise<string> {
  const wrapKey = await wrapKeyFor(ownerPubKeyHex);
  const bek = await aesgcmDecrypt(wrapKey, hexDec(encryptedBekHex));
  const dek = await deriveDek(bek, noteId);
  const dekKey = await importDek(dek, ["decrypt"]);
  const pt = await aesgcmDecrypt(dekKey, blob);
  return new TextDecoder().decode(pt);
}

// ── Board sharing ─────────────────────────────────────────────────────────────

// Encrypt the owner's BEK for a recipient using cross-ECDH wrap key.
export async function encryptBekForRecipient(
  boardId: string,
  recipientPubKeyHex: string,
): Promise<string> {
  const bek = await deriveBek(boardId);
  const recipientWrap = await wrapKeyFor(recipientPubKeyHex);
  const encryptedBek = await aesgcmEncrypt(recipientWrap, bek);
  return hexEnc(encryptedBek);
}

// ── Individual note sharing ───────────────────────────────────────────────────

// Re-encrypt the derived DEK for a specific recipient (for note-level shares).
export async function encryptDekForRecipient(
  boardId: string,
  noteId: string,
  recipientPubKeyHex: string,
): Promise<string> {
  const bek = await deriveBek(boardId);
  const dek = await deriveDek(bek, noteId);
  const recipientWrap = await wrapKeyFor(recipientPubKeyHex);
  const encryptedDek = await aesgcmEncrypt(recipientWrap, dek);
  return hexEnc(encryptedDek);
}

// Decrypt an individually-shared note (recipient has DEK in note_shares).
// ownerPubKeyHex: needed to derive the ECDH wrap key used by the owner to encrypt the DEK.
export async function decryptNoteAsRecipient(
  blob: Uint8Array,
  encryptedDekHex: string,
  ownerPubKeyHex: string,
): Promise<string> {
  const wrapKey = await wrapKeyFor(ownerPubKeyHex);
  const dek = await aesgcmDecrypt(wrapKey, hexDec(encryptedDekHex));
  const dekKey = await importDek(dek, ["decrypt"]);
  const pt = await aesgcmDecrypt(dekKey, blob);
  return new TextDecoder().decode(pt);
}
