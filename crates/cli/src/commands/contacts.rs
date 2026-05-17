use crate::client::JotClient;
use crate::error::CliError;

pub async fn run() -> Result<(), CliError> {
    let client = JotClient::from_config();
    let body = client.get_json("/identity/contacts").await?;
    let arr = body.as_array().cloned().unwrap_or_default();
    if arr.is_empty() {
        println!("(no recent contacts)");
        return Ok(());
    }
    for c in arr {
        let id = c["id"].as_str().or(c["identity_id"].as_str()).unwrap_or("");
        let name = c["friendly_name"]
            .as_str()
            .or(c["name"].as_str())
            .unwrap_or("");
        println!("  {name}\t{id}");
    }
    Ok(())
}
