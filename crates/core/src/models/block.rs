use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockType { Text, Heading, Todo, Quote, Code, Embed, Divider }

impl BlockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Text => "text",
            BlockType::Heading => "heading",
            BlockType::Todo => "todo",
            BlockType::Quote => "quote",
            BlockType::Code => "code",
            BlockType::Embed => "embed",
            BlockType::Divider => "divider",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "heading" => BlockType::Heading,
            "todo" => BlockType::Todo,
            "quote" => BlockType::Quote,
            "code" => BlockType::Code,
            "embed" => BlockType::Embed,
            "divider" => BlockType::Divider,
            _ => BlockType::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub note_id: Uuid,
    pub parent_block_id: Option<Uuid>,
    pub position: f64,
    pub block_type: BlockType,
    pub content: Vec<u8>,
    pub metadata: Option<Vec<u8>>,
    pub collapsed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind { Note, Block, Tag }

impl TargetKind {
    pub fn as_str(&self) -> &'static str {
        match self { TargetKind::Note => "note", TargetKind::Block => "block", TargetKind::Tag => "tag" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s { "note" => Some(Self::Note), "block" => Some(Self::Block), "tag" => Some(Self::Tag), _ => None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind { PageRef, BlockRef, BlockEmbed, Tag }

impl LinkKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkKind::PageRef => "page_ref",
            LinkKind::BlockRef => "block_ref",
            LinkKind::BlockEmbed => "block_embed",
            LinkKind::Tag => "tag",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "page_ref" => Some(Self::PageRef),
            "block_ref" => Some(Self::BlockRef),
            "block_embed" => Some(Self::BlockEmbed),
            "tag" => Some(Self::Tag),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockLink {
    pub id: Uuid,
    pub source_block_id: Uuid,
    pub target_kind: TargetKind,
    pub target_id: String,
    pub link_kind: LinkKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub identity_id: Uuid,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn block_type_round_trip() {
        for s in ["text","heading","todo","quote","code","embed","divider"] {
            assert_eq!(BlockType::from_str(s).as_str(), s);
        }
    }
    #[test]
    fn link_kind_round_trip() {
        for s in ["page_ref","block_ref","block_embed","tag"] {
            assert_eq!(LinkKind::from_str(s).unwrap().as_str(), s);
        }
    }
    #[test]
    fn unknown_block_type_defaults_to_text() {
        assert!(matches!(BlockType::from_str("nonsense"), BlockType::Text));
    }
}
