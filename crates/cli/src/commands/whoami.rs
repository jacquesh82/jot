use crate::config::Config;
use crate::error::CliError;

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
                    "present"
                } else {
                    "absent"
                }
            );
        }
        _ => {
            eprintln!("Not registered — run: jot serve");
            std::process::exit(1);
        }
    }
    Ok(())
}
