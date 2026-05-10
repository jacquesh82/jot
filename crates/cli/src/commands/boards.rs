use crate::client::JotClient;
use crate::error::CliError;

pub async fn run() -> Result<(), CliError> {
    let client = JotClient::from_config();
    let boards = client.get_boards().await?;
    if boards.is_empty() {
        println!("(no boards)");
    } else {
        for b in &boards {
            println!("{}\t{}\t(pos:{})", b.id, b.name, b.position);
        }
    }
    Ok(())
}
