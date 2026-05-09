# Spec : crate api — Axum REST + auth JWT + WebSocket

**Date :** 2026-05-10
**Sous-projet :** 4 / 8
**Statut :** approuvé

---

## 1. Périmètre

Implémenter `crate api` : serveur HTTP Axum exposant l'API REST complète de jot.
- Auth JWT EdDSA (jsonwebtoken + ed25519-dalek)
- Endpoints : auth, link, notes, boards, devices, health
- WebSocket temps réel (broadcast d'événements)
- Tests d'intégration avec `axum::test` + SQLite en mémoire

Dépend de `jot-core`, `storage`. Remplace le stub existant dans `crates/api/`.

---

## 2. Structure

```
crates/api/
├── Cargo.toml
└── src/
    ├── lib.rs              # build_router() → Router
    ├── error.rs            # ApiError → IntoResponse
    ├── state.rs            # AppState
    ├── auth/
    │   ├── mod.rs          # sign_token, verify_token, DeviceClaims
    │   └── middleware.rs   # AuthenticatedDevice extractor
    └── routes/
        ├── mod.rs
        ├── health.rs       # GET /health
        ├── auth.rs         # POST /auth/register, POST /auth/device
        ├── link.rs         # POST /link/init, GET /link/{token}, POST /link/confirm, GET /link/status/{token}
        ├── notes.rs        # CRUD notes + blob upload/download
        ├── boards.rs       # CRUD boards + reorder
        ├── devices.rs      # list, delete, rename
        └── ws.rs           # GET /ws WebSocket
```

---

## 3. Dépendances

```toml
[dependencies]
jot-core = { path = "../core" }
storage = { path = "../storage" }
axum = { version = "0.7", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
jsonwebtoken = "9"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
rand = "0.8"
async-trait = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
tower = { version = "0.4", features = ["util"] }
tempfile = "3"
```

---

## 4. AppState (`state.rs`)

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub blobs: Arc<dyn BlobStore>,
    pub server_signing_key: Vec<u8>,    // Ed25519 seed (32 bytes)
    pub server_verifying_key: Vec<u8>,  // Ed25519 public key (32 bytes)
    pub ws_tx: broadcast::Sender<WsEvent>,
}
```

La clé Ed25519 du serveur est générée au premier démarrage et stockée dans `~/.jot/server_key`. En test, une clé éphémère est générée en mémoire.

`ws_tx` est un `tokio::sync::broadcast::Sender<WsEvent>` — capacité 128 — partagé entre tous les handlers pour broadcaster les mutations.

---

## 5. Auth JWT (`auth/mod.rs`)

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceClaims {
    pub sub: String,           // device_id (UUID)
    pub identity_id: String,
    pub exp: usize,            // now + 30 jours
    pub iat: usize,
}

pub fn sign_token(claims: &DeviceClaims, signing_key: &[u8]) -> Result<String, ApiError>
pub fn verify_token(token: &str, verifying_key: &[u8]) -> Result<DeviceClaims, ApiError>
// Algorithme : Algorithm::EdDSA
```

**Middleware (`auth/middleware.rs`)** — extracteur Axum :
```rust
pub struct AuthenticatedDevice(pub DeviceClaims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedDevice
where S: Send + Sync
{
    type Rejection = ApiError;
    // Extrait Authorization: Bearer <token>
    // Appelle verify_token() avec server_verifying_key depuis extensions
    // → 401 si absent ou invalide
}
```

---

## 6. Endpoints

### 6.1 Health

`GET /health` → `200 { "status": "ok" }` — pas d'auth.

### 6.2 Auth (`routes/auth.rs`)

**`POST /auth/register`** — pas d'auth requise :
```
Body: { uuid: String, pseudo: String, avatar: String }
→ 201 Created (identité stockée dans une DashMap<Uuid, Identity> dans AppState pour v1)
```

**`POST /auth/device`** — pas d'auth requise :
```
Body: { device_id, identity_id, pub_key_x25519, pub_key_ed25519, name }
→ db.insert_device()
→ JWT signé { sub: device_id, identity_id, exp: now+30j }
→ 201 { "token": "<jwt>" }
```

### 6.3 Link (`routes/link.rs`) — pas d'auth

**`POST /link/init`** :
```
→ token = UUID aléatoire, code = 4 chiffres aléatoires
→ expires_at = now + 5 min
→ db.insert_link()
→ 201 { token, code, expires_at }
```

**`GET /link/{token}`** :
```
→ db.get_link(token) → 404 si absent, 200 { token, code, status, expires_at }
```

**`POST /link/confirm`** :
```
Body: { token, encrypted_symkey: String (base64) }
→ db.confirm_link(token, base64::decode(encrypted_symkey))
→ 200 { "status": "confirmed" }
```

**`GET /link/status/{token}`** :
```
→ db.get_link(token) → { status: "pending"|"confirmed"|"expired" }
```

### 6.4 Notes (`routes/notes.rs`) — auth requise

| Endpoint | Action |
|---|---|
| `GET /notes?board_id=<uuid>` | `db.list_notes(board_id)` — retourne metadata sans `content` |
| `POST /notes` | Body: NoteMetadata JSON → `db.insert_note()` → 201 |
| `GET /notes/{id}` | `db.get_note(id)` → 404 si absent |
| `DELETE /notes/{id}` | `db.delete_note(id)` + `blobs.delete(blob_key)` → 204 |
| `PATCH /notes/{id}` | Body: `{ position?, color? }` → update partiel → 200 |
| `GET /notes/{id}/blob` | `blobs.get(blob_key)` → `application/octet-stream` |
| `PUT /notes/{id}/blob` | Body bytes → `blobs.put(blob_key, data)` → 200 |

Après chaque mutation (POST, DELETE, PATCH, PUT blob), envoie `WsEvent` via `ws_tx.send()`.

### 6.5 Boards (`routes/boards.rs`) — auth requise

| Endpoint | Action |
|---|---|
| `GET /boards` | `db.list_boards(identity_id)` |
| `POST /boards` | `db.insert_board()` → 201 |
| `PATCH /boards/{id}` | Body: `{ name?, position? }` → update partiel |
| `DELETE /boards/{id}` | `db.delete_board(id)` → 204 |
| `PATCH /boards/{id}/reorder` | Body: `[{ note_id, position }]` → batch `update_note_position` |

### 6.6 Devices (`routes/devices.rs`) — auth requise

| Endpoint | Action |
|---|---|
| `GET /devices` | `db.list_devices(identity_id)` |
| `DELETE /devices/{id}` | `db.delete_device(id)` → 204 |
| `POST /devices/{id}/rename` | Body: `{ name }` → update name → 200 |

### 6.7 WebSocket (`routes/ws.rs`)

`GET /ws?token=<jwt>` — authentification via query param (les WS ne supportent pas les headers custom dans les browsers).

```rust
#[derive(Clone, Serialize)]
pub enum WsEvent {
    NoteUpdated { id: Uuid },
    NoteDeleted { id: Uuid },
    BoardUpdated { id: Uuid },
}
```

- Upgrade HTTP → WS
- Vérifie le JWT via `verify_token(token, verifying_key)`
- Souscrit au `broadcast::Receiver`
- Loop : reçoit événements → sérialise JSON → envoie au client WS
- Fermeture propre sur disconnect client

---

## 7. Gestion d'erreurs (`error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("storage error: {0}")]
    Storage(#[from] storage::StorageError),
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError { ... }
// NotFound → 404, Unauthorized → 401, BadRequest → 400, _ → 500
// Body: { "error": "<message>" }
```

---

## 8. Tests (`axum::test`)

```rust
async fn test_app() -> Router {
    let db = Db::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    let dir = tempdir().unwrap();
    let blobs = Arc::new(LocalStore::new(dir.path()));
    let (signing_key, verifying_key) = generate_device_keypair(); // from jot-core
    let state = AppState::new(db, blobs, signing_key.to_bytes().to_vec(), verifying_key.to_bytes().to_vec());
    build_router(state)
}
```

**Scénarios couverts :**
- `GET /health` → 200
- `POST /auth/device` → 201 + JWT parseable
- `GET /notes` sans token → 401
- `GET /notes` avec token valide → 200
- `POST /notes` + `PUT /notes/{id}/blob` + `GET /notes/{id}/blob` → round-trip
- `DELETE /notes/{id}` → 204 + blob absent du store
- Flux link : `POST /link/init` → `POST /link/confirm` → `GET /link/status/{token}` = confirmed
- `GET /ws?token=<jwt>` → 101 Switching Protocols

---

## 9. Hors périmètre

- `rust-embed` SPA web → sous-projet 6 (intégration)
- Feature flag `pro-ai` / LanceDB → sous-projet 7
- Rate limiting, CORS fine-grained → production concern, non bloquant pour v1
- Identités stockées en DashMap en mémoire (pas en DB) pour v1 — suffisant jusqu'au SP6
