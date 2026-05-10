# Workspace scaffold + crate core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bootstrap the Cargo workspace and implement `crate core` with data models and full cryptographic stack (HKDF, AES-256-GCM, Ed25519, X25519).

**Architecture:** Domain-organised modules inside `crate core` — `models/` for pure data structs, `crypto/` for four focused crypto primitives. `thiserror` in `core` (lib), `anyhow` in future bins. Stub crates (`detect`, `storage`, `api`, `cli`) compile immediately.

**Tech Stack:** Rust 2021, aes-gcm 0.10, hkdf 0.12, sha2 0.10, ed25519-dalek 2, x25519-dalek 2, rand 0.8, serde 1, uuid 1, chrono 0.4, thiserror 1.

---

## Task 1: Workspace Cargo.toml + stub crates

**Files:**
- Create: `Cargo.toml`
- Create: `crates/detect/Cargo.toml`
- Create: `crates/detect/src/lib.rs`
- Create: `crates/storage/Cargo.toml`
- Create: `crates/storage/src/lib.rs`
- Create: `crates/api/Cargo.toml`
- Create: `crates/api/src/lib.rs`
- Create: `crates/cli/Cargo.toml`
- Create: `crates/cli/src/lib.rs`

- [ ] **Step 1: Create root workspace `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/detect",
    "crates/storage",
    "crates/api",
    "crates/cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Jacques HULLU <jacques@hullu.fr>"]
license = "MIT"
```

- [ ] **Step 2: Create stub `crates/detect/Cargo.toml`**

```toml
[package]
name = "detect"
version.workspace = true
edition.workspace = true
```

- [ ] **Step 3: Create `crates/detect/src/lib.rs`**

```rust
// stub — implemented in sub-project 2
```

- [ ] **Step 4: Repeat for `storage`, `api`, `cli` — same pattern**

`crates/storage/Cargo.toml`:
```toml
[package]
name = "storage"
version.workspace = true
edition.workspace = true
```

`crates/storage/src/lib.rs`:
```rust
// stub — implemented in sub-project 3
```

`crates/api/Cargo.toml`:
```toml
[package]
name = "api"
version.workspace = true
edition.workspace = true
```

`crates/api/src/lib.rs`:
```rust
// stub — implemented in sub-project 4
```

`crates/cli/Cargo.toml`:
```toml
[package]
name = "cli"
version.workspace = true
edition.workspace = true
```

`crates/cli/src/lib.rs`:
```rust
// stub — implemented in sub-project 5
```

- [ ] **Step 5: Verify workspace compiles (core not yet added)**

```bash
cargo check --workspace
```
Expected: error about missing `crates/core` member — normal, we add it next.

---

## Task 2: `crate core` — Cargo.toml + scaffolding

**Files:**
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/core/src/error.rs`

- [ ] **Step 1: Create `crates/core/Cargo.toml`**

```toml
[package]
name = "core"
version.workspace = true
edition.workspace = true

[dependencies]
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
thiserror = "1"
aes-gcm = "0.10"
hkdf = "0.12"
sha2 = "0.10"
ed25519-dalek = { version = "2", features = ["rand_core"] }
x25519-dalek = { version = "2", features = ["static_secrets"] }
rand = "0.8"
```

- [ ] **Step 2: Create `crates/core/src/error.rs`**

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("encryption failed: {0}")]
    Encryption(String),
    #[error("decryption failed: invalid key or corrupted data")]
    Decryption,
    #[error("signature verification failed")]
    SignatureInvalid,
    #[error("HKDF expand failed")]
    KeyDerivation,
}
```

- [ ] **Step 3: Create `crates/core/src/lib.rs`**

```rust
pub mod crypto;
pub mod error;
pub mod models;

pub use error::CoreError;
```

- [ ] **Step 4: Add `crates/core` to workspace members in root `Cargo.toml`**

Update `Cargo.toml`:
```toml
members = [
    "crates/core",
    "crates/detect",
    "crates/storage",
    "crates/api",
    "crates/cli",
]
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p core
```
Expected: `error[E0583]: file not found for module` for `crypto` and `models` — normal, we add them next.

---

## Task 3: Models — `note.rs`

**Files:**
- Create: `crates/core/src/models/note.rs`
- Create: `crates/core/src/models/mod.rs` (partial)

- [ ] **Step 1: Create `crates/core/src/models/note.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteType {
    Text,
    Voice,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub note_type: NoteType,
    pub content: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
    pub duration_ms: Option<u32>,
    pub color: String,
    pub board_id: Uuid,
    pub position: i32,
    pub blob_key: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Create `crates/core/src/models/mod.rs` (partial)**

```rust
pub mod note;

pub use note::{Note, NoteType};
```

- [ ] **Step 3: Check compile**

```bash
cargo check -p core
```
Expected: errors about missing `board`, `device`, `link` — normal.

---

## Task 4: Models — `board.rs`, `device.rs`, `link.rs`

**Files:**
- Create: `crates/core/src/models/board.rs`
- Create: `crates/core/src/models/device.rs`
- Create: `crates/core/src/models/link.rs`
- Modify: `crates/core/src/models/mod.rs`

- [ ] **Step 1: Create `crates/core/src/models/board.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub name: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Create `crates/core/src/models/device.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub pub_key_x25519: String,
    pub pub_key_ed25519: String,
    pub name: String,
    pub last_seen: DateTime<Utc>,
}
```

- [ ] **Step 3: Create `crates/core/src/models/link.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkStatus {
    Pending,
    Confirmed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkSession {
    pub token: String,
    pub code: String,
    pub status: LinkStatus,
    pub pub_key_initiator: String,
    pub encrypted_symkey: Option<Vec<u8>>,
    pub expires_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Complete `crates/core/src/models/mod.rs`**

```rust
pub mod board;
pub mod device;
pub mod link;
pub mod note;

pub use board::Board;
pub use device::Device;
pub use link::{LinkSession, LinkStatus};
pub use note::{Note, NoteType};
```

- [ ] **Step 5: Verify models compile**

```bash
cargo check -p core
```
Expected: errors about missing `crypto` module — normal.

- [ ] **Step 6: Commit**

```bash
git add crates/ Cargo.toml
git commit -m "feat: workspace scaffold + core models (Note, Board, Device, LinkSession)"
```

---

## Task 5: `crypto/kdf.rs` — HKDF-SHA256

**Files:**
- Create: `crates/core/src/crypto/kdf.rs`
- Create: `crates/core/src/crypto/mod.rs` (partial)

- [ ] **Step 1: Create stub `crates/core/src/crypto/kdf.rs`**

```rust
use crate::CoreError;
use hkdf::Hkdf;
use sha2::Sha256;

pub struct DerivedKeys {
    pub notes_key: [u8; 32],
    pub ai_key: [u8; 32],
}

pub fn derive_keys(master_key: &[u8; 32]) -> Result<DerivedKeys, CoreError> {
    unimplemented!()
}
```

- [ ] **Step 2: Create partial `crates/core/src/crypto/mod.rs`**

```rust
pub mod kdf;

pub use kdf::{DerivedKeys, derive_keys};
```

- [ ] **Step 3: Write failing tests in `kdf.rs`**

Append to `crates/core/src/crypto/kdf.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_master_key() -> [u8; 32] {
        [42u8; 32]
    }

    #[test]
    fn derived_keys_are_reproducible() {
        let k1 = derive_keys(&test_master_key()).unwrap();
        let k2 = derive_keys(&test_master_key()).unwrap();
        assert_eq!(k1.notes_key, k2.notes_key);
        assert_eq!(k1.ai_key, k2.ai_key);
    }

    #[test]
    fn notes_key_and_ai_key_differ() {
        let keys = derive_keys(&test_master_key()).unwrap();
        assert_ne!(keys.notes_key, keys.ai_key);
    }

    #[test]
    fn different_master_keys_produce_different_outputs() {
        let k1 = derive_keys(&[1u8; 32]).unwrap();
        let k2 = derive_keys(&[2u8; 32]).unwrap();
        assert_ne!(k1.notes_key, k2.notes_key);
    }
}
```

- [ ] **Step 4: Run tests — expect failure**

```bash
cargo test -p core crypto::kdf
```
Expected: `panicked at 'not yet implemented'`

- [ ] **Step 5: Implement `derive_keys`**

Replace the `unimplemented!()` body:
```rust
pub fn derive_keys(master_key: &[u8; 32]) -> Result<DerivedKeys, CoreError> {
    let hk = Hkdf::<Sha256>::new(None, master_key);

    let mut notes_key = [0u8; 32];
    hk.expand(b"jot-v1-notes", &mut notes_key)
        .map_err(|_| CoreError::KeyDerivation)?;

    let mut ai_key = [0u8; 32];
    hk.expand(b"jot-v1-ai", &mut ai_key)
        .map_err(|_| CoreError::KeyDerivation)?;

    Ok(DerivedKeys { notes_key, ai_key })
}
```

- [ ] **Step 6: Run tests — expect pass**

```bash
cargo test -p core crypto::kdf
```
Expected: `test result: ok. 3 passed`

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/crypto/
git commit -m "feat(core): HKDF-SHA256 key derivation (notes_key + ai_key)"
```

---

## Task 6: `crypto/encryption.rs` — AES-256-GCM

**Files:**
- Create: `crates/core/src/crypto/encryption.rs`
- Modify: `crates/core/src/crypto/mod.rs`

- [ ] **Step 1: Create stub `crates/core/src/crypto/encryption.rs`**

```rust
use crate::CoreError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::{rngs::OsRng, RngCore};

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CoreError> {
    unimplemented!()
}

pub fn decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, CoreError> {
    unimplemented!()
}
```

- [ ] **Step 2: Write failing tests**

Append to `crates/core/src/crypto/encryption.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [7u8; 32]
    }

    #[test]
    fn round_trip() {
        let plaintext = b"hello, jot!";
        let ciphertext = encrypt(&test_key(), plaintext).unwrap();
        let recovered = decrypt(&test_key(), &ciphertext).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn ciphertext_differs_from_plaintext() {
        let plaintext = b"secret note";
        let ciphertext = encrypt(&test_key(), plaintext).unwrap();
        assert_ne!(&ciphertext[12..], plaintext.as_slice());
    }

    #[test]
    fn two_encryptions_produce_different_blobs() {
        let plaintext = b"same content";
        let c1 = encrypt(&test_key(), plaintext).unwrap();
        let c2 = encrypt(&test_key(), plaintext).unwrap();
        assert_ne!(c1, c2); // different random nonces
    }

    #[test]
    fn tampered_nonce_fails_decryption() {
        let plaintext = b"tamper test";
        let mut ciphertext = encrypt(&test_key(), plaintext).unwrap();
        ciphertext[0] ^= 0xFF; // flip first nonce byte
        assert!(decrypt(&test_key(), &ciphertext).is_err());
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let plaintext = b"wrong key test";
        let ciphertext = encrypt(&test_key(), plaintext).unwrap();
        let wrong_key = [99u8; 32];
        assert!(decrypt(&wrong_key, &ciphertext).is_err());
    }

    #[test]
    fn too_short_ciphertext_fails() {
        assert!(decrypt(&test_key(), &[0u8; 11]).is_err());
    }
}
```

- [ ] **Step 3: Run tests — expect failure**

```bash
cargo test -p core crypto::encryption
```
Expected: `panicked at 'not yet implemented'`

- [ ] **Step 4: Implement `encrypt` and `decrypt`**

Replace the two `unimplemented!()` bodies:
```rust
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CoreError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CoreError::Encryption(e.to_string()))?;

    // blob format: [12-byte nonce] || [ciphertext + 16-byte GCM tag]
    let mut blob = Vec::with_capacity(12 + encrypted.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&encrypted);
    Ok(blob)
}

pub fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, CoreError> {
    if blob.len() < 12 {
        return Err(CoreError::Decryption);
    }
    let (nonce_bytes, cipher_data) = blob.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, cipher_data).map_err(|_| CoreError::Decryption)
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test -p core crypto::encryption
```
Expected: `test result: ok. 6 passed`

- [ ] **Step 6: Update `crypto/mod.rs`**

```rust
pub mod encryption;
pub mod kdf;

pub use encryption::{decrypt, encrypt};
pub use kdf::{derive_keys, DerivedKeys};
```

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/crypto/encryption.rs crates/core/src/crypto/mod.rs
git commit -m "feat(core): AES-256-GCM encryption/decryption with random nonce"
```

---

## Task 7: `crypto/signing.rs` — Ed25519

**Files:**
- Create: `crates/core/src/crypto/signing.rs`
- Modify: `crates/core/src/crypto/mod.rs`

- [ ] **Step 1: Create stub `crates/core/src/crypto/signing.rs`**

```rust
use crate::CoreError;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};

pub fn generate_device_keypair() -> (SigningKey, VerifyingKey) {
    unimplemented!()
}

pub fn sign(key: &SigningKey, message: &[u8]) -> Signature {
    unimplemented!()
}

pub fn verify(key: &VerifyingKey, message: &[u8], sig: &Signature) -> Result<(), CoreError> {
    unimplemented!()
}
```

- [ ] **Step 2: Write failing tests**

Append to `crates/core/src/crypto/signing.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_ok() {
        let (signing_key, verifying_key) = generate_device_keypair();
        let message = b"device token payload";
        let sig = sign(&signing_key, message);
        assert!(verify(&verifying_key, message, &sig).is_ok());
    }

    #[test]
    fn verify_fails_with_wrong_key() {
        let (signing_key, _) = generate_device_keypair();
        let (_, other_verifying_key) = generate_device_keypair();
        let message = b"device token payload";
        let sig = sign(&signing_key, message);
        assert!(verify(&other_verifying_key, message, &sig).is_err());
    }

    #[test]
    fn verify_fails_with_tampered_message() {
        let (signing_key, verifying_key) = generate_device_keypair();
        let message = b"original message";
        let sig = sign(&signing_key, message);
        assert!(verify(&verifying_key, b"tampered message", &sig).is_err());
    }

    #[test]
    fn two_keypairs_are_different() {
        let (k1, _) = generate_device_keypair();
        let (k2, _) = generate_device_keypair();
        assert_ne!(k1.to_bytes(), k2.to_bytes());
    }
}
```

- [ ] **Step 3: Run tests — expect failure**

```bash
cargo test -p core crypto::signing
```
Expected: `panicked at 'not yet implemented'`

- [ ] **Step 4: Implement**

Replace the three `unimplemented!()` bodies:
```rust
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

pub fn generate_device_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

pub fn sign(key: &SigningKey, message: &[u8]) -> Signature {
    key.sign(message)
}

pub fn verify(key: &VerifyingKey, message: &[u8], sig: &Signature) -> Result<(), CoreError> {
    key.verify(message, sig).map_err(|_| CoreError::SignatureInvalid)
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test -p core crypto::signing
```
Expected: `test result: ok. 4 passed`

- [ ] **Step 6: Update `crypto/mod.rs`**

```rust
pub mod encryption;
pub mod kdf;
pub mod signing;

pub use encryption::{decrypt, encrypt};
pub use kdf::{derive_keys, DerivedKeys};
pub use signing::{generate_device_keypair, sign, verify};
```

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/crypto/signing.rs crates/core/src/crypto/mod.rs
git commit -m "feat(core): Ed25519 keypair generation, sign, verify"
```

---

## Task 8: `crypto/exchange.rs` — X25519 ECDH

**Files:**
- Create: `crates/core/src/crypto/exchange.rs`
- Modify: `crates/core/src/crypto/mod.rs`

- [ ] **Step 1: Create stub `crates/core/src/crypto/exchange.rs`**

```rust
use x25519_dalek::{EphemeralSecret, PublicKey};

pub fn generate_ephemeral_keypair() -> (EphemeralSecret, PublicKey) {
    unimplemented!()
}

pub fn diffie_hellman(secret: EphemeralSecret, peer_pub: &PublicKey) -> [u8; 32] {
    unimplemented!()
}
```

- [ ] **Step 2: Write failing tests**

Append to `crates/core/src/crypto/exchange.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_secret_is_symmetric() {
        let (alice_secret, alice_pub) = generate_ephemeral_keypair();
        let (bob_secret, bob_pub) = generate_ephemeral_keypair();

        let alice_shared = diffie_hellman(alice_secret, &bob_pub);
        let bob_shared = diffie_hellman(bob_secret, &alice_pub);

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn different_pairs_produce_different_secrets() {
        let (s1, p1) = generate_ephemeral_keypair();
        let (s2, p2) = generate_ephemeral_keypair();
        let (s3, _) = generate_ephemeral_keypair();

        let shared_ab = diffie_hellman(s1, &p2);
        let shared_ac = diffie_hellman(s3, &p1);

        assert_ne!(shared_ab, shared_ac);
    }
}
```

- [ ] **Step 3: Run tests — expect failure**

```bash
cargo test -p core crypto::exchange
```
Expected: `panicked at 'not yet implemented'`

- [ ] **Step 4: Implement**

Replace the two `unimplemented!()` bodies:
```rust
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey};

pub fn generate_ephemeral_keypair() -> (EphemeralSecret, PublicKey) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

pub fn diffie_hellman(secret: EphemeralSecret, peer_pub: &PublicKey) -> [u8; 32] {
    secret.diffie_hellman(peer_pub).to_bytes()
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test -p core crypto::exchange
```
Expected: `test result: ok. 2 passed`

- [ ] **Step 6: Finalize `crypto/mod.rs`**

```rust
pub mod encryption;
pub mod exchange;
pub mod kdf;
pub mod signing;

pub use encryption::{decrypt, encrypt};
pub use exchange::{diffie_hellman, generate_ephemeral_keypair};
pub use kdf::{derive_keys, DerivedKeys};
pub use signing::{generate_device_keypair, sign, verify};
```

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/crypto/exchange.rs crates/core/src/crypto/mod.rs
git commit -m "feat(core): X25519 ECDH ephemeral key exchange"
```

---

## Task 9: Full workspace check + CI

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace
```
Expected: all tests in `crates/core` pass, stub crates have 0 tests.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```
Expected: no warnings. Fix any lint before proceeding.

- [ ] **Step 3: Run fmt check**

```bash
cargo fmt --all -- --check
```
Expected: no diff. If diff, run `cargo fmt --all` then re-check.

- [ ] **Step 4: Create `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - name: fmt
        run: cargo fmt --all -- --check
      - name: clippy
        run: cargo clippy --workspace -- -D warnings
      - name: test
        run: cargo test --workspace
```

- [ ] **Step 5: Final commit**

```bash
git add .github/
git commit -m "ci: add GitHub Actions workflow (fmt + clippy + test)"
```
