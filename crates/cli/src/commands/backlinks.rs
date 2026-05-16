use crate::client::JotClient;
use crate::error::CliError;
use uuid::Uuid;

pub async fn run(note: Option<Uuid>, block: Option<Uuid>) -> Result<(), CliError> {
    let client = JotClient::from_config();
    let path = match (note, block) {
        (Some(n), None) => format!("/notes/{}/backlinks", n),
        (None, Some(b)) => format!("/blocks/{}/backlinks", b),
        _ => {
            return Err(crate::error::CliError::Config(
                "specify exactly one of --note or --block".into(),
            ))
        }
    };
    let body = client.get_json(&path).await?;
    println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
    Ok(())
}
