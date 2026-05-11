// Implementation in Task 7.
use crate::models::{LinkKind, TargetKind};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedLink {
    pub target_kind: TargetKind,
    pub target_id: String,
    pub link_kind: LinkKind,
}

pub fn extract_links(_markdown: &str, _title_to_id: &HashMap<String, String>) -> Vec<ExtractedLink> {
    Vec::new()
}
