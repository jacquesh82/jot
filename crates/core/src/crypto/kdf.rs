use crate::CoreError;
use hkdf::Hkdf;
use sha2::Sha256;

pub struct DerivedKeys {
    pub notes_key: [u8; 32],
    pub ai_key: [u8; 32],
}

/// Derive a Board Encryption Key from the identity private key and board UUID.
/// BEK = HKDF-SHA256(ikm=privkey, info="jot-board-v1" || board_id_bytes)
pub fn derive_bek(privkey: &[u8; 32], board_id_bytes: &[u8; 16]) -> Result<[u8; 32], CoreError> {
    let hk = Hkdf::<Sha256>::new(None, privkey);
    let mut bek = [0u8; 32];
    let mut info = [0u8; 12 + 16];
    info[..12].copy_from_slice(b"jot-board-v1");
    info[12..].copy_from_slice(board_id_bytes);
    hk.expand(&info, &mut bek)
        .map_err(|_| CoreError::KeyDerivation)?;
    Ok(bek)
}

/// Derive a Data Encryption Key from a BEK and note UUID.
/// DEK = HKDF-SHA256(ikm=bek, info="jot-note-v1" || note_id_bytes)
pub fn derive_dek(bek: &[u8; 32], note_id_bytes: &[u8; 16]) -> Result<[u8; 32], CoreError> {
    let hk = Hkdf::<Sha256>::new(None, bek);
    let mut dek = [0u8; 32];
    let mut info = [0u8; 11 + 16];
    info[..11].copy_from_slice(b"jot-note-v1");
    info[11..].copy_from_slice(note_id_bytes);
    hk.expand(&info, &mut dek)
        .map_err(|_| CoreError::KeyDerivation)?;
    Ok(dek)
}

/// Derive a DEK-wrapping key from an X25519 shared secret using HKDF-SHA256.
pub fn derive_wrap_key(shared_secret: &[u8; 32]) -> Result<[u8; 32], CoreError> {
    let hk = Hkdf::<Sha256>::new(None, shared_secret);
    let mut wrap_key = [0u8; 32];
    hk.expand(b"jot-share-v1", &mut wrap_key)
        .map_err(|_| CoreError::KeyDerivation)?;
    Ok(wrap_key)
}

pub fn derive_keys(master_key: &[u8; 32]) -> Result<DerivedKeys, CoreError> {
    let hk = Hkdf::<Sha256>::new(None, master_key);

    let mut notes_key = [0u8; 32];
    hk.expand(b"jot-v1-notes", &mut notes_key)
        .map_err(|_| CoreError::KeyDerivation)?;

    let mut ai_key = [0u8; 32];
    hk.expand(b"jot-v1-ai", &mut ai_key)
        .map_err(|_| CoreError::KeyDerivation)?;

    Ok(DerivedKeys { notes_key, ai_key })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_master_key() -> [u8; 32] {
        [42u8; 32]
    }

    #[test]
    fn derived_keys_are_reproducible() {
        let k1 = derive_keys(&test_master_key()).unwrap();
        let k2 = derive_keys(&test_master_key()).unwrap();
        assert_eq!(k1.notes_key, k2.notes_key);
        assert_eq!(k1.ai_key, k2.ai_key);
    }

    #[test]
    fn notes_key_and_ai_key_differ() {
        let keys = derive_keys(&test_master_key()).unwrap();
        assert_ne!(keys.notes_key, keys.ai_key);
    }

    #[test]
    fn different_master_keys_produce_different_outputs() {
        let k1 = derive_keys(&[1u8; 32]).unwrap();
        let k2 = derive_keys(&[2u8; 32]).unwrap();
        assert_ne!(k1.notes_key, k2.notes_key);
    }
}
