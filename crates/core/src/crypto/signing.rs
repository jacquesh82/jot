use crate::CoreError;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

pub fn generate_device_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

pub fn sign(key: &SigningKey, message: &[u8]) -> Signature {
    key.sign(message)
}

pub fn verify(key: &VerifyingKey, message: &[u8], sig: &Signature) -> Result<(), CoreError> {
    key.verify(message, sig)
        .map_err(|_| CoreError::SignatureInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_ok() {
        let (signing_key, verifying_key) = generate_device_keypair();
        let message = b"device token payload";
        let sig = sign(&signing_key, message);
        assert!(verify(&verifying_key, message, &sig).is_ok());
    }

    #[test]
    fn verify_fails_with_wrong_key() {
        let (signing_key, _) = generate_device_keypair();
        let (_, other_verifying_key) = generate_device_keypair();
        let message = b"device token payload";
        let sig = sign(&signing_key, message);
        assert!(verify(&other_verifying_key, message, &sig).is_err());
    }

    #[test]
    fn verify_fails_with_tampered_message() {
        let (signing_key, verifying_key) = generate_device_keypair();
        let message = b"original message";
        let sig = sign(&signing_key, message);
        assert!(verify(&verifying_key, b"tampered message", &sig).is_err());
    }

    #[test]
    fn two_keypairs_are_different() {
        let (k1, _) = generate_device_keypair();
        let (k2, _) = generate_device_keypair();
        assert_ne!(k1.to_bytes(), k2.to_bytes());
    }
}
