use crate::client::JotClient;
use crate::error::CliError;
use clap::Subcommand;
use uuid::Uuid;

#[derive(Subcommand)]
pub enum NoteCmd {
    /// Set or clear a note's encrypted title (pass empty string to clear)
    Title {
        /// Note ID
        id: Uuid,
        /// Title text (empty string clears the title)
        text: String,
    },
}

pub async fn run(cmd: NoteCmd) -> Result<(), CliError> {
    let client = JotClient::from_config();
    match cmd {
        NoteCmd::Title { id, text } => {
            client.set_note_title(id, &text).await?;
            println!("title updated");
            Ok(())
        }
    }
}
