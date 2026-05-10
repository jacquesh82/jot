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
    /// Create a new object (board, …)
    New {
        #[command(subcommand)]
        what: commands::new::NewCommand,
    },
    /// List notes (default) or boards (--boards)
    List {
        #[arg(long)]
        board: Option<uuid::Uuid>,
        #[arg(long, help = "List boards instead of notes")]
        boards: bool,
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
        Command::New { what } => commands::new::run(what).await,
        Command::List { board, boards } => commands::list::run(board, boards).await,
        Command::Serve { port } => commands::serve::run(port).await,
        Command::Tui => tui::run_tui().await,
    }
}
