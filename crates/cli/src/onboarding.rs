use crate::client::JotClient;
use crate::config::Config;
use crate::error::CliError;
use crate::identity;
use crate::t;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::io::{self, BufRead, Write};
use std::time::{Duration, Instant};

/// If no token is stored, walk the user through the SPA-equivalent welcome:
/// create a new account *or* link this CLI to an existing identity.
/// On success the JWT (and identity/device ids) are persisted to config.
pub async fn ensure_authenticated() -> Result<(), CliError> {
    let cfg = Config::load();
    if cfg.token.is_some() {
        return Ok(());
    }
    welcome().await
}

async fn welcome() -> Result<(), CliError> {
    println!("{}", t!("onboarding.welcome"));
    println!();
    println!("  1) {}", t!("onboarding.createAccount"));
    println!("  2) {}", t!("onboarding.linkAccount"));
    println!();
    let choice = prompt(&t!("onboarding.choose"))?;
    match choice.trim() {
        "1" | "c" | "C" => create_flow().await,
        "2" | "l" | "L" => link_flow().await,
        other => Err(CliError::Config(format!("invalid choice: {other}"))),
    }
}

async fn create_flow() -> Result<(), CliError> {
    let mut cfg = Config::load();
    let server = prompt_default(&t!("onboarding.serverUrl"), cfg.server_url())?;
    if !server.is_empty() {
        cfg.server_url = Some(server);
    }
    let display_name = prompt(&t!("onboarding.displayName"))?;
    let client = JotClient::new(&cfg);

    // First attempt: no invite. If the server requires one, prompt for it.
    let display = if display_name.trim().is_empty() {
        None
    } else {
        Some(display_name.trim())
    };

    let res = client.register(display, None).await;
    let (jwt, identity_id, device_id) = match res {
        Ok(v) => v,
        Err(CliError::Server(msg)) if msg.contains("invite_required") || msg.contains("invalid_invite") || msg.contains("invite_revoked") => {
            let invite = prompt(&t!("onboarding.inviteToken"))?;
            client.register(display, Some(invite.trim())).await?
        }
        Err(e) => return Err(e),
    };

    cfg.token = Some(jwt);
    if !identity_id.is_empty() {
        cfg.identity_id = Some(identity_id);
    }
    if !device_id.is_empty() {
        cfg.device_id = Some(device_id);
    }
    cfg.save()?;

    // Register our X25519 pubkey so we can share/be shared with right away.
    let authed = JotClient::new(&cfg);
    let (_secret, public) = identity::load_or_generate()?;
    authed.register_pubkey(&hex::encode(public.as_bytes())).await?;

    println!("{}", t!("onboarding.accountCreated"));
    Ok(())
}

async fn link_flow() -> Result<(), CliError> {
    let mut cfg = Config::load();
    let server = prompt_default(&t!("onboarding.serverUrl"), cfg.server_url())?;
    if !server.is_empty() {
        cfg.server_url = Some(server);
        cfg.save()?;
    }
    let client = JotClient::new(&cfg);

    let resp = client
        .post_json_optauth("/link/init", &serde_json::json!({}))
        .await?;
    let token = resp["token"].as_str().unwrap_or("").to_string();
    let code = resp["code"].as_str().unwrap_or("");
    let base = cfg.server_url().trim_end_matches('/');
    let url = format!("{}/#/link?token={}", base, token);

    println!();
    println!("{}", t!("onboarding.linkInstructions"));
    println!("  {}: {code}", t!("onboarding.code"));
    println!("  {}: {url}", t!("onboarding.url"));
    println!();
    println!("{}", t!("cmd.link.waiting"));

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        let (status, jwt) = client.link_status(&token).await?;
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

fn prompt(label: &str) -> Result<String, CliError> {
    print!("{label} ");
    io::stdout().flush().ok();
    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| CliError::Config(format!("stdin: {e}")))?;
    Ok(line.trim().to_string())
}

fn prompt_default(label: &str, default: &str) -> Result<String, CliError> {
    let l = format!("{label} [{default}]:");
    let v = prompt(&l)?;
    Ok(if v.is_empty() { String::new() } else { v })
}
