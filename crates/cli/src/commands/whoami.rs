use crate::client::JotClient;
use crate::config::Config;
use crate::error::CliError;
use crate::t;

pub async fn run(set_name: Option<String>, set_lang: Option<String>) -> Result<(), CliError> {
    let config = Config::load();
    if set_name.is_some() || set_lang.is_some() {
        let client = JotClient::new(&config);
        let mut body = serde_json::Map::new();
        if let Some(n) = set_name {
            body.insert("friendly_name".into(), serde_json::Value::String(n));
        }
        if let Some(l) = set_lang {
            body.insert("lang".into(), serde_json::Value::String(l));
        }
        client
            .patch_json("/identity/me", &serde_json::Value::Object(body))
            .await?;
        println!("identity updated");
        return Ok(());
    }
    match (config.identity_id.as_deref(), config.device_id.as_deref()) {
        (Some(iid), Some(did)) => {
            println!("Identity : {}", iid);
            println!("Device   : {}", did);
            println!("Server   : {}", config.server_url());
            println!(
                "Token    : {}",
                if config.token.is_some() {
                    t!("cmd.whoami.tokenPresent")
                } else {
                    t!("cmd.whoami.tokenAbsent")
                }
            );
        }
        _ => {
            eprintln!("{}", t!("cmd.whoami.notRegistered"));
            std::process::exit(1);
        }
    }
    Ok(())
}
