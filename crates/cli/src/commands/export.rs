use crate::client::JotClient;
use crate::error::CliError;
use std::path::PathBuf;

pub async fn run(out: Option<PathBuf>) -> Result<(), CliError> {
    let client = JotClient::from_config();
    let body = client.get_json("/export").await?;
    let pretty = serde_json::to_string_pretty(&body)
        .map_err(|e| CliError::Server(format!("serialize: {e}")))?;
    match out {
        Some(p) => {
            std::fs::write(&p, pretty)?;
            println!("exported to {}", p.display());
        }
        None => println!("{pretty}"),
    }
    Ok(())
}
