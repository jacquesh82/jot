use crate::client::JotClient;
use crate::error::CliError;

pub async fn run(token: &str) -> Result<(), CliError> {
    let client = JotClient::from_config();
    client.confirm_link(token).await?;
    println!("Device linked successfully.");
    Ok(())
}
