use crate::config::Config;
use crate::error::CliError;
use crate::t;

pub async fn run() -> Result<(), CliError> {
    let config = Config::load();
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
