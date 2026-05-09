#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("encryption failed: {0}")]
    Encryption(String),
    #[error("decryption failed: invalid key or corrupted data")]
    Decryption,
    #[error("signature verification failed")]
    SignatureInvalid,
    #[error("HKDF expand failed")]
    KeyDerivation,
}
