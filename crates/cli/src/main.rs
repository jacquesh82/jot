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
    /// Approve a pending device link request (from the web UI)
    Link {
        /// Link token shown in the browser
        token: String,
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
        /// Allow anyone to create an account without an invite token
        #[arg(long)]
        open_registration: bool,
    },
    /// Create an invite token (anyone with the link can register)
    Invite {
        /// Optional label to identify this invite
        #[arg(long)]
        label: Option<String>,
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
        Command::Link { token } => commands::link::run(&token).await,
        Command::Read { id } => commands::read::run(id).await,
        Command::Whoami => commands::whoami::run().await,
        Command::Serve { port, open_registration } => commands::serve::run(port, open_registration).await,
        Command::Invite { label } => commands::invite::run(label).await,
        Command::Tui => tui::run_tui().await,
    }
}
