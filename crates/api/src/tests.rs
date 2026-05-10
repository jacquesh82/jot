use crate::test_helpers::{test_app_with_state, test_token, test_token_with_identity};
use axum::http::{Method, Request, StatusCode};
use axum::{body::Body, Router};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

// ─── Low-level HTTP helpers ───────────────────────────────────────────────────

async fn api_get(app: Router, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(Method::GET).uri(uri);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn api_post(app: Router, uri: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let resp = app
        .oneshot(
            req.body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn api_patch(app: Router, uri: &str, body: Value, token: &str) -> (StatusCode, Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn api_delete(app: Router, uri: &str, token: &str) -> StatusCode {
    app.oneshot(
        Request::builder()
            .method(Method::DELETE)
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}

async fn api_put_bytes(app: Router, uri: &str, data: Vec<u8>, token: &str) -> StatusCode {
    app.oneshot(
        Request::builder()
            .method(Method::PUT)
            .uri(uri)
            .header("content-type", "application/octet-stream")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(data))
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}

async fn api_get_bytes(app: Router, uri: &str, token: &str) -> (StatusCode, Vec<u8>) {
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, bytes)
}

// ─── Domain helpers ───────────────────────────────────────────────────────────

async fn create_board(app: Router, token: &str, name: &str) -> String {
    let (status, body) = api_post(app, "/boards", json!({ "name": name }), Some(token)).await;
    assert_eq!(status, StatusCode::CREATED, "create_board failed: {body}");
    body["id"].as_str().unwrap().to_owned()
}

async fn create_note(app: Router, token: &str, board_id: &str) -> String {
    let blob_key = Uuid::new_v4().to_string();
    let (status, body) = api_post(
        app,
        "/notes",
        json!({
            "note_type": "text",
            "board_id": board_id,
            "blob_key": blob_key,
            "size": 0,
        }),
        Some(token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create_note failed: {body}");
    body["id"].as_str().unwrap().to_owned()
}

// ─── Health ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_includes_schema_version() {
    let app = test_app_with_state().await.0;
    let (status, body) = api_get(app, "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(body["schema_version"].is_number());
}

// ─── Register ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn register_open_creates_account() {
    let (_, state) = test_app_with_state().await;
    let state = state.with_open_registration(true);
    let app = crate::build_router(state);
    let (status, body) = api_post(app, "/register", json!({}), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["jwt"].as_str().is_some());
    assert!(body["identity_id"].as_str().is_some());
    assert!(body["device_id"].as_str().is_some());
}

#[tokio::test]
async fn register_closed_without_invite_returns_403() {
    let app = test_app_with_state().await.0;
    let (status, _) = api_post(app, "/register", json!({}), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn register_with_valid_invite_succeeds() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token_with_identity(&state).await;

    // Create invite
    let (status, invite) = api_post(
        app.clone(),
        "/invites",
        json!({ "label": "friend" }),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let invite_token = invite["token"].as_str().unwrap().to_owned();

    // Register with it
    let (status, body) = api_post(
        app,
        "/register",
        json!({ "invite_token": invite_token }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["jwt"].as_str().is_some());
}

#[tokio::test]
async fn register_with_revoked_invite_returns_403() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token_with_identity(&state).await;

    let (_, invite) = api_post(app.clone(), "/invites", json!({}), Some(&token)).await;
    let invite_token = invite["token"].as_str().unwrap().to_owned();

    // Revoke it
    api_delete(app.clone(), &format!("/invites/{invite_token}"), &token).await;

    // Try to register
    let (status, _) = api_post(
        app,
        "/register",
        json!({ "invite_token": invite_token }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── Identity ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_me_returns_identity() {
    let (app, state) = test_app_with_state().await;
    let (token, _, identity_id) = test_token_with_identity(&state).await;
    let (status, body) = api_get(app, "/identity/me", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"].as_str().unwrap(), identity_id);
    assert!(body["friendly_name"].as_str().is_some());
}

#[tokio::test]
async fn update_me_changes_friendly_name() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token_with_identity(&state).await;
    let (status, body) = api_patch(
        app,
        "/identity/me",
        json!({ "friendly_name": "alice" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["friendly_name"], "alice");
}

#[tokio::test]
async fn update_me_duplicate_name_returns_400() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (token_b, _, _) = test_token_with_identity(&state).await;

    // User A takes the name "bob"
    api_patch(
        app.clone(),
        "/identity/me",
        json!({ "friendly_name": "bob" }),
        &token_a,
    )
    .await;

    // User B tries to take the same name
    let (status, _) = api_patch(
        app,
        "/identity/me",
        json!({ "friendly_name": "bob" }),
        &token_b,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn lookup_by_name_finds_identity() {
    let (app, state) = test_app_with_state().await;
    let (token, _, identity_id) = test_token_with_identity(&state).await;

    api_patch(
        app.clone(),
        "/identity/me",
        json!({ "friendly_name": "charlie" }),
        &token,
    )
    .await;

    let (status, body) = api_get(app, "/identity/lookup/charlie", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"].as_str().unwrap(), identity_id);
}

#[tokio::test]
async fn lookup_by_name_case_insensitive() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token_with_identity(&state).await;

    api_patch(
        app.clone(),
        "/identity/me",
        json!({ "friendly_name": "Dave" }),
        &token,
    )
    .await;

    let (status, body) = api_get(app, "/identity/lookup/dave", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["friendly_name"], "Dave");
}

// ─── Boards ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_board_returns_201_with_id() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let (status, body) = api_post(app, "/boards", json!({ "name": "Work" }), Some(&token)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(Uuid::parse_str(body["id"].as_str().unwrap()).is_ok());
}

#[tokio::test]
async fn list_boards_returns_only_owned() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token(&state).await;
    let (token_b, _, _) = test_token(&state).await;

    api_post(
        app.clone(),
        "/boards",
        json!({ "name": "A1" }),
        Some(&token_a),
    )
    .await;
    api_post(
        app.clone(),
        "/boards",
        json!({ "name": "A2" }),
        Some(&token_a),
    )
    .await;
    api_post(
        app.clone(),
        "/boards",
        json!({ "name": "B1" }),
        Some(&token_b),
    )
    .await;

    let (status, body) = api_get(app, "/boards", Some(&token_a)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn patch_board_updates_name() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "Old Name").await;

    let status = api_patch(
        app,
        &format!("/boards/{board_id}"),
        json!({ "name": "New Name" }),
        &token,
    )
    .await
    .0;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn delete_board_returns_204() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "To Delete").await;

    let status = api_delete(app.clone(), &format!("/boards/{board_id}"), &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = api_get(app, "/boards", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn boards_require_auth() {
    let app = test_app_with_state().await.0;
    let (status, _) = api_get(app, "/boards", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ─── Notes ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_note_returns_id() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "Notes Board").await;
    let note_id = create_note(app, &token, &board_id).await;
    assert!(Uuid::parse_str(&note_id).is_ok());
}

#[tokio::test]
async fn list_notes_returns_notes_for_board() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "Board").await;

    create_note(app.clone(), &token, &board_id).await;
    create_note(app.clone(), &token, &board_id).await;

    let (status, body) = api_get(app, &format!("/notes?board_id={board_id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_note_returns_metadata() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "Board").await;
    let note_id = create_note(app.clone(), &token, &board_id).await;

    let (status, body) = api_get(app, &format!("/notes/{note_id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"].as_str().unwrap(), note_id);
    assert_eq!(body["note_type"].as_str().unwrap(), "text");
}

#[tokio::test]
async fn delete_note_returns_204() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "Board").await;
    let note_id = create_note(app.clone(), &token, &board_id).await;

    let status = api_delete(app.clone(), &format!("/notes/{note_id}"), &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = api_get(app, &format!("/notes/{note_id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_note_updates_color() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "Board").await;
    let note_id = create_note(app.clone(), &token, &board_id).await;

    let (status, _) = api_patch(
        app,
        &format!("/notes/{note_id}"),
        json!({ "color": "#FF0000" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn list_notes_forbidden_for_other_users_board() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token(&state).await;
    let (token_b, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token_a, "Private").await;

    let (status, _) = api_get(app, &format!("/notes?board_id={board_id}"), Some(&token_b)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── Blob ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn put_blob_then_get_blob_returns_same_content() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token, "Board").await;
    let note_id = create_note(app.clone(), &token, &board_id).await;

    let content = b"hello, jot!".to_vec();
    let status = api_put_bytes(
        app.clone(),
        &format!("/notes/{note_id}/blob"),
        content.clone(),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, returned) = api_get_bytes(app, &format!("/notes/{note_id}/blob"), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(returned, content);
}

#[tokio::test]
async fn get_blob_forbidden_for_other_user() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token(&state).await;
    let (token_b, _, _) = test_token(&state).await;
    let board_id = create_board(app.clone(), &token_a, "Private").await;
    let note_id = create_note(app.clone(), &token_a, &board_id).await;

    let (status, _) = api_get_bytes(app, &format!("/notes/{note_id}/blob"), &token_b).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── Note shares ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn share_note_and_list_shares() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (_, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Board").await;
    let note_id = create_note(app.clone(), &token_a, &board_id).await;

    let (status, _) = api_post(
        app.clone(),
        &format!("/notes/{note_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, shares) = api_get(app, &format!("/notes/{note_id}/shares"), Some(&token_a)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = shares.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["shared_with_id"].as_str().unwrap(), identity_b);
}

#[tokio::test]
async fn get_notes_shared_with_me() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (token_b, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Board").await;
    let note_id = create_note(app.clone(), &token_a, &board_id).await;

    api_post(
        app.clone(),
        &format!("/notes/{note_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;

    let (status, body) = api_get(app, "/notes/shared", Some(&token_b)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["note_id"].as_str().unwrap(), note_id);
}

#[tokio::test]
async fn delete_note_share_revokes_access() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (_token_b, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Board").await;
    let note_id = create_note(app.clone(), &token_a, &board_id).await;

    api_post(
        app.clone(),
        &format!("/notes/{note_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;

    let status = api_delete(
        app.clone(),
        &format!("/notes/{note_id}/shares/{identity_b}"),
        &token_a,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, shares) = api_get(app, &format!("/notes/{note_id}/shares"), Some(&token_a)).await;
    assert_eq!(shares.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn cannot_share_note_with_yourself() {
    let (app, state) = test_app_with_state().await;
    let (token, _, identity_id) = test_token_with_identity(&state).await;
    let board_id = create_board(app.clone(), &token, "Board").await;
    let note_id = create_note(app.clone(), &token, &board_id).await;

    let (status, _) = api_post(
        app,
        &format!("/notes/{note_id}/shares"),
        json!({ "target": identity_id }),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ─── Board shares ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn share_board_and_list_shares() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (_, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Shared Board").await;

    let (status, _) = api_post(
        app.clone(),
        &format!("/boards/{board_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, shares) =
        api_get(app, &format!("/boards/{board_id}/shares"), Some(&token_a)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = shares.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["shared_with_id"].as_str().unwrap(), identity_b);
}

#[tokio::test]
async fn get_boards_shared_with_me() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (token_b, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Shared").await;

    api_post(
        app.clone(),
        &format!("/boards/{board_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;

    let (status, body) = api_get(app, "/boards/shared", Some(&token_b)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["board_id"].as_str().unwrap(), board_id);
}

#[tokio::test]
async fn shared_board_notes_accessible_to_recipient() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (token_b, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Board").await;
    create_note(app.clone(), &token_a, &board_id).await;

    api_post(
        app.clone(),
        &format!("/boards/{board_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;

    // Recipient can read notes
    let (status, body) = api_get(app, &format!("/notes?board_id={board_id}"), Some(&token_b)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn revoke_board_share_removes_access() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (token_b, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Board").await;

    api_post(
        app.clone(),
        &format!("/boards/{board_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;

    api_delete(
        app.clone(),
        &format!("/boards/{board_id}/shares/{identity_b}"),
        &token_a,
    )
    .await;

    let (status, _) = api_get(app, &format!("/notes?board_id={board_id}"), Some(&token_b)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── Invites ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_invite_returns_token() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token_with_identity(&state).await;
    let (status, body) = api_post(app, "/invites", json!({ "label": "alice" }), Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["token"].as_str().is_some());
    assert_eq!(body["label"], "alice");
}

#[tokio::test]
async fn list_invites_returns_created_tokens() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token_with_identity(&state).await;

    api_post(app.clone(), "/invites", json!({}), Some(&token)).await;
    api_post(app.clone(), "/invites", json!({}), Some(&token)).await;

    let (status, body) = api_get(app, "/invites", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn revoke_invite_sets_revoked_at() {
    let (app, state) = test_app_with_state().await;
    let (token, _, _) = test_token_with_identity(&state).await;

    let (_, invite) = api_post(app.clone(), "/invites", json!({}), Some(&token)).await;
    let invite_token = invite["token"].as_str().unwrap();

    let status = api_delete(app.clone(), &format!("/invites/{invite_token}"), &token).await;
    assert_eq!(status, StatusCode::OK);

    let (_, invites) = api_get(app, "/invites", Some(&token)).await;
    let first = &invites.as_array().unwrap()[0];
    assert!(first["revoked_at"].as_str().is_some());
}

// ─── Devices ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_devices_returns_own_device() {
    let (app, state) = test_app_with_state().await;
    let (token, device_id, _) = test_token(&state).await;

    let (status, body) = api_get(app, "/devices", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str().unwrap(), device_id);
}

#[tokio::test]
async fn rename_device_returns_200() {
    let (app, state) = test_app_with_state().await;
    let (token, device_id, _) = test_token(&state).await;

    let (status, _) = api_post(
        app,
        &format!("/devices/{device_id}/rename"),
        json!({ "name": "My Laptop" }),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn delete_device_returns_204() {
    let (app, state) = test_app_with_state().await;
    let (token, device_id, _) = test_token(&state).await;

    let status = api_delete(app.clone(), &format!("/devices/{device_id}"), &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Device gone — subsequent request should 401 (middleware rejects deleted device)
    let (status, _) = api_get(app, "/boards", Some(&token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ─── Export ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn export_returns_boards_and_notes() {
    let (app, state) = test_app_with_state().await;
    let (token, _, identity_id) = test_token_with_identity(&state).await;
    let board_id = create_board(app.clone(), &token, "Export Board").await;
    create_note(app.clone(), &token, &board_id).await;

    let (status, body) = api_get(app, "/export", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["identity"]["id"].as_str().unwrap(), identity_id);
    let boards = body["boards"].as_array().unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0]["id"].as_str().unwrap(), board_id);
    assert_eq!(boards[0]["notes"].as_array().unwrap().len(), 1);
}

// ─── Contacts ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn contacts_appear_after_board_share() {
    let (app, state) = test_app_with_state().await;
    let (token_a, _, _) = test_token_with_identity(&state).await;
    let (_, _, identity_b) = test_token_with_identity(&state).await;

    let board_id = create_board(app.clone(), &token_a, "Board").await;
    api_post(
        app.clone(),
        &format!("/boards/{board_id}/shares"),
        json!({ "target": identity_b }),
        Some(&token_a),
    )
    .await;

    let (status, body) = api_get(app, "/identity/contacts", Some(&token_a)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert!(arr.iter().any(|c| c["id"].as_str() == Some(&identity_b)));
}
