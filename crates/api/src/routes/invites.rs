use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use storage::db::invites::InviteToken;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateInviteBody {
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct InviteResponse {
    pub token: String,
    pub label: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

fn to_response(t: InviteToken) -> InviteResponse {
    InviteResponse {
        token: t.token,
        label: t.label,
        created_at: t.created_at,
        revoked_at: t.revoked_at,
    }
}

pub async fn create_invite(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Json(body): Json<CreateInviteBody>,
) -> Result<Json<InviteResponse>, ApiError> {
    let token = Uuid::new_v4().to_string();
    let label = body.label.unwrap_or_default();
    let invite = state
        .db
        .create_invite_token(&token, &claims.identity_id, &label)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(to_response(invite)))
}

pub async fn list_invites(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
) -> Result<Json<Vec<InviteResponse>>, ApiError> {
    let tokens = state
        .db
        .list_invite_tokens(&claims.identity_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(tokens.into_iter().map(to_response).collect()))
}

pub async fn revoke_invite(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ok = state
        .db
        .revoke_invite_token(&token, &claims.identity_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    if ok {
        Ok(Json(serde_json::json!({ "status": "revoked" })))
    } else {
        Err(ApiError::NotFound)
    }
}
