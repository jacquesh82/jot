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
    #[serde(default)]
    pub title: Option<Vec<u8>>,
    #[serde(default)]
    pub is_journal: bool,
    #[serde(default)]
    pub journal_date: Option<String>,
    #[serde(default)]
    pub schema_version: i32,
}
