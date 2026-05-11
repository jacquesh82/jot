//! End-to-end happy-path test for the block lifecycle.
//!
//! Exercises board → note → blocks → links → tags → backlinks → delete
//! through the real axum Router via `tower::ServiceExt::oneshot`.

use api::auth::{make_claims, sign_token};
use api::{build_router, AppState};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use base64::Engine;
use chrono::Utc;
use ed25519_dalek::pkcs8::spki::EncodePublicKey;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::SigningKey;
use http_body_util::BodyExt;
use jot_core::models::Device;
use rand::rngs::OsRng;
use serde_json::{json, Value};
use std::sync::Arc;
use storage::{Db, LocalStore};
use tempfile::tempdir;
use tower::ServiceExt;
use uuid::Uuid;

// ─── Bootstrap ────────────────────────────────────────────────────────────────

async fn boot() -> (Router, AppState) {
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
    let state = AppState::new(db, blobs, signing_pem, verifying_pem, [0u8; 32]);
    let router = build_router(state.clone());
    (router, state)
}

async fn make_token(state: &AppState) -> String {
    let device_id = Uuid::new_v4();
    let identity_id = Uuid::new_v4();
    let device = Device {
        id: device_id,
        identity_id,
        pub_key_x25519: String::new(),
        pub_key_ed25519: String::new(),
        name: "e2e-device".to_string(),
        last_seen: Utc::now(),
    };
    state.db.insert_device(&device).await.unwrap();
    state
        .db
        .insert_identity(&identity_id.to_string(), &format!("u-{}", &identity_id.to_string()[..8]))
        .await
        .unwrap();
    let claims = make_claims(&device_id.to_string(), &identity_id.to_string());
    sign_token(&claims, &state.signing_key_pem).unwrap()
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn req(
    app: Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let body = match body {
        Some(v) => Body::from(serde_json::to_vec(&v).unwrap()),
        None => Body::empty(),
    };
    let resp = app.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

fn b64(s: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

// ─── The test ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn blocks_full_lifecycle() {
    let (app, state) = boot().await;
    let token = make_token(&state).await;

    // 1. Create a board
    let (status, body) = req(
        app.clone(),
        Method::POST,
        "/boards",
        Some(json!({ "name": "E2E Board" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create board: {body}");
    let board_id = body["id"].as_str().unwrap().to_string();

    // 2. Create a text note on that board
    let (status, body) = req(
        app.clone(),
        Method::POST,
        "/notes",
        Some(json!({
            "note_type": "text",
            "board_id": board_id,
            "blob_key": Uuid::new_v4().to_string(),
            "size": 0,
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create note: {body}");
    let note_id = body["id"].as_str().unwrap().to_string();

    // 3. Create two blocks under the note
    let (status, body) = req(
        app.clone(),
        Method::POST,
        &format!("/notes/{note_id}/blocks"),
        Some(json!({
            "block_type": "text",
            "content_b64": b64("first block"),
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create block 1: {body}");
    let block1 = body["id"].as_str().unwrap().to_string();
    let block1_updated_at = body["updated_at"].as_str().unwrap().to_string();

    let (status, body) = req(
        app.clone(),
        Method::POST,
        &format!("/notes/{note_id}/blocks"),
        Some(json!({
            "block_type": "heading",
            "content_b64": b64("second block"),
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create block 2: {body}");
    let block2 = body["id"].as_str().unwrap().to_string();

    // 4. List blocks: 2 items, ordered by position ascending
    let (status, body) = req(
        app.clone(),
        Method::GET,
        &format!("/notes/{note_id}/blocks"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    let pos0 = arr[0]["position"].as_f64().unwrap();
    let pos1 = arr[1]["position"].as_f64().unwrap();
    assert!(pos0 < pos1, "positions must be ordered: {pos0} < {pos1}");

    // Wait a beat so updated_at can actually change (chrono millisecond resolution).
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // 5. PATCH block 1 content — updated_at must advance
    let (status, body) = req(
        app.clone(),
        Method::PATCH,
        &format!("/blocks/{block1}"),
        Some(json!({ "content_b64": b64("first block — edited") })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "patch block: {body}");
    let new_updated_at = body["updated_at"].as_str().unwrap();
    assert!(
        new_updated_at > block1_updated_at.as_str(),
        "updated_at must advance: {block1_updated_at} -> {new_updated_at}"
    );

    // 6. Move block 1 to be a child of block 2
    let (status, _) = req(
        app.clone(),
        Method::POST,
        &format!("/blocks/{block1}/move"),
        Some(json!({ "new_parent_id": block2, "new_position": 1.0 })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = req(
        app.clone(),
        Method::GET,
        &format!("/blocks/{block1}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["parent_block_id"].as_str().unwrap(), block2);

    // 7. Put one TAG link on block 1
    let (status, _) = req(
        app.clone(),
        Method::PUT,
        &format!("/blocks/{block1}/links"),
        Some(json!({
            "links": [
                { "target_kind": "tag", "target_id": "foo", "link_kind": "tag" }
            ]
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = req(
        app.clone(),
        Method::GET,
        "/tags/foo/blocks",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tagged = body.as_array().unwrap();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].as_str().unwrap(), block1);

    // 8. block 2 has no backlinks yet
    let (status, body) = req(
        app.clone(),
        Method::GET,
        &format!("/blocks/{block2}/backlinks"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);

    // 9. Create block 3 that links to block 1 → block 1 gains one backlink
    let (status, body) = req(
        app.clone(),
        Method::POST,
        &format!("/notes/{note_id}/blocks"),
        Some(json!({
            "block_type": "text",
            "content_b64": b64("third block points to first"),
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let block3 = body["id"].as_str().unwrap().to_string();

    let (status, _) = req(
        app.clone(),
        Method::PUT,
        &format!("/blocks/{block3}/links"),
        Some(json!({
            "links": [
                { "target_kind": "block", "target_id": block1, "link_kind": "block_ref" }
            ]
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = req(
        app.clone(),
        Method::GET,
        &format!("/blocks/{block1}/backlinks"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let backs = body.as_array().unwrap();
    assert_eq!(backs.len(), 1);
    assert_eq!(backs[0]["source_block_id"].as_str().unwrap(), block3);

    // 10. Delete block 1 — subsequent GET returns 404.
    let (status, _) = req(
        app.clone(),
        Method::DELETE,
        &format!("/blocks/{block1}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = req(
        app,
        Method::GET,
        &format!("/blocks/{block1}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
