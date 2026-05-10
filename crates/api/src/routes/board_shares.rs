use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ShareBoardBody {
    pub target: String, // UUID or friendly name
}

#[derive(Serialize)]
pub struct BoardShareResponse {
    pub shared_with_id: String,
    pub shared_with_name: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SharedBoardResponse {
    pub board_id: String,
    pub board_name: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
}

pub async fn list_board_shares(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(board_id): Path<Uuid>,
) -> Result<Json<Vec<BoardShareResponse>>, ApiError> {
    let bid = board_id.to_string();
    if !state.db.owns_board(&bid, &auth.0.identity_id).await? {
        return Err(ApiError::Forbidden("not the board owner".into()));
    }
    let shares = state.db.get_board_shares(&bid).await?;
    Ok(Json(
        shares
            .into_iter()
            .map(|s| BoardShareResponse {
                shared_with_id: s.shared_with_id,
                shared_with_name: s.shared_with_name,
                created_at: s.created_at,
            })
            .collect(),
    ))
}

pub async fn share_board(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(board_id): Path<Uuid>,
    Json(body): Json<ShareBoardBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let bid = board_id.to_string();
    if !state.db.owns_board(&bid, &auth.0.identity_id).await? {
        return Err(ApiError::Forbidden("not the board owner".into()));
    }

    // Resolve target: try UUID first, then friendly name
    let target_id = if let Ok(uid) = Uuid::parse_str(&body.target) {
        uid.to_string()
    } else {
        state
            .db
            .get_identity_by_name(&body.target)
            .await?
            .ok_or_else(|| ApiError::NotFound)?
            .id
    };

    if target_id == auth.0.identity_id {
        return Err(ApiError::BadRequest("cannot share with yourself".into()));
    }

    state
        .db
        .share_board(&bid, &auth.0.identity_id, &target_id)
        .await?;
    Ok(Json(
        serde_json::json!({ "status": "shared", "shared_with_id": target_id }),
    ))
}

pub async fn revoke_board_share(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path((board_id, identity_id)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let bid = board_id.to_string();
    if !state.db.owns_board(&bid, &auth.0.identity_id).await? {
        return Err(ApiError::Forbidden("not the board owner".into()));
    }
    let ok = state.db.delete_board_share(&bid, &identity_id).await?;
    if ok {
        Ok(Json(serde_json::json!({ "status": "revoked" })))
    } else {
        Err(ApiError::NotFound)
    }
}

pub async fn get_boards_shared_with_me(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<Vec<SharedBoardResponse>>, ApiError> {
    let boards = state
        .db
        .get_boards_shared_with_me(&auth.0.identity_id)
        .await?;
    Ok(Json(
        boards
            .into_iter()
            .map(|b| SharedBoardResponse {
                board_id: b.board_id,
                board_name: b.board_name,
                owner_identity_id: b.owner_identity_id,
                owner_friendly_name: b.owner_friendly_name,
            })
            .collect(),
    ))
}
