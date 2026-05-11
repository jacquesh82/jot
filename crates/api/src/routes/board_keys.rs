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

#[derive(Deserialize)]
pub struct PutBekBody {
    /// Hex-encoded BEK encrypted for the recipient with ECDH(owner_priv, recipient_pub)→HKDF→AES-GCM
    pub encrypted_bek: String,
}

#[derive(Serialize)]
pub struct GetBekResponse {
    /// Hex-encoded encrypted BEK
    pub encrypted_bek: String,
    /// Hex-encoded X25519 public key of the board owner (needed to derive the ECDH wrap key)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_pubkey_x25519: Option<String>,
}

/// GET /boards/:id/key — member retrieves their encrypted BEK.
pub async fn get_board_key(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(board_id): Path<Uuid>,
) -> Result<Json<GetBekResponse>, ApiError> {
    let bid = board_id.to_string();

    // Must be a member (not the owner — owner derives locally).
    let board = state.db.get_board(board_id).await?.ok_or(ApiError::NotFound)?;
    if board.identity_id.to_string() == claims.identity_id {
        return Err(ApiError::BadRequest("owner derives BEK locally".into()));
    }
    let has_access = state.db.can_access_board(&bid, &claims.identity_id).await?;
    if !has_access {
        return Err(ApiError::Forbidden("not a board member".into()));
    }

    let bek = state
        .db
        .get_board_key(&bid, &claims.identity_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Include owner pubkey so the recipient can verify the ECDH derivation.
    let owner_pubkey = state
        .db
        .get_identity_by_id(&board.identity_id.to_string())
        .await?
        .and_then(|i| i.public_key_x25519)
        .map(|b| hex::encode(b));

    Ok(Json(GetBekResponse {
        encrypted_bek: hex::encode(bek),
        owner_pubkey_x25519: owner_pubkey,
    }))
}

/// PUT /boards/:id/keys/:identity_id — owner stores encrypted BEK for a board member.
pub async fn put_board_key(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path((board_id, identity_id)): Path<(Uuid, String)>,
    Json(body): Json<PutBekBody>,
) -> Result<StatusCode, ApiError> {
    let bid = board_id.to_string();

    if !state.db.owns_board(&bid, &claims.identity_id).await? {
        return Err(ApiError::Forbidden("only the board owner can distribute keys".into()));
    }
    if identity_id == claims.identity_id {
        return Err(ApiError::BadRequest("owner derives BEK locally, not stored".into()));
    }

    let bek_bytes = hex::decode(&body.encrypted_bek)
        .map_err(|_| ApiError::BadRequest("encrypted_bek must be hex".into()))?;

    state.db.put_board_key(&bid, &identity_id, bek_bytes).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /boards/:id/keys/:identity_id — owner removes a member's BEK (revoke access).
pub async fn delete_board_key(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path((board_id, identity_id)): Path<(Uuid, String)>,
) -> Result<StatusCode, ApiError> {
    let bid = board_id.to_string();

    if !state.db.owns_board(&bid, &claims.identity_id).await? {
        return Err(ApiError::Forbidden("only the board owner can revoke keys".into()));
    }

    state.db.delete_board_key(&bid, &identity_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
