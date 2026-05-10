use crate::client::JotClient;
use crate::error::CliError;
use uuid::Uuid;

pub async fn run(id: Uuid) -> Result<(), CliError> {
    let client = JotClient::from_config();
    let bytes = client.get_blob(id).await?;
    let text = String::from_utf8(bytes)
        .map_err(|_| CliError::Server("note content is not valid UTF-8 (binary blob)".into()))?;
    print!("{}", text);
    Ok(())
}
