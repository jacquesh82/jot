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
