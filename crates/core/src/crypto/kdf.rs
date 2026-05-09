use crate::CoreError;
use hkdf::Hkdf;
use sha2::Sha256;

pub struct DerivedKeys {
    pub notes_key: [u8; 32],
    pub ai_key: [u8; 32],
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
