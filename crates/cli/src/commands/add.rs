use crate::client::JotClient;
use crate::error::CliError;
use atty::Stream;
use std::io::Read;
use uuid::Uuid;

pub async fn run(text: Vec<String>, board: Option<Uuid>) -> Result<(), CliError> {
    let content = if !text.is_empty() {
        text.join(" ")
    } else if !atty::is(Stream::Stdin) {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf.trim().to_string()
    } else {
        read_from_editor()?
    };

    if content.is_empty() {
        eprintln!("Note content is empty — aborting.");
        return Ok(());
    }

    let client = JotClient::from_config();

    let board_id = match board {
        Some(id) => id,
        None => {
            let boards = client.get_boards().await?;
            boards
                .first()
                .map(|b| b.id)
                .ok_or_else(|| CliError::Server("no boards found — create one first".into()))?
        }
    };

    let note_id = client.create_note(board_id, &content).await?;
    println!("Note created: {}", note_id);
    Ok(())
}

fn read_from_editor() -> Result<String, CliError> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let path = std::env::temp_dir().join(format!("jot-{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&path, "")?;
    std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| CliError::Config(format!("failed to open editor '{}': {}", editor, e)))?;
    let content = std::fs::read_to_string(&path)?;
    let _ = std::fs::remove_file(&path);
    Ok(content.trim().to_string())
}
