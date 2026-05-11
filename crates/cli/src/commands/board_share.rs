use crate::client::JotClient;
use crate::error::CliError;
use crate::identity;
use crate::t;
use jot_core::crypto::{derive_bek, encrypt};
use uuid::Uuid;

pub async fn run_share(board_id: Uuid, target: String) -> Result<(), CliError> {
    let client = JotClient::from_config();

    // Resolve recipient + ensure they have a public key.
    let target_info = client
        .lookup_identity(&target)
        .await?
        .ok_or_else(|| CliError::Server(t!("cmd.boardShare.identityNotFound", "target" => target)))?;
    let recipient_pubkey_hex = target_info
        .public_key_x25519
        .ok_or_else(|| CliError::Server(t!("cmd.boardShare.noPubkey", "target" => target)))?;

    // Load our key pair and register our public key.
    let (secret, public) = identity::load_or_generate()?;
    client.register_pubkey(&hex::encode(public.as_bytes())).await?;

    // Derive BEK from our identity key + board_id.
    let privkey = secret.to_bytes();
    let bek = derive_bek(&privkey, board_id.as_bytes())
        .map_err(|e| CliError::Server(format!("BEK derivation failed: {e}")))?;

    // Encrypt BEK for the recipient with cross-ECDH wrap key.
    let recipient_wrap = identity::cross_wrap_key(&secret, &recipient_pubkey_hex)?;
    let encrypted_bek = encrypt(&recipient_wrap, &bek)
        .map_err(|e| CliError::Server(format!("BEK encryption failed: {e}")))?;

    // Grant board-level access.
    client.share_board(board_id, &target).await?;

    // Store the encrypted BEK for the recipient.
    client
        .put_board_key(board_id, &target_info.id, &hex::encode(&encrypted_bek))
        .await?;

    println!("{}", t!("cmd.boardShare.shared", "id" => board_id, "target" => target));
    Ok(())
}

pub async fn run_revoke(board_id: Uuid, identity_id: String) -> Result<(), CliError> {
    let client = JotClient::from_config();
    // Revoke board share (also handled by the API: deletes note-level DEKs for completeness).
    client.revoke_board_share(board_id, &identity_id).await?;
    // Delete the BEK so the ex-member can no longer derive any note DEKs.
    client.delete_board_key(board_id, &identity_id).await?;
    println!("{}", t!("cmd.boardShare.revoked", "id" => board_id, "target" => identity_id));
    Ok(())
}
