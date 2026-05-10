use crate::client::JotClient;
use crate::error::CliError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum NewCommand {
    /// Create a new board
    Board {
        /// Board name
        name: String,
    },
}

pub async fn run(cmd: NewCommand) -> Result<(), CliError> {
    let client = JotClient::from_config();
    match cmd {
        NewCommand::Board { name } => {
            let board = client.create_board(&name).await?;
            println!("Board created: {} ({})", board.name, board.id);
        }
    }
    Ok(())
}
