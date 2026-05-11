use crate::auth::middleware::AuthenticatedDevice;
use crate::state::{AppState, WsEvent};
use crate::ApiError;
use storage::db::shares::permission_allows;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use jot_core::models::{Note, NoteType};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
pub struct NotesQuery {
    pub board_id: Uuid,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateNoteBody {
    /// "text" | "voice" | "image"
    pub note_type: String,
    pub color: Option<String>,
    pub board_id: Uuid,
    pub position: Option<i32>,
    pub blob_key: String,
    pub size: i64,
    /// Plaintext first-line preview (client-side excerpt, max ~80 chars)
    pub snippet: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct PatchNoteBody {
    pub position: Option<i32>,
    pub color: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct NoteMetadata {
    pub id: String,
    /// "text" | "voice" | "image"
    pub note_type: String,
    pub color: String,
    pub board_id: String,
    pub position: i32,
    pub blob_key: String,
    pub size: i64,
    pub created_at: String,
    pub updated_at: String,
    /// Present and true when this note has been shared with at least one identity
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub shared: bool,
    /// Plaintext first-line preview stored by the client at write time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    /// True when the note blob is E2E encrypted (a DEK exists for the caller)
    pub encrypted: bool,
    /// Block schema version: 0 = legacy single blob, 1 = block-structured
    pub schema_version: i32,
}

fn to_metadata(note: &Note) -> NoteMetadata {
    let snippet = if note.content.is_empty() {
        None
    } else {
        String::from_utf8(note.content.clone()).ok()
    };
    NoteMetadata {
        id: note.id.to_string(),
        note_type: match note.note_type {
            NoteType::Text => "text",
            NoteType::Voice => "voice",
            NoteType::Image => "image",
        }
        .to_string(),
        color: note.color.clone(),
        board_id: note.board_id.to_string(),
        position: note.position,
        blob_key: note.blob_key.clone(),
        size: note.size,
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
        shared: false,
        snippet,
        encrypted: false,
        schema_version: note.schema_version,
    }
}

fn parse_note_type(s: &str) -> Result<NoteType, ApiError> {
    match s {
        "text" => Ok(NoteType::Text),
        "voice" => Ok(NoteType::Voice),
        "image" => Ok(NoteType::Image),
        _ => Err(ApiError::BadRequest(format!("unknown note_type: {s}"))),
    }
}

#[utoipa::path(
    get,
    path = "/notes",
    tag = "notes",
    security(("bearer_auth" = [])),
    params(NotesQuery),
    responses(
        (status = 200, description = "List of notes for the board", body = Vec<NoteMetadata>),
        (status = 403, description = "No access to this board"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_notes(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Query(q): Query<NotesQuery>,
) -> Result<Json<Vec<NoteMetadata>>, ApiError> {
    let bid = q.board_id.to_string();
    if !state.db.can_access_board(&bid, &auth.0.identity_id).await? {
        return Err(ApiError::Forbidden("no access to this board".into()));
    }
    let notes = state.db.list_notes(q.board_id).await?;

    // Determine encryption status for the caller:
    // - owner: always encrypted (DEK derived locally, never stored)
    // - board member: encrypted when a BEK exists in board_keys
    // - individual share recipient: encrypted when a DEK exists in note_shares
    let is_owner = state.db.owns_board(&bid, &auth.0.identity_id).await?;
    let (shared_ids, all_encrypted) = if is_owner {
        let shared = state.db.list_shared_note_ids_for_board(&bid, &auth.0.identity_id).await?;
        (shared, true)
    } else {
        let (shared, has_bek) = tokio::try_join!(
            state.db.list_shared_note_ids_for_board(&bid, &auth.0.identity_id),
            state.db.has_board_key(&bid, &auth.0.identity_id),
        )?;
        (shared, has_bek)
    };

    // For individual share recipients (not owner, no BEK) fall back to note-level DEK check.
    let note_level_encrypted: std::collections::HashSet<String> = if !is_owner && !all_encrypted {
        state.db.list_encrypted_note_ids_for_board(&bid, &auth.0.identity_id).await?
    } else {
        std::collections::HashSet::new()
    };

    let shared_set: std::collections::HashSet<String> = shared_ids.into_iter().collect();
    Ok(Json(
        notes
            .iter()
            .map(|n| {
                let id = n.id.to_string();
                let mut m = to_metadata(n);
                m.shared = shared_set.contains(&id);
                m.encrypted = all_encrypted || note_level_encrypted.contains(&id);
                m
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/notes",
    tag = "notes",
    security(("bearer_auth" = [])),
    request_body = CreateNoteBody,
    responses(
        (status = 201, description = "Note created", body = serde_json::Value,
         example = json!({"id": "550e8400-e29b-41d4-a716-446655440000"})),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_note(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Json(body): Json<CreateNoteBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let now = Utc::now();
    let note = Note {
        id: Uuid::new_v4(),
        note_type: parse_note_type(&body.note_type)?,
        content: body.snippet.map(|s| s.into_bytes()).unwrap_or_default(),
        thumbnail: None,
        duration_ms: None,
        color: body.color.unwrap_or_else(|| "#FFFFFF".to_string()),
        board_id: body.board_id,
        position: body.position.unwrap_or(0),
        blob_key: body.blob_key,
        size: body.size,
        created_at: now,
        updated_at: now,
        title: None,
        is_journal: false,
        journal_date: None,
        schema_version: 0,
    };
    state.db.insert_note(&note).await?;
    let _ = state.ws_tx.send(WsEvent::NoteUpdated {
        id: note.id.to_string(),
    });
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": note.id.to_string() })),
    ))
}

#[utoipa::path(
    get,
    path = "/notes/{id}",
    tag = "notes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Note ID")),
    responses(
        (status = 200, description = "Note metadata", body = NoteMetadata),
        (status = 404, description = "Note not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_note(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<NoteMetadata>, ApiError> {
    let note = state.db.get_note(id).await?.ok_or(ApiError::NotFound)?;
    let bid = note.board_id.to_string();
    let is_owner = state.db.owns_board(&bid, &auth.0.identity_id).await?;
    let encrypted = if is_owner {
        true
    } else {
        let has_bek = state.db.has_board_key(&bid, &auth.0.identity_id).await?;
        if has_bek {
            true
        } else {
            state.db.get_note_dek(&id.to_string(), &auth.0.identity_id).await?.is_some()
        }
    };
    let mut m = to_metadata(&note);
    m.encrypted = encrypted;
    Ok(Json(m))
}

#[utoipa::path(
    delete,
    path = "/notes/{id}",
    tag = "notes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Note ID")),
    responses(
        (status = 204, description = "Note deleted"),
        (status = 404, description = "Note not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_note(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let note = state.db.get_note(id).await?.ok_or(ApiError::NotFound)?;
    let bid = note.board_id.to_string();
    let caller = &auth.0.identity_id;
    let is_owner = state.db.owns_board(&bid, caller).await?;
    if !is_owner {
        let perm = state
            .db
            .get_note_share_permission(&id.to_string(), caller)
            .await?
            .unwrap_or_default();
        if !permission_allows(&perm, "delete") {
            return Err(ApiError::Forbidden("delete permission required".into()));
        }
    }
    state.blobs.delete(&note.blob_key).await.ok();
    state.db.delete_note(id).await?;
    let _ = state
        .ws_tx
        .send(WsEvent::NoteDeleted { id: id.to_string() });
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    patch,
    path = "/notes/{id}",
    tag = "notes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Note ID")),
    request_body = PatchNoteBody,
    responses(
        (status = 200, description = "Note updated"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn patch_note(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchNoteBody>,
) -> Result<StatusCode, ApiError> {
    let note = state.db.get_note(id).await?.ok_or(ApiError::NotFound)?;
    let bid = note.board_id.to_string();
    let caller = &auth.0.identity_id;
    let is_owner = state.db.owns_board(&bid, caller).await?;
    if !is_owner {
        let perm = state
            .db
            .get_note_share_permission(&id.to_string(), caller)
            .await?
            .unwrap_or_default();
        if !permission_allows(&perm, "write") {
            return Err(ApiError::Forbidden("write permission required".into()));
        }
    }
    if let Some(pos) = body.position {
        state.db.update_note_position(id, pos).await?;
    }
    if let Some(color) = body.color {
        state.db.update_note_color(id, &color).await?;
    }
    if let Some(snippet) = body.snippet {
        state.db.update_note_snippet(id, snippet.as_bytes()).await?;
    }
    let _ = state
        .ws_tx
        .send(WsEvent::NoteUpdated { id: id.to_string() });
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/notes/{id}/blob",
    tag = "notes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Note ID")),
    responses(
        (status = 200, description = "Raw blob content (application/octet-stream)"),
        (status = 403, description = "No access to this board"),
        (status = 404, description = "Note not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_blob(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let note = state.db.get_note(id).await?.ok_or(ApiError::NotFound)?;
    let bid = note.board_id.to_string();
    let caller = &auth.0.identity_id;
    if !state.db.can_access_board(&bid, caller).await? {
        // Fall back to note-level share (individual share recipients have no board access).
        let perm = state
            .db
            .get_note_share_permission(&id.to_string(), caller)
            .await?
            .unwrap_or_default();
        if !permission_allows(&perm, "read") {
            return Err(ApiError::Forbidden("no access to this note".into()));
        }
    }
    let data = state.blobs.get(&note.blob_key).await?;
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], data))
}

#[utoipa::path(
    put,
    path = "/notes/{id}/blob",
    tag = "notes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Note ID")),
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    responses(
        (status = 200, description = "Blob uploaded"),
        (status = 403, description = "Board is read-only"),
        (status = 404, description = "Note not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn put_blob(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let note = state.db.get_note(id).await?.ok_or(ApiError::NotFound)?;
    let bid = note.board_id.to_string();
    let caller = &auth.0.identity_id;
    let is_owner = state.db.owns_board(&bid, caller).await?;
    if !is_owner {
        let perm = state
            .db
            .get_note_share_permission(&id.to_string(), caller)
            .await?
            .unwrap_or_default();
        if !permission_allows(&perm, "write") {
            return Err(ApiError::Forbidden("write permission required".into()));
        }
    }
    state.blobs.put(&note.blob_key, &body).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize, ToSchema)]
pub struct PatchSchemaVersionBody {
    pub schema_version: i32,
}

pub async fn patch_schema_version(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchSchemaVersionBody>,
) -> Result<StatusCode, ApiError> {
    let perm = state
        .db
        .note_permission_for(&id.to_string(), &auth.0.identity_id)
        .await?;
    if !permission_allows(&perm, "write") {
        return Err(ApiError::Forbidden(
            "no write permission on note".into(),
        ));
    }
    state.db.set_note_schema_version(id, body.schema_version).await?;
    let _ = state
        .ws_tx
        .send(WsEvent::NoteUpdated { id: id.to_string() });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_legacy_text_notes(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<Vec<String>>, ApiError> {
    let identity = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity id".into()))?;
    let ids = state.db.list_legacy_text_notes_for_identity(identity).await?;
    Ok(Json(ids.into_iter().map(|i| i.to_string()).collect()))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_app;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn list_notes_without_token_returns_401() {
        let app = test_app().await;
        let board_id = Uuid::new_v4();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/notes?board_id={}", board_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
