use crate::client::JotClient;
use crate::error::CliError;
use clap::Subcommand;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Subcommand, Debug)]
pub enum BlockCmd {
    /// Add a block to a note
    Add {
        #[arg(long)]
        note: Uuid,
        #[arg(long)]
        parent: Option<Uuid>,
        #[arg(long)]
        position: Option<f64>,
        #[arg(long, default_value = "text")]
        r#type: String,
        #[arg(long)]
        text: String,
    },
    /// List blocks of a note (flat or tree)
    List {
        #[arg(long)]
        note: Uuid,
        #[arg(long, default_value_t = false)]
        tree: bool,
    },
    /// Show a single block's content
    Show { id: Uuid },
    /// Open the block content in $EDITOR
    Edit { id: Uuid },
    /// Move a block to a new parent / position
    Move {
        id: Uuid,
        #[arg(long)]
        to: Option<Uuid>,
        #[arg(long)]
        position: f64,
    },
    /// Indent a block under its previous sibling
    Indent { id: Uuid },
    /// Outdent a block (move under its grandparent)
    Outdent { id: Uuid },
    /// Delete a block
    Delete { id: Uuid },
    /// Print the (( )) reference syntax for a block id
    Ref { id: Uuid },
    /// List backlinks pointing at this block
    Backlinks { id: Uuid },
    /// Migrate legacy notes into block-structured form (Task 14)
    Migrate {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        note: Option<Uuid>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

pub async fn run(cmd: BlockCmd) -> Result<(), CliError> {
    let client = JotClient::from_config();
    match cmd {
        BlockCmd::Add {
            note,
            parent,
            position,
            r#type,
            text,
        } => {
            let b = client
                .create_block(note, parent, position, &r#type, text.as_bytes(), None)
                .await?;
            println!("{}", b.id);
        }
        BlockCmd::List { note, tree } => {
            let blocks = client.list_blocks(note).await?;
            if tree {
                print_tree(&blocks);
            } else {
                for b in &blocks {
                    println!("{} [{}] pos={}", b.id, b.block_type.as_str(), b.position);
                }
            }
        }
        BlockCmd::Show { id } => {
            let b = client.get_block(id).await?;
            println!("{}", String::from_utf8_lossy(&b.content));
        }
        BlockCmd::Edit { id } => {
            let current = client.get_block(id).await?;
            let edited = edit_in_editor(&String::from_utf8_lossy(&current.content))?;
            client
                .patch_block(id, None, Some(edited.as_bytes()))
                .await?;
            println!("Block updated.");
        }
        BlockCmd::Move { id, to, position } => {
            client.move_block(id, to, position).await?;
        }
        BlockCmd::Indent { id } => {
            client.indent_block(id).await?;
        }
        BlockCmd::Outdent { id } => {
            client.outdent_block(id).await?;
        }
        BlockCmd::Delete { id } => {
            client.delete_block(id).await?;
        }
        BlockCmd::Ref { id } => {
            println!("(({}))", id);
        }
        BlockCmd::Backlinks { id } => {
            let body = client.block_backlinks(id).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&body)
                    .unwrap_or_else(|_| body.to_string())
            );
        }
        BlockCmd::Migrate { .. } => {
            return Err(CliError::Config(
                "not yet implemented — see Task 14".into(),
            ));
        }
    }
    Ok(())
}

fn print_tree(blocks: &[jot_core::models::Block]) {
    let mut by_parent: HashMap<Option<Uuid>, Vec<&jot_core::models::Block>> = HashMap::new();
    for b in blocks {
        by_parent.entry(b.parent_block_id).or_default().push(b);
    }
    for kids in by_parent.values_mut() {
        kids.sort_by(|a, b| {
            a.position
                .partial_cmp(&b.position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    walk(&by_parent, None, 0);
}

fn walk(
    by_parent: &HashMap<Option<Uuid>, Vec<&jot_core::models::Block>>,
    parent: Option<Uuid>,
    depth: usize,
) {
    if let Some(kids) = by_parent.get(&parent) {
        for k in kids {
            println!(
                "{}{} {}",
                "  ".repeat(depth),
                k.id,
                String::from_utf8_lossy(&k.content)
            );
            walk(by_parent, Some(k.id), depth + 1);
        }
    }
}

/// Open $EDITOR (or $VISUAL, falling back to vi) on `initial` and return the trimmed result.
fn edit_in_editor(initial: &str) -> Result<String, CliError> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let path = std::env::temp_dir().join(format!("jot-block-{}.md", Uuid::new_v4()));
    std::fs::write(&path, initial)?;
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| CliError::Config(format!("failed to launch editor {editor}: {e}")))?;
    if !status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(CliError::Config(format!(
            "editor {editor} exited with status {status}"
        )));
    }
    let content = std::fs::read_to_string(&path)?;
    let _ = std::fs::remove_file(&path);
    Ok(content.trim_end().to_string())
}
