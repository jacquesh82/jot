use crate::client::JotClient;
use crate::error::CliError;
use jot_core::models::BlockType;

pub async fn run(tag: Option<String>) -> Result<(), CliError> {
    let client = JotClient::from_config();
    let boards = client.get_boards().await?;
    let mut total = 0usize;
    for b in &boards {
        let notes = client.get_notes(b.id).await?;
        for n in notes {
            if n.note_type != "text" || n.schema_version < 1 {
                continue;
            }
            let blocks = match client.list_blocks(n.id).await {
                Ok(v) => v,
                Err(_) => continue,
            };
            for blk in blocks {
                if !matches!(blk.block_type, BlockType::Todo) {
                    continue;
                }
                let plain = match client.decrypt_with_note_dek(b.id, n.id, &blk.content).await {
                    Ok(p) => String::from_utf8_lossy(&p).to_string(),
                    Err(_) => continue,
                };
                if let Some(t) = &tag {
                    if !plain.contains(&format!("#{t}")) {
                        continue;
                    }
                }
                let checked = plain.starts_with("[x]") || plain.starts_with("[X]");
                let mark = if checked { "[x]" } else { "[ ]" };
                println!("  {mark} {plain}  ({})", &blk.id.to_string()[..8]);
                total += 1;
            }
        }
    }
    println!();
    println!("Total: {total}");
    Ok(())
}
