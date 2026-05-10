use crate::client::JotClient;
use crate::error::CliError;
use uuid::Uuid;

pub async fn run(board: Option<Uuid>, boards: bool, devices: bool) -> Result<(), CliError> {
    let client = JotClient::from_config();

    if devices {
        let ds = client.get_devices().await?;
        if ds.is_empty() {
            println!("(no devices)");
        } else {
            for d in &ds {
                println!("{}\t{}\tlast_seen:{}", d.id, d.name, d.last_seen);
            }
        }
        return Ok(());
    }

    if boards {
        let bs = client.get_boards().await?;
        if bs.is_empty() {
            println!("(no boards — run: jot new board \"My Board\")");
        } else {
            for b in &bs {
                println!("{}\t{}", b.id, b.name);
            }
        }
        return Ok(());
    }

    let board_id = match board {
        Some(id) => id,
        None => {
            let bs = client.get_boards().await?;
            bs.first()
                .map(|b| b.id)
                .ok_or_else(|| CliError::Server("no boards — run: jot new board \"My Board\"".into()))?
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
