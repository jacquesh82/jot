use crate::client::JotClient;
use crate::config::Config;
use crate::error::CliError;
use crate::t;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum InviteCmd {
    /// Create a new invite token
    Create {
        #[arg(long)]
        label: Option<String>,
    },
    /// List all invites
    List,
    /// Revoke an invite by token
    Revoke { token: String },
}

pub async fn run(cmd: InviteCmd) -> Result<(), CliError> {
    let config = Config::load();
    let client = JotClient::new(&config);
    match cmd {
        InviteCmd::Create { label } => create(&config, &client, label).await,
        InviteCmd::List => list(&client).await,
        InviteCmd::Revoke { token } => revoke(&client, &token).await,
    }
}

async fn create(
    config: &Config,
    client: &JotClient,
    label: Option<String>,
) -> Result<(), CliError> {
    let body = serde_json::json!({ "label": label.unwrap_or_default() });
    let resp = client.post_json("/invites", &body).await?;

    let invite_token = resp["token"].as_str().unwrap_or("");
    let label_str = resp["label"].as_str().unwrap_or("");

    let base = config.server_url().trim_end_matches('/');
    let invite_url = format!("{}/#/register?invite={}", base, invite_token);

    println!("{}", t!("cmd.invite.created"));
    if !label_str.is_empty() {
        println!("{}", t!("cmd.invite.label", "label" => label_str));
    }
    println!("{}", t!("cmd.invite.token", "token" => invite_token));
    println!("{}", t!("cmd.invite.url", "url" => invite_url));
    Ok(())
}

async fn list(client: &JotClient) -> Result<(), CliError> {
    let body = client.get_json("/invites").await?;
    let arr = body.as_array().cloned().unwrap_or_default();
    if arr.is_empty() {
        println!("(no invites)");
        return Ok(());
    }
    for inv in arr {
        let token = inv["token"].as_str().unwrap_or("");
        let label = inv["label"].as_str().unwrap_or("");
        let used = inv["used"].as_bool().unwrap_or(false);
        let created = inv["created_at"].as_str().unwrap_or("");
        let flag = if used { "USED" } else { "open" };
        println!("  {flag:5}  {token}  {created}  {label}");
    }
    Ok(())
}

async fn revoke(client: &JotClient, token: &str) -> Result<(), CliError> {
    client.delete_path(&format!("/invites/{}", token)).await?;
    println!("invite revoked");
    Ok(())
}
