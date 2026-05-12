use serde::{Deserialize, Serialize};
use std::sync::Arc;
use storage::{BlobStore, Db};
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WsEvent {
    NoteUpdated { id: String },
    NoteDeleted { id: String },
    BoardUpdated { id: String },
    BlockCreated { note_id: String, block_id: String },
    BlockUpdated { note_id: String, block_id: String },
    BlockMoved { note_id: String, block_id: String },
    BlockDeleted { note_id: String, block_id: String },
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub blobs: Arc<dyn BlobStore>,
    pub signing_key_pem: String,
    pub verifying_key_pem: String,
    pub ws_tx: broadcast::Sender<WsEvent>,
    pub open_registration: bool,
    pub schema_version: i64,
    /// Raw 32-byte X25519 identity private key — served to the SPA so CLI and web share the same key pair.
    pub identity_privkey: [u8; 32],
}

impl AppState {
    pub fn new(
        db: Db,
        blobs: Arc<dyn BlobStore>,
        signing_key_pem: String,
        verifying_key_pem: String,
        identity_privkey: [u8; 32],
    ) -> Self {
        let (ws_tx, _) = broadcast::channel(128);
        Self {
            db: Arc::new(db),
            blobs,
            signing_key_pem,
            verifying_key_pem,
            ws_tx,
            open_registration: false,
            schema_version: 0,
            identity_privkey,
        }
    }

    pub fn with_open_registration(mut self, enabled: bool) -> Self {
        self.open_registration = enabled;
        self
    }

    pub fn with_schema_version(mut self, version: i64) -> Self {
        self.schema_version = version;
        self
    }
}
