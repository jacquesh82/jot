use crate::client::JotClient;
use crate::error::CliError;
use clap::Subcommand;
use uuid::Uuid;

#[derive(Subcommand)]
pub enum DeviceCmd {
    /// Rename a device
    Rename { id: Uuid, name: String },
    /// Delete (revoke) a device
    Delete { id: Uuid },
}

pub async fn run(cmd: DeviceCmd) -> Result<(), CliError> {
    let client = JotClient::from_config();
    match cmd {
        DeviceCmd::Rename { id, name } => {
            client.rename_device(id, &name).await?;
            println!("device renamed");
        }
        DeviceCmd::Delete { id } => {
            client.delete_device(id).await?;
            println!("device deleted");
        }
    }
    Ok(())
}
