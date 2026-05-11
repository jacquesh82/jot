use crate::client::JotClient;
use crate::error::CliError;
use uuid::Uuid;

pub async fn run(id: Uuid) -> Result<(), CliError> {
    let client = JotClient::from_config();
    let text = client.get_note_text(id).await?;
    print!("{}", text);
    Ok(())
}
