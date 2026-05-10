use crate::state::AppState;
use axum::Router;
use ed25519_dalek::pkcs8::spki::EncodePublicKey;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::sync::Arc;
use storage::{Db, LocalStore};
use tempfile::tempdir;

pub async fn test_app_with_state() -> (Router, AppState) {
    let db = Db::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    let dir = Box::leak(Box::new(tempdir().unwrap()));
    let blobs = Arc::new(LocalStore::new(dir.path()));
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let signing_pem = signing_key
        .to_pkcs8_pem(Default::default())
        .unwrap()
        .to_string();
    let verifying_pem = verifying_key
        .to_public_key_pem(Default::default())
        .unwrap();
    let state = AppState::new(db, blobs, signing_pem, verifying_pem);
    let app = crate::build_router(state.clone());
    (app, state)
}

pub async fn test_app() -> Router {
    test_app_with_state().await.0
}
