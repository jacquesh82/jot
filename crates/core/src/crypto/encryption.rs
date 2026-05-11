use crate::CoreError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::{rngs::OsRng, RngCore};

pub fn generate_dek() -> [u8; 32] {
    let mut dek = [0u8; 32];
    OsRng.fill_bytes(&mut dek);
    dek
}

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CoreError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CoreError::Encryption(e.to_string()))?;

    // blob format: [12-byte nonce] || [ciphertext + 16-byte GCM tag]
    let mut blob = Vec::with_capacity(12 + encrypted.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&encrypted);
    Ok(blob)
}

pub fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, CoreError> {
    if blob.len() < 12 {
        return Err(CoreError::Decryption);
    }
    let (nonce_bytes, cipher_data) = blob.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, cipher_data)
        .map_err(|_| CoreError::Decryption)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [7u8; 32]
    }

    #[test]
    fn round_trip() {
        let plaintext = b"hello, jot!";
        let ciphertext = encrypt(&test_key(), plaintext).unwrap();
        let recovered = decrypt(&test_key(), &ciphertext).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn ciphertext_differs_from_plaintext() {
        let plaintext = b"secret note";
        let ciphertext = encrypt(&test_key(), plaintext).unwrap();
        assert_ne!(&ciphertext[12..], plaintext.as_slice());
    }

    #[test]
    fn two_encryptions_produce_different_blobs() {
        let plaintext = b"same content";
        let c1 = encrypt(&test_key(), plaintext).unwrap();
        let c2 = encrypt(&test_key(), plaintext).unwrap();
        assert_ne!(c1, c2);
    }

    #[test]
    fn tampered_nonce_fails_decryption() {
        let plaintext = b"tamper test";
        let mut ciphertext = encrypt(&test_key(), plaintext).unwrap();
        ciphertext[0] ^= 0xFF;
        assert!(decrypt(&test_key(), &ciphertext).is_err());
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let plaintext = b"wrong key test";
        let ciphertext = encrypt(&test_key(), plaintext).unwrap();
        let wrong_key = [99u8; 32];
        assert!(decrypt(&wrong_key, &ciphertext).is_err());
    }

    #[test]
    fn too_short_ciphertext_fails() {
        assert!(decrypt(&test_key(), &[0u8; 11]).is_err());
    }
}
