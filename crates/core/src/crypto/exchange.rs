use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey};

pub fn generate_ephemeral_keypair() -> (EphemeralSecret, PublicKey) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

pub fn diffie_hellman(secret: EphemeralSecret, peer_pub: &PublicKey) -> [u8; 32] {
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
}
