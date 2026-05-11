use crate::client::JotClient;
use crate::config::Config;
use crate::error::CliError;
use crate::t;
use std::io::{self, Write};

pub async fn run() -> Result<(), CliError> {
    eprint!("{}", t!("cmd.delete.prompt"));
    io::stderr().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| CliError::Config(e.to_string()))?;

    if input.trim() != "delete" {
        eprintln!("{}", t!("cmd.delete.aborted"));
        return Ok(());
    }

    let client = JotClient::from_config();
    client.delete_account().await?;

    // Wipe local token so subsequent commands fail cleanly.
    let mut config = Config::load();
    config.token = None;
    config.identity_id = None;
    config.device_id = None;
    config.save()?;

    println!("{}", t!("cmd.delete.done"));
    Ok(())
}
