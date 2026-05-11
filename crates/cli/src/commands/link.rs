use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use crate::client::JotClient;
use crate::config::Config;
use crate::error::CliError;
use crate::identity;
use crate::t;
use std::time::{Duration, Instant};

pub async fn run(token: &str) -> Result<(), CliError> {
    let mut cfg = Config::load();
    let client = JotClient::new(&cfg);

    println!("{}", t!("cmd.link.waiting"));
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        let (status, jwt) = client.link_status(token).await?;
        match status.as_str() {
            "confirmed" => {
                let jwt = jwt.ok_or_else(|| CliError::Server("confirmed but no jwt".into()))?;
                // Decode claims from the JWT payload (no signature check needed here)
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
