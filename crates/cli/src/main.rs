mod client;
mod commands;
mod config;
mod error;
mod tui;

use clap::{Parser, Subcommand};
use error::CliError;

#[derive(Parser)]
#[command(name = "jot", about = "Encrypted note-taking CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a note (reads args, stdin, or $EDITOR)
    Add {
        text: Vec<String>,
        #[arg(long)]
        board: Option<uuid::Uuid>,
    },
    /// List notes in a board
    List {
        #[arg(long)]
        board: Option<uuid::Uuid>,
    },
    /// Manage boards (list, create)
    Boards {
        #[command(subcommand)]
        cmd: commands::boards::BoardsCommand,
    },
    /// Start the API server and register this device
    Serve {
        #[arg(long, default_value = "3000")]
        port: u16,
    },
    /// Launch the TUI
    Tui,
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Add { text, board } => commands::add::run(text, board).await,
        Command::List { board } => commands::list::run(board).await,
        Command::Boards { cmd } => commands::boards::run(cmd).await,
        Command::Serve { port } => commands::serve::run(port).await,
        Command::Tui => tui::run_tui().await,
    }
}
