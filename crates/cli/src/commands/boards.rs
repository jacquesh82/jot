use crate::client::JotClient;
use crate::error::CliError;
use crate::t;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum BoardsCommand {
    /// List all boards
    List,
    /// Create a new board
    New {
        /// Board name
        name: String,
    },
}

pub async fn run(cmd: BoardsCommand) -> Result<(), CliError> {
    let client = JotClient::from_config();
    match cmd {
        BoardsCommand::List => {
            let boards = client.get_boards().await?;
            if boards.is_empty() {
                println!("{}", t!("cmd.list.noBoards"));
            } else {
                for b in &boards {
                    println!("{}\t{}", b.id, b.name);
                }
            }
        }
        BoardsCommand::New { name } => {
            let board = client.create_board(&name).await?;
            println!("{}", t!("cmd.board.created", "name" => board.name, "id" => board.id));
        }
    }
    Ok(())
}
