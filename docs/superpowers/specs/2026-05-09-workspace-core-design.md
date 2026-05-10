# Spec : Workspace scaffold + crate core

**Date :** 2026-05-09
**Sous-projet :** 1 / 8
**Statut :** approuvé

---

## 1. Périmètre

Mettre en place le workspace Cargo et implémenter `crate core` avec :
- Les modèles de données (Note, Board, Device, LinkSession)
- La cryptographie complète (HKDF, AES-256-GCM, Ed25519, X25519)
- Les types d'erreurs

Les crates `detect`, `storage`, `api`, `cli` sont créés en stubs (Cargo.toml + lib.rs vide).

---

## 2. Structure du workspace

```
jot/
├── Cargo.toml                  # workspace, resolver = "2"
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── note.rs
│   │       │   ├── board.rs
│   │       │   ├── device.rs
│   │       │   └── link.rs
│   │       └── crypto/
│   │           ├── mod.rs
│   │           ├── kdf.rs
│   │           ├── encryption.rs
│   │           ├── signing.rs
│   │           └── exchange.rs
│   ├── detect/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── storage/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── api/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── cli/
│       ├── Cargo.toml
│       └── src/lib.rs
└── .github/workflows/ci.yml
```

---

## 3. Modèles de données

Tous les modèles dérivent `serde::{Serialize, Deserialize}`. Pas de `sqlx::FromRow` à ce stade (ajouté dans `crate storage`). Pas de logique métier dans les modèles.

### Note

```rust
pub enum NoteType { Text, Voice, Image }

pub struct Note {
    pub id: Uuid,
    pub note_type: NoteType,
    pub content: Vec<u8>,         // blob chiffré AES-256-GCM
    pub thumbnail: Option<Vec<u8>>,
    pub duration_ms: Option<u32>, // voice uniquement
    pub color: String,
    pub board_id: Uuid,
    pub position: i32,
    pub blob_key: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Board

```rust
pub struct Board {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub name: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}
```

### Device

```rust
pub struct Device {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub pub_key_x25519: String,   // hex
    pub pub_key_ed25519: String,  // hex
    pub name: String,
    pub last_seen: DateTime<Utc>,
}
```

### LinkSession

```rust
pub enum LinkStatus { Pending, Confirmed, Expired }

pub struct LinkSession {
    pub token: String,
    pub code: String,
    pub status: LinkStatus,
    pub pub_key_initiator: String, // hex
    pub encrypted_symkey: Option<Vec<u8>>,
    pub expires_at: DateTime<Utc>,
}
```

---

## 4. Cryptographie

### 4.1 kdf.rs — Dérivation HKDF-SHA256

```rust
pub struct DerivedKeys {
    pub notes_key: [u8; 32],
    pub ai_key: [u8; 32],
}

pub fn derive_keys(master_key: &[u8; 32]) -> DerivedKeys
```

Infos HKDF : `"jot-v1-notes"` → `notes_key`, `"jot-v1-ai"` → `ai_key`.

### 4.2 encryption.rs — AES-256-GCM

```rust
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CoreError>
pub fn decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, CoreError>
```

Format blob : `[12 bytes nonce] || [ciphertext] || [16 bytes GCM tag]`. Nonce aléatoire à chaque appel.

### 4.3 signing.rs — Ed25519

```rust
pub fn generate_device_keypair() -> (SigningKey, VerifyingKey)
pub fn sign(key: &SigningKey, message: &[u8]) -> Signature
pub fn verify(key: &VerifyingKey, message: &[u8], sig: &Signature) -> Result<(), CoreError>
```

### 4.4 exchange.rs — X25519 ECDH

```rust
pub fn generate_ephemeral_keypair() -> (EphemeralSecret, PublicKey)
pub fn diffie_hellman(secret: EphemeralSecret, peer_pub: &PublicKey) -> [u8; 32]
```

Le shared secret est utilisé une seule fois (flux liaison device) puis jeté. Jamais persisté.

---

## 5. Gestion d'erreurs

`thiserror` dans `core` (lib). `anyhow` sera utilisé dans `cli` et `api` (bins).

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

---

## 6. Dépendances `core/Cargo.toml`

```toml
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

---

## 7. Tests

Tests unitaires dans `#[cfg(test)]` de chaque module :

| Module | Cas couverts |
|---|---|
| `kdf` | Dérivation reproductible, `notes_key ≠ ai_key` |
| `encryption` | Round-trip encrypt→decrypt, tamper détecté (nonce modifié) |
| `signing` | sign→verify OK, verify échoue avec mauvaise clé |
| `exchange` | ECDH symétrique (shared secret Alice == shared secret Bob) |

---

## 8. CI

`.github/workflows/ci.yml` : `cargo test --workspace` + `cargo clippy -- -D warnings` + `cargo fmt --check`.

---

## 9. Hors périmètre

- `sqlx::FromRow` sur les modèles → sous-projet 3 (`storage`)
- Validation métier (limite 2000 chars, taille blobs) → sous-projet 4 (`api`)
- Feature flag `pro-ai` / LanceDB → sous-projet 7
- Client Flutter → sous-projet 8
