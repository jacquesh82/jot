mod client;
mod commands;
mod config;
mod error;
mod i18n;
mod identity;
mod tui;

use clap::{Parser, Subcommand};
use error::CliError;

#[derive(Parser)]
#[command(name = "jot", about = "Encrypted note-taking CLI")]
struct Cli {
    /// Language: en, fr, es, de (overrides config and LANG env var)
    #[arg(long, global = true, value_name = "LANG")]
    lang: Option<String>,
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
    /// Share a note with another identity
    Share {
        /// Note ID
        note: uuid::Uuid,
        /// Recipient friendly name or UUID
        with: String,
        /// Permission level: read | write | delete (default: read)
        #[arg(long, default_value = "read")]
        permission: String,
    },
    /// Revoke a share and rotate the note's DEK
    Revoke {
        /// Note ID
        note: uuid::Uuid,
        /// Identity ID to revoke
        identity: String,
    },
    /// List who a note is shared with
    Shares {
        /// Note ID
        note: uuid::Uuid,
    },
    /// Share a board with another identity (propagates note DEKs)
    BoardShare {
        /// Board ID
        board: uuid::Uuid,
        /// Recipient friendly name or UUID
        with: String,
    },
    /// Revoke a board share (clears note DEKs for the ex-member)
    BoardRevoke {
        /// Board ID
        board: uuid::Uuid,
        /// Identity ID to revoke
        identity: String,
    },
    /// Apply pending database schema migrations
    Migrate,
    /// Block-level operations
    Block {
        #[command(subcommand)]
        cmd: commands::block::BlockCmd,
    },
    /// Note-level operations (title, …)
    Note {
        #[command(subcommand)]
        cmd: commands::note::NoteCmd,
    },
    /// Launch the TUI
    Tui,
    /// Permanently delete this account and all associated data
    DeleteAccount,
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    let config = config::Config::load();
    let lang = cli
        .lang
        .clone()
        .or_else(|| config.lang.clone())
        .or_else(|| {
            std::env::var("LANG")
                .ok()
                .or_else(|| std::env::var("LC_ALL").ok())
                .map(|v| v[..2.min(v.len())].to_lowercase())
        })
        .unwrap_or_else(|| "en".to_string());
    crate::i18n::init(&lang);
    match cli.command {
        Command::Add { text, board } => commands::add::run(text, board).await,
        Command::New { what } => commands::new::run(what).await,
        Command::List {
            board,
            boards,
            devices,
        } => commands::list::run(board, boards, devices).await,
        Command::Link { token } => commands::link::run(&token).await,
        Command::Read { id } => commands::read::run(id).await,
        Command::Whoami => commands::whoami::run().await,
        Command::Serve {
            port,
            open_registration,
        } => commands::serve::run(port, open_registration).await,
        Command::Invite { label } => commands::invite::run(label).await,
        Command::Share { note, with, permission } => commands::share::run_share(note, with, permission).await,
        Command::Revoke { note, identity } => commands::share::run_revoke(note, identity).await,
        Command::Shares { note } => commands::share::run_list(note).await,
        Command::BoardShare { board, with } => commands::board_share::run_share(board, with).await,
        Command::BoardRevoke { board, identity } => commands::board_share::run_revoke(board, identity).await,
        Command::Migrate => commands::migrate::run().await,
        Command::Block { cmd } => commands::block::run(cmd).await,
        Command::Note { cmd } => commands::note::run(cmd).await,
        Command::Tui => tui::run_tui().await,
        Command::DeleteAccount => commands::account_delete::run().await,
    }
}
