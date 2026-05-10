use crate::auth::middleware::AuthenticatedDevice;
use crate::state::{AppState, WsEvent};
use crate::ApiError;
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
}

#[derive(Deserialize, ToSchema)]
pub struct PatchNoteBody {
    pub position: Option<i32>,
    pub color: Option<String>,
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
}

fn to_metadata(note: &Note) -> NoteMetadata {
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
    let shared_ids: std::collections::HashSet<String> = state
        .db
        .list_shared_note_ids_for_board(&bid, &auth.0.identity_id)
        .await?
        .into_iter()
        .collect();
    Ok(Json(
        notes
            .iter()
            .map(|n| {
                let mut m = to_metadata(n);
                m.shared = shared_ids.contains(&n.id.to_string());
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
        content: vec![],
        thumbnail: None,
        duration_ms: None,
        color: body.color.unwrap_or_else(|| "#FFFFFF".to_string()),
        board_id: body.board_id,
        position: body.position.unwrap_or(0),
        blob_key: body.blob_key,
        size: body.size,
        created_at: now,
        updated_at: now,
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
    _auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<NoteMetadata>, ApiError> {
    let note = state.db.get_note(id).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(to_metadata(&note)))
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
    _auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let note = state.db.get_note(id).await?.ok_or(ApiError::NotFound)?;
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
    _auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchNoteBody>,
) -> Result<StatusCode, ApiError> {
    if let Some(pos) = body.position {
        state.db.update_note_position(id, pos).await?;
    }
    if let Some(color) = body.color {
        state.db.update_note_color(id, &color).await?;
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
    if !state.db.can_access_board(&bid, &auth.0.identity_id).await? {
        return Err(ApiError::Forbidden("no access to this board".into()));
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
    if !state.db.owns_board(&bid, &auth.0.identity_id).await? {
        return Err(ApiError::Forbidden("board is read-only".into()));
    }
    state.blobs.put(&note.blob_key, &body).await?;
    Ok(StatusCode::OK)
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
