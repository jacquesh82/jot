mod client;
mod commands;
mod config;
mod error;
mod i18n;
mod identity;
mod onboarding;
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
    /// Initiate a new device-link session and print token/code/URL
    LinkInit,
    /// Print the status of a link token (pending|confirmed|expired)
    LinkStatus { token: String },
    /// Read the content of a note
    Read {
        /// Note ID
        id: uuid::Uuid,
    },
    /// Show current identity and device (or update with --set-name/--set-lang)
    Whoami {
        #[arg(long, value_name = "NAME")]
        set_name: Option<String>,
        #[arg(long, value_name = "LANG")]
        set_lang: Option<String>,
    },
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
    /// List all invites
    Invites,
    /// Revoke an invite by token
    InviteRevoke { token: String },
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
    /// Board-level operations (rename, delete, reorder)
    Board {
        #[command(subcommand)]
        cmd: commands::board::BoardCmd,
    },
    /// Device-level operations (rename, delete)
    Device {
        #[command(subcommand)]
        cmd: commands::device::DeviceCmd,
    },
    /// Tag-level operations (list, set, blocks)
    Tag {
        #[command(subcommand)]
        cmd: commands::tag::TagCmd,
    },
    /// Export all your data as JSON
    Export {
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// List recent contacts
    Contacts,
    /// Show backlinks for a note or block
    Backlinks {
        #[arg(long)]
        note: Option<uuid::Uuid>,
        #[arg(long)]
        block: Option<uuid::Uuid>,
    },
    /// Show notes grouped by day (journal-style)
    Journal {
        /// Filter to one date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
    },
    /// List all todo blocks across all notes
    Todo {
        /// Filter by tag (without #)
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show counts of boards / notes (and per-board breakdown)
    Stats,
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

    // Mirror the SPA flow: if no token is stored, walk the user through
    // create-or-link before running any command that requires auth.
    let needs_auth = !matches!(
        cli.command,
        Command::Migrate
            | Command::Serve { .. }
            | Command::Link { .. }
            | Command::LinkInit
            | Command::LinkStatus { .. }
    );
    if needs_auth && config::Config::load().token.is_none() {
        onboarding::ensure_authenticated().await?;
    }

    match cli.command {
        Command::Add { text, board } => commands::add::run(text, board).await,
        Command::New { what } => commands::new::run(what).await,
        Command::List {
            board,
            boards,
            devices,
        } => commands::list::run(board, boards, devices).await,
        Command::Link { token } => commands::link::run(&token).await,
        Command::LinkInit => {
            commands::link::run_cmd(commands::link::LinkCmd::Init).await
        }
        Command::LinkStatus { token } => {
            commands::link::run_cmd(commands::link::LinkCmd::Status { token }).await
        }
        Command::Read { id } => commands::read::run(id).await,
        Command::Whoami { set_name, set_lang } => commands::whoami::run(set_name, set_lang).await,
        Command::Serve {
            port,
            open_registration,
        } => commands::serve::run(port, open_registration).await,
        Command::Invite { label } => {
            commands::invite::run(commands::invite::InviteCmd::Create { label }).await
        }
        Command::Invites => {
            commands::invite::run(commands::invite::InviteCmd::List).await
        }
        Command::InviteRevoke { token } => {
            commands::invite::run(commands::invite::InviteCmd::Revoke { token }).await
        }
        Command::Share { note, with, permission } => commands::share::run_share(note, with, permission).await,
        Command::Revoke { note, identity } => commands::share::run_revoke(note, identity).await,
        Command::Shares { note } => commands::share::run_list(note).await,
        Command::BoardShare { board, with } => commands::board_share::run_share(board, with).await,
        Command::BoardRevoke { board, identity } => commands::board_share::run_revoke(board, identity).await,
        Command::Migrate => commands::migrate::run().await,
        Command::Block { cmd } => commands::block::run(cmd).await,
        Command::Note { cmd } => commands::note::run(cmd).await,
        Command::Board { cmd } => commands::board::run(cmd).await,
        Command::Device { cmd } => commands::device::run(cmd).await,
        Command::Tag { cmd } => commands::tag::run(cmd).await,
        Command::Export { out } => commands::export::run(out).await,
        Command::Contacts => commands::contacts::run().await,
        Command::Backlinks { note, block } => commands::backlinks::run(note, block).await,
        Command::Journal { date } => commands::journal::run(date).await,
        Command::Todo { tag } => commands::todo::run(tag).await,
        Command::Stats => commands::stats::run().await,
        Command::Tui => tui::run_tui().await,
        Command::DeleteAccount => commands::account_delete::run().await,
    }
}
