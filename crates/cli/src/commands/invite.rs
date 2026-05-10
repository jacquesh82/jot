use crate::client::JotClient;
use crate::config::Config;
use crate::error::CliError;

pub async fn run(label: Option<String>) -> Result<(), CliError> {
    let config = Config::load();
    let client = JotClient::new(&config);

    let body = serde_json::json!({ "label": label.unwrap_or_default() });
    let resp = client.post_json("/invites", &body).await?;

    let invite_token = resp["token"].as_str().unwrap_or("");
    let label_str = resp["label"].as_str().unwrap_or("");

    let base = config.server_url().trim_end_matches('/');
    let invite_url = format!("{}/#/register?invite={}", base, invite_token);

    println!("Invite token created");
    if !label_str.is_empty() {
        println!("  Label  : {}", label_str);
    }
    println!("  Token  : {}", invite_token);
    println!("  URL    : {}", invite_url);
    Ok(())
}
