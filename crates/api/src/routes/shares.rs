use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ShareEntry {
    pub shared_with_id: String,
    pub shared_with_name: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ShareBody {
    /// Target identity UUID or friendly name
    pub target: String,
}

#[derive(Serialize, ToSchema)]
pub struct SharedNoteEntry {
    pub note_id: String,
    pub note_type: String,
    pub board_id: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
}

#[utoipa::path(
    get,
    path = "/notes/shared",
    tag = "shares",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Notes shared with the current identity", body = Vec<SharedNoteEntry>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_shared_with_me(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
) -> Result<Json<Vec<SharedNoteEntry>>, ApiError> {
    let rows = state.db.get_shared_with_me(&claims.identity_id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| SharedNoteEntry {
                note_id: r.note_id,
                note_type: r.note_type,
                board_id: r.board_id,
                owner_identity_id: r.owner_identity_id,
                owner_friendly_name: r.owner_friendly_name,
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/notes/{id}/shares",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Note ID")),
    responses(
        (status = 200, description = "List of share entries for this note", body = Vec<ShareEntry>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found")
    )
)]
pub async fn list_shares(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
) -> Result<Json<Vec<ShareEntry>>, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state
        .db
        .get_note(note_uuid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let board = state
        .db
        .get_board(note.board_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if board.identity_id.to_string() != claims.identity_id {
        return Err(ApiError::Unauthorized);
    }
    let shares = state.db.get_note_shares(&note_id).await?;
    let mut entries = Vec::new();
    for s in shares {
        let name = state
            .db
            .get_identity_by_id(&s.shared_with_id)
            .await?
            .map(|i| i.friendly_name);
        entries.push(ShareEntry {
            shared_with_id: s.shared_with_id,
            shared_with_name: name,
            created_at: s.created_at,
        });
    }
    Ok(Json(entries))
}

#[utoipa::path(
    post,
    path = "/notes/{id}/shares",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Note ID")),
    request_body = ShareBody,
    responses(
        (status = 201, description = "Note shared"),
        (status = 400, description = "Invalid target or sharing with yourself"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note or target identity not found")
    )
)]
pub async fn share_note(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
    Json(body): Json<ShareBody>,
) -> Result<StatusCode, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state
        .db
        .get_note(note_uuid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let board = state
        .db
        .get_board(note.board_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if board.identity_id.to_string() != claims.identity_id {
        return Err(ApiError::Unauthorized);
    }
    let target_id = if let Ok(uuid) = Uuid::parse_str(&body.target) {
        uuid.to_string()
    } else {
        state
            .db
            .get_identity_by_name(&body.target)
            .await?
            .ok_or_else(|| ApiError::BadRequest("identity not found".into()))?
            .id
    };
    if target_id == claims.identity_id {
        return Err(ApiError::BadRequest("cannot share with yourself".into()));
    }
    state
        .db
        .share_note(&note_id, &claims.identity_id, &target_id, None)
        .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete,
    path = "/notes/{id}/shares/{identity_id}",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Note ID"),
        ("identity_id" = String, Path, description = "Identity to revoke access from")
    ),
    responses(
        (status = 204, description = "Share revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found")
    )
)]
pub async fn delete_share(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path((note_id, target_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state
        .db
        .get_note(note_uuid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let board = state
        .db
        .get_board(note.board_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if board.identity_id.to_string() != claims.identity_id {
        return Err(ApiError::Unauthorized);
    }
    state.db.delete_share(&note_id, &target_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
