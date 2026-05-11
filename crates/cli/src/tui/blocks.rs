use crate::client::JotClient;
use crate::error::CliError;
use jot_core::models::Block;
use ratatui::{
    layout::Rect,
    prelude::*,
    widgets::{Block as TBlock, Borders, Paragraph},
};
use std::collections::HashMap;
use uuid::Uuid;

/// Fetch the block list for `note_id` and decrypt each block's content using
/// the existing per-note DEK derivation.
pub async fn load(
    client: &JotClient,
    note_id: Uuid,
    board_id: Uuid,
) -> Result<(Vec<Block>, HashMap<Uuid, String>), CliError> {
    let blocks = client.list_blocks(note_id).await?;
    let mut decrypted = HashMap::new();
    for b in &blocks {
        let plain = match client
            .decrypt_with_note_dek(board_id, note_id, &b.content)
            .await
        {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(_) => "<decryption failed>".to_string(),
        };
        decrypted.insert(b.id, plain);
    }
    Ok((blocks, decrypted))
}

pub fn render(f: &mut Frame, area: Rect, panel: &crate::tui::app::BlockPanel) {
    let mut lines: Vec<Line> = Vec::new();
    let by_parent = group_by_parent(&panel.blocks);
    walk(
        &by_parent,
        None,
        0,
        &panel.plaintexts,
        panel.cursor,
        0,
        &mut lines,
    );
    let p = Paragraph::new(lines).block(TBlock::default().borders(Borders::ALL).title("Blocks"));
    f.render_widget(p, area);
}

/// Flatten blocks in the same depth-first order used by the renderer, so a
/// linear cursor index in `BlockPanel` lines up with what the user sees.
pub fn flatten_depth_first(blocks: &[Block]) -> Vec<&Block> {
    let by_parent = group_by_parent(blocks);
    let mut out: Vec<&Block> = Vec::new();
    fn rec<'a>(
        m: &HashMap<Option<Uuid>, Vec<&'a Block>>,
        parent: Option<Uuid>,
        out: &mut Vec<&'a Block>,
    ) {
        if let Some(kids) = m.get(&parent) {
            for k in kids {
                out.push(*k);
                rec(m, Some(k.id), out);
            }
        }
    }
    rec(&by_parent, None, &mut out);
    out
}

fn group_by_parent(blocks: &[Block]) -> HashMap<Option<Uuid>, Vec<&Block>> {
    let mut m: HashMap<Option<Uuid>, Vec<&Block>> = HashMap::new();
    for b in blocks {
        m.entry(b.parent_block_id).or_default().push(b);
    }
    for v in m.values_mut() {
        v.sort_by(|a, b| {
            a.position
                .partial_cmp(&b.position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    m
}

fn walk<'a>(
    by_parent: &HashMap<Option<Uuid>, Vec<&'a Block>>,
    parent: Option<Uuid>,
    depth: usize,
    pts: &HashMap<Uuid, String>,
    cursor: usize,
    mut idx: usize,
    out: &mut Vec<Line<'a>>,
) -> usize {
    if let Some(kids) = by_parent.get(&parent) {
        for k in kids {
            let prefix = "  ".repeat(depth);
            let style = if idx == cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let text = pts.get(&k.id).cloned().unwrap_or_default();
            out.push(Line::from(Span::styled(
                format!("{}\u{2022} {}", prefix, text),
                style,
            )));
            idx += 1;
            idx = walk(by_parent, Some(k.id), depth + 1, pts, cursor, idx, out);
        }
    }
    idx
}
