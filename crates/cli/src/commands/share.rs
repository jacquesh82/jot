use crate::client::JotClient;
use crate::error::CliError;
use crate::t;
use uuid::Uuid;

pub async fn run_share(note_id: Uuid, target: String, permission: String) -> Result<(), CliError> {
    if !matches!(permission.as_str(), "read" | "write" | "delete") {
        return Err(CliError::Server(t!("cmd.share.invalidPerm")));
    }
    let client = JotClient::from_config();
    client.share_note(note_id, &target, &permission).await?;
    println!(
        "{}",
        t!("cmd.share.shared", "id" => note_id, "target" => target, "perm" => permission)
    );
    Ok(())
}

pub async fn run_revoke(note_id: Uuid, target_id: String) -> Result<(), CliError> {
    let client = JotClient::from_config();
    println!("{}", t!("cmd.share.rotating"));
    client.revoke_share(note_id, &target_id).await?;
    println!("{}", t!("cmd.share.revoked"));
    Ok(())
}

pub async fn run_list(note_id: Uuid) -> Result<(), CliError> {
    let client = JotClient::from_config();
    let shares = client.list_note_shares(note_id).await?;
    if shares.is_empty() {
        println!("{}", t!("cmd.share.noShares"));
        return Ok(());
    }
    println!("{}", t!("cmd.share.sharedWith"));
    for s in &shares {
        let name = s.shared_with_name.as_deref().unwrap_or(&s.shared_with_id);
        let perm = s.permission.as_deref().unwrap_or("read");
        let enc = if s.public_key_x25519.is_some() {
            "\u{1f512}"
        } else {
            "\u{26a0} no key"
        };
        println!("  {enc}  {name}  ({})  [{perm}]", &s.shared_with_id[..8]);
    }
    Ok(())
}
