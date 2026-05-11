use crate::models::{LinkKind, TargetKind};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedLink {
    pub target_kind: TargetKind,
    pub target_id: String,
    pub link_kind: LinkKind,
}

static PAGE_RE: OnceLock<Regex> = OnceLock::new();
static BLOCK_RE: OnceLock<Regex> = OnceLock::new();
static EMBED_RE: OnceLock<Regex> = OnceLock::new();
static TAG_RE: OnceLock<Regex> = OnceLock::new();

fn page_re()  -> &'static Regex { PAGE_RE.get_or_init(|| Regex::new(r"\[\[([^\]\n]+?)\]\]").unwrap()) }
fn embed_re() -> &'static Regex { EMBED_RE.get_or_init(|| Regex::new(r"!\(\(([0-9a-fA-F-]{36})\)\)").unwrap()) }
fn block_re() -> &'static Regex { BLOCK_RE.get_or_init(|| Regex::new(r"\(\(([0-9a-fA-F-]{36})\)\)").unwrap()) }
fn tag_re()   -> &'static Regex { TAG_RE.get_or_init(|| Regex::new(r"(?:^|\s)#([A-Za-z0-9_\-]+)").unwrap()) }

pub fn extract_links(markdown: &str, title_to_id: &std::collections::HashMap<String, String>) -> Vec<ExtractedLink> {
    let mut out = Vec::new();
    for cap in embed_re().captures_iter(markdown) {
        out.push(ExtractedLink { target_kind: TargetKind::Block, target_id: cap[1].to_string(), link_kind: LinkKind::BlockEmbed });
    }
    for cap in block_re().captures_iter(markdown) {
        let id = cap[1].to_string();
        if out.iter().any(|l| l.target_id == id && l.link_kind == LinkKind::BlockEmbed) { continue; }
        out.push(ExtractedLink { target_kind: TargetKind::Block, target_id: id, link_kind: LinkKind::BlockRef });
    }
    for cap in page_re().captures_iter(markdown) {
        let title = cap[1].trim().to_string();
        let id = title_to_id.get(&title.to_lowercase()).cloned().unwrap_or(title);
        out.push(ExtractedLink { target_kind: TargetKind::Note, target_id: id, link_kind: LinkKind::PageRef });
    }
    for cap in tag_re().captures_iter(markdown) {
        out.push(ExtractedLink { target_kind: TargetKind::Tag, target_id: cap[1].to_string(), link_kind: LinkKind::Tag });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn detects_page_ref() {
        let map = HashMap::from([("hello".to_string(), "note-uuid".to_string())]);
        let out = extract_links("see [[Hello]] today", &map);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].link_kind, LinkKind::PageRef);
        assert_eq!(out[0].target_id, "note-uuid");
    }

    #[test]
    fn unknown_page_returns_raw_title() {
        let out = extract_links("see [[Unknown]] today", &HashMap::new());
        assert_eq!(out[0].target_id, "Unknown");
    }

    #[test]
    fn detects_block_ref_vs_embed() {
        let md = "ref ((550e8400-e29b-41d4-a716-446655440000)) embed !((550e8400-e29b-41d4-a716-446655440001))";
        let out = extract_links(md, &HashMap::new());
        assert_eq!(out.len(), 2);
        let kinds: Vec<_> = out.iter().map(|l| l.link_kind).collect();
        assert!(kinds.contains(&LinkKind::BlockRef));
        assert!(kinds.contains(&LinkKind::BlockEmbed));
    }

    #[test]
    fn detects_tag() {
        let out = extract_links("status #wip and #done-2025", &HashMap::new());
        let tags: Vec<_> = out.iter().filter(|l| l.link_kind == LinkKind::Tag).map(|l| l.target_id.clone()).collect();
        assert_eq!(tags, vec!["wip", "done-2025"]);
    }
}
