use crate::client::JotClient;
use crate::error::CliError;

pub async fn run() -> Result<(), CliError> {
    let client = JotClient::from_config();
    let boards = client.get_boards().await?;
    let mut total_notes = 0usize;
    let mut per_board: Vec<(String, usize)> = Vec::new();
    for b in &boards {
        let notes = client.get_notes(b.id).await?;
        total_notes += notes.len();
        per_board.push((b.name.clone(), notes.len()));
    }
    println!("Boards : {}", boards.len());
    println!("Notes  : {}", total_notes);
    println!();
    println!("Per board:");
    for (name, n) in per_board {
        println!("  {n:>5}  {name}");
    }
    Ok(())
}
