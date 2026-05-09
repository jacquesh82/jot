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
