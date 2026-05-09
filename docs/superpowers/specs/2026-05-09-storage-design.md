# Spec : crate storage — SQLite + blob backend

**Date :** 2026-05-09
**Sous-projet :** 3 / 8
**Statut :** approuvé

---

## 1. Périmètre

Implémenter `crate storage` : couche de persistance complète pour jot.
- Base de données SQLite via `sqlx` (CRUD + migrations automatiques)
- Backend blobs abstrait (`BlobStore` trait) : `LocalStore` (filesystem) + `S3Store` (aws-sdk-s3)

Dépend de `jot-core` pour les modèles (`Note`, `Board`, `Device`, `LinkSession`).
`sqlx::FromRow` est implémenté ici (pas dans `core`).

---

## 2. Structure

```
crates/storage/
├── Cargo.toml
├── migrations/
│   └── 0001_initial.sql
└── src/
    ├── lib.rs
    ├── error.rs
    ├── db/
    │   ├── mod.rs       # Db struct (SqlitePool), connect(), migrate()
    │   ├── notes.rs
    │   ├── boards.rs
    │   ├── devices.rs
    │   └── links.rs
    └── blobs/
        ├── mod.rs       # BlobStore trait
        ├── local.rs     # LocalStore
        └── s3.rs        # S3Store
```

---

## 3. Dépendances

```toml
[dependencies]
jot-core = { path = "../core" }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio", "macros", "chrono", "uuid"] }
aws-sdk-s3 = "1"
tokio = { version = "1", features = ["fs"] }
async-trait = "0.1"
thiserror = "1"
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros"] }
tempfile = "3"
```

---

## 4. Schéma SQLite (`migrations/0001_initial.sql`)

```sql
CREATE TABLE IF NOT EXISTS boards (
    id          TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notes (
    id          TEXT PRIMARY KEY,
    note_type   TEXT NOT NULL,
    content     BLOB NOT NULL,
    thumbnail   BLOB,
    duration_ms INTEGER,
    color       TEXT NOT NULL DEFAULT '#FFFFFF',
    board_id    TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL DEFAULT 0,
    blob_key    TEXT NOT NULL,
    size        INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    id              TEXT PRIMARY KEY,
    identity_id     TEXT NOT NULL,
    pub_key_x25519  TEXT NOT NULL,
    pub_key_ed25519 TEXT NOT NULL,
    name            TEXT NOT NULL,
    last_seen       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS link_sessions (
    token               TEXT PRIMARY KEY,
    code                TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending',
    pub_key_initiator   TEXT NOT NULL,
    encrypted_symkey    BLOB,
    expires_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notes_board_id ON notes(board_id);
CREATE INDEX IF NOT EXISTS idx_notes_position ON notes(board_id, position);
```

`note_type` stocké en texte : `'text'` | `'voice'` | `'image'`.
`status` stocké en texte : `'pending'` | `'confirmed'` | `'expired'`.
Dates stockées en ISO 8601 UTC.

---

## 5. Couche DB

### 5.1 `db/mod.rs`

```rust
pub struct Db(SqlitePool);

impl Db {
    pub async fn connect(url: &str) -> Result<Self, StorageError>
    pub async fn migrate(&self) -> Result<(), StorageError>
}
```

`migrate()` appelle `sqlx::migrate!("./migrations")` — idempotent, appliqué au démarrage.

### 5.2 `db/notes.rs` — méthodes sur `Db`

```rust
pub async fn insert_note(&self, note: &Note) -> Result<(), StorageError>
pub async fn get_note(&self, id: Uuid) -> Result<Option<Note>, StorageError>
pub async fn list_notes(&self, board_id: Uuid) -> Result<Vec<Note>, StorageError>
pub async fn delete_note(&self, id: Uuid) -> Result<(), StorageError>
pub async fn update_note_position(&self, id: Uuid, position: i32) -> Result<(), StorageError>
pub async fn update_note_color(&self, id: Uuid, color: &str) -> Result<(), StorageError>
```

### 5.3 `db/boards.rs`

```rust
pub async fn insert_board(&self, board: &Board) -> Result<(), StorageError>
pub async fn get_board(&self, id: Uuid) -> Result<Option<Board>, StorageError>
pub async fn list_boards(&self, identity_id: Uuid) -> Result<Vec<Board>, StorageError>
pub async fn delete_board(&self, id: Uuid) -> Result<(), StorageError>
pub async fn update_board_name(&self, id: Uuid, name: &str) -> Result<(), StorageError>
pub async fn update_board_position(&self, id: Uuid, position: i32) -> Result<(), StorageError>
```

### 5.4 `db/devices.rs`

```rust
pub async fn insert_device(&self, device: &Device) -> Result<(), StorageError>
pub async fn get_device(&self, id: Uuid) -> Result<Option<Device>, StorageError>
pub async fn list_devices(&self, identity_id: Uuid) -> Result<Vec<Device>, StorageError>
pub async fn delete_device(&self, id: Uuid) -> Result<(), StorageError>
pub async fn touch_device(&self, id: Uuid) -> Result<(), StorageError>
```

`touch_device` met à jour `last_seen = now()`.

### 5.5 `db/links.rs`

```rust
pub async fn insert_link(&self, link: &LinkSession) -> Result<(), StorageError>
pub async fn get_link(&self, token: &str) -> Result<Option<LinkSession>, StorageError>
pub async fn confirm_link(&self, token: &str, encrypted_symkey: Vec<u8>) -> Result<(), StorageError>
pub async fn expire_link(&self, token: &str) -> Result<(), StorageError>
```

---

## 6. Blob storage

### 6.1 Trait `BlobStore` (`blobs/mod.rs`)

```rust
#[async_trait::async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, key: &str, data: &[u8]) -> Result<(), StorageError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
}
```

### 6.2 `LocalStore` (`blobs/local.rs`)

```rust
pub struct LocalStore { base_path: PathBuf }

impl LocalStore {
    pub fn new(base_path: impl Into<PathBuf>) -> Self
}
```

- `put`    → `tokio::fs::write(base_path/key, data)` (crée les répertoires parents si nécessaire)
- `get`    → `tokio::fs::read(base_path/key)` → `BlobNotFound` si absent
- `delete` → `tokio::fs::remove_file(base_path/key)`

### 6.3 `S3Store` (`blobs/s3.rs`)

```rust
pub struct S3Store {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl S3Store {
    pub async fn new(bucket: String, region: String, endpoint_url: Option<String>) -> Self
}
```

`endpoint_url` permet de cibler Cloudflare R2, MinIO, ou tout autre endpoint S3-compatible.

### 6.4 Sélection au runtime (dans `api`)

```rust
let store: Arc<dyn BlobStore> = match std::env::var("JOT_STORAGE").as_deref() {
    Ok("s3") => Arc::new(S3Store::new(bucket, region, endpoint_url).await),
    _        => Arc::new(LocalStore::new("~/.jot/blobs")),
};
```

---

## 7. Gestion d'erreurs (`error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("blob not found: {0}")]
    BlobNotFound(String),
    #[error("blob I/O error: {0}")]
    BlobIo(#[from] std::io::Error),
    #[error("S3 error: {0}")]
    S3(String),
}
```

---

## 8. Tests

### 8.1 DB (SQLite en mémoire)

```rust
async fn test_db() -> Db {
    let db = Db::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    db
}
```

Tests par module :
- **notes** : insert+get round-trip, list filtre par board_id, delete, get inexistant → None
- **boards** : insert+get, list filtre par identity_id, update_name, delete cascade notes
- **devices** : insert+get, list, touch_device met à jour last_seen, delete
- **links** : insert → confirm_link → status Confirmed, expire_link → status Expired

### 8.2 LocalStore (`tempfile::tempdir()`)

- put+get round-trip
- get sur clé inexistante → `StorageError::BlobNotFound`
- delete supprime le fichier

### 8.3 S3Store

Pas de test automatisé (nécessite credentials AWS/R2/MinIO) — hors scope CI.

---

## 9. Hors périmètre

- Pooling de connexions avancé (WAL mode, pool size) — defaults sqlx suffisants pour v1
- Chiffrement SQLite (SQLCipher) — les blobs sont déjà chiffrés côté client avant stockage
- Tests S3 automatisés — ajoutés si localstack est intégré en CI ultérieurement
