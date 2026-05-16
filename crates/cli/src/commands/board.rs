use crate::client::JotClient;
use crate::error::CliError;
use clap::Subcommand;
use uuid::Uuid;

#[derive(Subcommand)]
pub enum BoardCmd {
    /// Rename a board
    Rename { id: Uuid, name: String },
    /// Delete a board (and all its notes)
    Delete { id: Uuid },
    /// Move a board to a new position in the sidebar
    Move {
        id: Uuid,
        #[arg(long)]
        position: i32,
    },
    /// Reorder notes inside a board (note_id:position note_id:position ...)
    ReorderNotes {
        board: Uuid,
        /// pairs of note_id:position
        items: Vec<String>,
    },
}

pub async fn run(cmd: BoardCmd) -> Result<(), CliError> {
    let client = JotClient::from_config();
    match cmd {
        BoardCmd::Rename { id, name } => {
            client.rename_board(id, &name).await?;
            println!("board renamed");
        }
        BoardCmd::Delete { id } => {
            client.delete_board(id).await?;
            println!("board deleted");
        }
        BoardCmd::Move { id, position } => {
            client
                .patch_json(
                    &format!("/boards/{}", id),
                    &serde_json::json!({ "position": position }),
                )
                .await?;
            println!("board moved to position {position}");
        }
        BoardCmd::ReorderNotes { board, items } => {
            let mut payload = Vec::new();
            for pair in items {
                let (id_str, pos_str) = pair.split_once(':').ok_or_else(|| {
                    CliError::Config(format!("bad item '{pair}', expected uuid:position"))
                })?;
                let note_id: Uuid = id_str
                    .parse()
                    .map_err(|e| CliError::Config(format!("bad uuid {id_str}: {e}")))?;
                let position: i32 = pos_str
                    .parse()
                    .map_err(|e| CliError::Config(format!("bad position {pos_str}: {e}")))?;
                payload.push(serde_json::json!({ "note_id": note_id, "position": position }));
            }
            client
                .patch_json(
                    &format!("/boards/{}/reorder", board),
                    &serde_json::Value::Array(payload),
                )
                .await?;
            println!("notes reordered");
        }
    }
    Ok(())
}
