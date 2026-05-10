# Note Sharing: Cryptographic Design

## Overview

Jot uses a **per-note Data Encryption Key (DEK)** model. Each note has its own
symmetric key. When a note is shared, the DEK is re-encrypted for the recipient
so they can decrypt the note content without ever seeing the owner's master key.

---

## Current state (unencrypted transport)

In the current implementation the note body is stored and transmitted in
plaintext. The `blob_key` and `encrypted_symkey` fields exist in the schema but
are not yet used for real encryption. The sharing infrastructure (tables, API
routes, SPA UI) is fully wired; the cryptographic layer below is the planned
upgrade path.

---

## Planned: X25519 ECDH key wrapping

### Key hierarchy

```
Device key pair  (Ed25519 — for JWT signing)
    │
    └─► Identity key pair  (X25519 — for key agreement)
            │
            └─► Per-note DEK  (AES-256-GCM — encrypts note body)
```

Each identity generates a long-lived **X25519 key pair** stored in the
`identities` table (`public_key BLOB`). The private key never leaves the device
(stored in local keychain / `~/.config/jot/identity.key`).

### Note creation

1. Generate a random 32-byte DEK.
2. Encrypt the note body: `ciphertext = AES-256-GCM(DEK, nonce, plaintext)`.
3. Store `(ciphertext, nonce)` in the blob store.
4. Wrap the DEK for the owner:
   - Derive a wrapping key via ECDH: `wrap_key = HKDF(ECDH(owner_priv, owner_pub))`.
   - Store `encrypted_dek = AES-256-GCM(wrap_key, nonce2, DEK)` in
     `note_shares(owner_identity_id = owner_identity_id, shared_with_id = owner_identity_id)`.

### Sharing with another user

```
POST /notes/:id/shares  { target: "alice" | "<uuid>" }
```

Server-side steps:

1. Resolve `target` → `recipient_identity_id` + `recipient_public_key`.
2. Respond to the client with `recipient_public_key`.
3. Client derives shared secret: `shared = ECDH(owner_priv, recipient_pub)`.
4. Wrapping key: `wrap_key = HKDF-SHA256(shared, salt="jot-share-v1")`.
5. Client fetches its own wrapped DEK, unwraps it to get the raw DEK.
6. Client re-wraps: `encrypted_dek = AES-256-GCM(wrap_key, nonce, DEK)`.
7. Client sends `PUT /notes/:id/shares/:recipient_id  { encrypted_dek: <hex> }`.
8. Server stores the row — it never sees the plaintext DEK.

### Recipient decryption

1. Recipient fetches `encrypted_dek` from `GET /notes/:id/shares`.
2. Derives the same shared secret: `shared = ECDH(recipient_priv, owner_pub)`.
3. Unwraps: `DEK = AES-256-GCM-Decrypt(HKDF(shared), encrypted_dek)`.
4. Decrypts note body with the DEK.

---

## Security properties

| Property | Achieved by |
|---|---|
| Server never sees plaintext | All encryption/decryption on the client |
| Server never sees DEK | DEK wrapped with ECDH-derived key before upload |
| Per-note key isolation | Compromise of one DEK doesn't affect other notes |
| Forward secrecy (partial) | New ephemeral ECDH per share operation (planned) |
| Revocation | Delete `note_shares` row; recipient's wrapped DEK is discarded |

---

## Algorithms

| Primitive | Algorithm | Key size |
|---|---|---|
| Asymmetric | X25519 (ECDH) | 32 bytes |
| Symmetric | AES-256-GCM | 256 bits |
| KDF | HKDF-SHA-256 | — |
| Signing (JWT) | Ed25519 | 32 bytes |

---

## Implementation notes

- `jot-core` already has `encrypt` / `decrypt` (AES-256-GCM) and `derive_keys`
  (HKDF) in `crates/core/src/crypto/`.
- X25519 key generation: add `x25519-dalek` to `jot-core`.
- CLI: generate identity key pair on first `jot serve`; persist to
  `~/.config/jot/identity.key` (chmod 600).
- SPA: use `window.crypto.subtle` (`generateKey`, `deriveBits`, `wrapKey`) —
  no additional JS dependencies needed.
