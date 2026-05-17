use crate::error::CliError;
use jot_core::crypto::{generate_static_keypair, static_diffie_hellman};
use std::os::unix::fs::PermissionsExt;
use x25519_dalek::{PublicKey, StaticSecret};

/// Load the X25519 identity key pair from disk, generating one on first use.
pub fn load_or_generate() -> Result<(StaticSecret, PublicKey), CliError> {
    let key_path = identity_key_path()?;

    if key_path.exists() {
        let bytes = std::fs::read(&key_path)
            .map_err(|e| CliError::Config(format!("cannot read identity key: {e}")))?;
        if bytes.len() != 32 {
            return Err(CliError::Config("identity key file is corrupt".into()));
        }
        let mut raw = [0u8; 32];
        raw.copy_from_slice(&bytes);
        let secret = StaticSecret::from(raw);
        let public = PublicKey::from(&secret);
        return Ok((secret, public));
    }

    // First use: generate and persist.
    let (secret, public) = generate_static_keypair();
    if let Some(parent) = key_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CliError::Config(format!("cannot create config dir: {e}")))?;
    }
    std::fs::write(&key_path, secret.to_bytes())
        .map_err(|e| CliError::Config(format!("cannot write identity key: {e}")))?;
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| CliError::Config(format!("cannot set key permissions: {e}")))?;

    Ok((secret, public))
}

/// Derive the wrap key for sharing: ECDH(my_priv, peer_pub) → HKDF.
/// By ECDH symmetry: ECDH(owner_priv, recipient_pub) == ECDH(recipient_priv, owner_pub).
pub fn cross_wrap_key(
    my_secret: &StaticSecret,
    peer_pubkey_hex: &str,
) -> Result<[u8; 32], CliError> {
    let raw = hex::decode(peer_pubkey_hex)
        .map_err(|_| CliError::Config("invalid peer pubkey hex".into()))?;
    if raw.len() != 32 {
        return Err(CliError::Config("peer pubkey must be 32 bytes".into()));
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&raw);
    let peer_pub = PublicKey::from(bytes);
    let shared = static_diffie_hellman(my_secret, &peer_pub);
    jot_core::crypto::derive_wrap_key(&shared)
        .map_err(|e| CliError::Config(format!("key derivation failed: {e}")))
}

fn identity_key_path() -> Result<std::path::PathBuf, CliError> {
    let base = dirs::config_dir()
        .ok_or_else(|| CliError::Config("cannot determine config directory".into()))?;
    Ok(base.join("jot").join("identity.key"))
}
