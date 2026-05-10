use crate::auth::{make_claims, sign_token};
use crate::state::AppState;
use axum::Router;
use chrono::Utc;
use ed25519_dalek::pkcs8::spki::EncodePublicKey;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::SigningKey;
use jot_core::models::Device;
use rand::rngs::OsRng;
use std::sync::Arc;
use storage::{Db, LocalStore};
use tempfile::tempdir;
use uuid::Uuid;

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
    let verifying_pem = verifying_key.to_public_key_pem(Default::default()).unwrap();
    let state = AppState::new(db, blobs, signing_pem, verifying_pem);
    let app = crate::build_router(state.clone());
    (app, state)
}

pub async fn test_app() -> Router {
    test_app_with_state().await.0
}

/// Create a device row in the DB and return its signed JWT.
/// Use this in tests instead of bare `make_claims` + `sign_token` so the
/// middleware's device-existence check passes.
pub async fn test_token(state: &AppState) -> (String, String, String) {
    let device_id = Uuid::new_v4();
    let identity_id = Uuid::new_v4();
    let device = Device {
        id: device_id,
        identity_id,
        pub_key_x25519: String::new(),
        pub_key_ed25519: String::new(),
        name: "test-device".to_string(),
        last_seen: Utc::now(),
    };
    state.db.insert_device(&device).await.unwrap();
    let claims = make_claims(&device_id.to_string(), &identity_id.to_string());
    let token = sign_token(&claims, &state.signing_key_pem).unwrap();
    (token, device_id.to_string(), identity_id.to_string())
}

/// Like `test_token` but also inserts an identity row so `/identity/me` works.
pub async fn test_token_with_identity(state: &AppState) -> (String, String, String) {
    let (token, device_id, identity_id) = test_token(state).await;
    let name = format!("user-{}", &identity_id[..8]);
    state.db.insert_identity(&identity_id, &name).await.unwrap();
    (token, device_id, identity_id)
}
