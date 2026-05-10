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
    /// List notes (default), boards, or devices
    List {
        #[arg(long)]
        board: Option<uuid::Uuid>,
        #[arg(long, help = "List boards instead of notes")]
        boards: bool,
        #[arg(long, help = "List registered devices")]
        devices: bool,
    },
    /// Read the content of a note
    Read {
        /// Note ID
        id: uuid::Uuid,
    },
    /// Show current identity and device
    Whoami,
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
        Command::List { board, boards, devices } => commands::list::run(board, boards, devices).await,
        Command::Read { id } => commands::read::run(id).await,
        Command::Whoami => commands::whoami::run().await,
        Command::Serve { port } => commands::serve::run(port).await,
        Command::Tui => tui::run_tui().await,
    }
}
