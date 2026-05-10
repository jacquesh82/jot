use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct ShareEntry {
    pub shared_with_id: String,
    pub shared_with_name: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct ShareBody {
    /// Target identity UUID or friendly name
    pub target: String,
}

#[derive(Serialize)]
pub struct SharedNoteEntry {
    pub note_id: String,
    pub note_type: String,
    pub board_id: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
}

/// GET /notes/shared — notes shared with the current identity
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

/// GET /notes/:id/shares — list who a note is shared with
pub async fn list_shares(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
) -> Result<Json<Vec<ShareEntry>>, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    // Verify ownership: note's board must belong to caller's identity
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

/// POST /notes/:id/share — share a note with another identity
pub async fn share_note(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
    Json(body): Json<ShareBody>,
) -> Result<StatusCode, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    // Verify ownership
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
    // Resolve target: try as UUID first, then as friendly name
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

/// DELETE /notes/:id/shares/:identity_id — revoke sharing
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
