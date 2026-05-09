pub mod middleware;

use crate::ApiError;
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceClaims {
    pub sub: String,
    pub identity_id: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn sign_token(claims: &DeviceClaims, signing_key_pem: &str) -> Result<String, ApiError> {
    let key = EncodingKey::from_ed_pem(signing_key_pem.as_bytes())
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    jsonwebtoken::encode(&Header::new(Algorithm::EdDSA), claims, &key)
        .map_err(|e| ApiError::Internal(e.to_string()))
}

pub fn verify_token(token: &str, verifying_key_pem: &str) -> Result<DeviceClaims, ApiError> {
    let key = DecodingKey::from_ed_pem(verifying_key_pem.as_bytes())
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.validate_exp = true;
    jsonwebtoken::decode::<DeviceClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|_| ApiError::Unauthorized)
}

pub fn make_claims(device_id: &str, identity_id: &str) -> DeviceClaims {
    let now = Utc::now().timestamp() as usize;
    DeviceClaims {
        sub: device_id.to_string(),
        identity_id: identity_id.to_string(),
        iat: now,
        exp: now + 30 * 24 * 3600,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::pkcs8::spki::EncodePublicKey;
    use ed25519_dalek::pkcs8::EncodePrivateKey;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn test_keypair_pem() -> (String, String) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let signing_pem = signing_key
            .to_pkcs8_pem(Default::default())
            .unwrap()
            .to_string();
        let verifying_pem = verifying_key
            .to_public_key_pem(Default::default())
            .unwrap();
        (signing_pem, verifying_pem)
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let (signing_pem, verifying_pem) = test_keypair_pem();
        let claims = make_claims("device-1", "identity-1");
        let token = sign_token(&claims, &signing_pem).unwrap();
        let decoded = verify_token(&token, &verifying_pem).unwrap();
        assert_eq!(decoded.sub, "device-1");
        assert_eq!(decoded.identity_id, "identity-1");
    }

    #[test]
    fn verify_fails_with_wrong_key() {
        let (signing_pem, _) = test_keypair_pem();
        let (_, other_verifying_pem) = test_keypair_pem();
        let claims = make_claims("device-1", "identity-1");
        let token = sign_token(&claims, &signing_pem).unwrap();
        assert!(verify_token(&token, &other_verifying_pem).is_err());
    }

    #[test]
    fn verify_fails_with_tampered_token() {
        let (signing_pem, verifying_pem) = test_keypair_pem();
        let claims = make_claims("device-1", "identity-1");
        let token = sign_token(&claims, &signing_pem).unwrap();
        let tampered = format!("{}x", token);
        assert!(verify_token(&tampered, &verifying_pem).is_err());
    }
}
