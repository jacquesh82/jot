use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

pub fn generate_ephemeral_keypair() -> (EphemeralSecret, PublicKey) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

pub fn diffie_hellman(secret: EphemeralSecret, peer_pub: &PublicKey) -> [u8; 32] {
    secret.diffie_hellman(peer_pub).to_bytes()
}

/// Generate a persistent X25519 identity key pair.
pub fn generate_static_keypair() -> (StaticSecret, PublicKey) {
    let secret = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

/// X25519 DH with a persistent static secret.
pub fn static_diffie_hellman(secret: &StaticSecret, peer_pub: &PublicKey) -> [u8; 32] {
    secret.diffie_hellman(peer_pub).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_secret_is_symmetric() {
        let (alice_secret, alice_pub) = generate_ephemeral_keypair();
        let (bob_secret, bob_pub) = generate_ephemeral_keypair();

        let alice_shared = diffie_hellman(alice_secret, &bob_pub);
        let bob_shared = diffie_hellman(bob_secret, &alice_pub);

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn different_pairs_produce_different_secrets() {
        let (s1, p1) = generate_ephemeral_keypair();
        let (_s2, p2) = generate_ephemeral_keypair();
        let (s3, _) = generate_ephemeral_keypair();

        let shared_ab = diffie_hellman(s1, &p2);
        let shared_ac = diffie_hellman(s3, &p1);

        assert_ne!(shared_ab, shared_ac);
    }

    #[test]
    fn static_keypair_roundtrip() {
        let (secret, public) = generate_static_keypair();
        let bytes = secret.to_bytes();
        let recovered = StaticSecret::from(bytes);
        assert_eq!(public.as_bytes(), PublicKey::from(&recovered).as_bytes());
    }

    #[test]
    fn static_dh_self_wrap_is_deterministic() {
        let (secret, public) = generate_static_keypair();
        let s1 = static_diffie_hellman(&secret, &public);
        let s2 = static_diffie_hellman(&secret, &public);
        assert_eq!(s1, s2);
    }

    #[test]
    fn static_dh_symmetric() {
        let (alice_secret, alice_pub) = generate_static_keypair();
        let (bob_secret, bob_pub) = generate_static_keypair();
        let alice_shared = static_diffie_hellman(&alice_secret, &bob_pub);
        let bob_shared = static_diffie_hellman(&bob_secret, &alice_pub);
        assert_eq!(alice_shared, bob_shared);
    }
}
