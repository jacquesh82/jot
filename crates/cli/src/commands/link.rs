use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use crate::client::JotClient;
use crate::config::Config;
use crate::error::CliError;
use crate::identity;
use crate::t;
use clap::Subcommand;
use std::time::{Duration, Instant};

#[derive(Subcommand)]
pub enum LinkCmd {
    /// Confirm a pending link by polling until completion (default — also `jot link <token>`)
    Confirm { token: String },
    /// Initiate a new link session and print the token/code/URL for the other device
    Init,
    /// Print the current status of a link token
    Status { token: String },
}

/// Top-level entrypoint kept for backwards compatibility: `jot link <token>`.
pub async fn run(token: &str) -> Result<(), CliError> {
    confirm(token).await
}

pub async fn run_cmd(cmd: LinkCmd) -> Result<(), CliError> {
    match cmd {
        LinkCmd::Confirm { token } => confirm(&token).await,
        LinkCmd::Init => init().await,
        LinkCmd::Status { token } => status(&token).await,
    }
}

async fn init() -> Result<(), CliError> {
    let cfg = Config::load();
    let client = JotClient::new(&cfg);
    let resp = client.post_json_optauth("/link/init", &serde_json::json!({})).await?;
    let token = resp["token"].as_str().unwrap_or("");
    let code = resp["code"].as_str().unwrap_or("");
    let expires = resp["expires_at"].as_str().unwrap_or("");
    let base = cfg.server_url().trim_end_matches('/');
    let url = format!("{}/#/link?token={}", base, token);
    println!("Link initiated.");
    println!("  Token   : {token}");
    println!("  Code    : {code}");
    println!("  URL     : {url}");
    println!("  Expires : {expires}");
    if cfg.token.is_none() {
        println!();
        println!("Now run `jot link confirm {token}` here (or open the URL) once approved on the other device.");
    } else {
        println!();
        println!("Pre-confirmed (you were authenticated). Polling for adoption...");
        confirm(token).await?;
    }
    Ok(())
}

async fn status(token: &str) -> Result<(), CliError> {
    let cfg = Config::load();
    let client = JotClient::new(&cfg);
    let (s, jwt) = client.link_status(token).await?;
    println!("status: {s}");
    if jwt.is_some() {
        println!("jwt: <present>");
    }
    Ok(())
}

async fn confirm(token: &str) -> Result<(), CliError> {
    let mut cfg = Config::load();
    let client = JotClient::new(&cfg);

    println!("{}", t!("cmd.link.waiting"));
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        let (status, jwt) = client.link_status(token).await?;
        match status.as_str() {
            "confirmed" => {
                let jwt = jwt.ok_or_else(|| CliError::Server("confirmed but no jwt".into()))?;
                if let Some(payload) = jwt.split('.').nth(1) {
                    if let Ok(bytes) = URL_SAFE_NO_PAD.decode(payload) {
                        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            cfg.device_id = v["sub"].as_str().map(str::to_owned);
                            cfg.identity_id = v["identity_id"].as_str().map(str::to_owned);
                        }
                    }
                }
                cfg.token = Some(jwt);
                cfg.save()?;
                let authed = JotClient::new(&cfg);
                let (_secret, public) = identity::load_or_generate()?;
                authed
                    .register_pubkey(&hex::encode(public.as_bytes()))
                    .await?;
                println!("{}", t!("cmd.link.success"));
                return Ok(());
            }
            "expired" => return Err(CliError::Server(t!("cmd.link.expired"))),
            _ => {}
        }
        if Instant::now() >= deadline {
            return Err(CliError::Server(t!("cmd.link.timedOut")));
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
