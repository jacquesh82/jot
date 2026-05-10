use crate::client::JotClient;
use crate::error::CliError;
use uuid::Uuid;

pub async fn run(board: Option<Uuid>) -> Result<(), CliError> {
    let client = JotClient::from_config();

    let board_id = match board {
        Some(id) => id,
        None => {
            let boards = client.get_boards().await?;
            boards
                .first()
                .map(|b| b.id)
                .ok_or_else(|| CliError::Server("no boards found".into()))?
        }
    };

    let notes = client.get_notes(board_id).await?;
    if notes.is_empty() {
        println!("(no notes in this board)");
    } else {
        for note in &notes {
            println!("{}\t{}\tpos:{}", note.id, note.note_type, note.position);
        }
    }
    Ok(())
}
