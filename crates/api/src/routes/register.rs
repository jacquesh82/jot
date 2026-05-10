use crate::auth::{make_claims, sign_token};
use crate::state::AppState;
use crate::ApiError;
use axum::{extract::State, Json};
use chrono::Utc;
use jot_core::models::Device;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct RegisterBody {
    /// Invite token (required unless server runs with --open-registration)
    pub invite_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct RegisterResponse {
    pub jwt: String,
    pub identity_id: String,
    pub device_id: String,
}

#[utoipa::path(
    post,
    path = "/register",
    tag = "auth",
    request_body = RegisterBody,
    responses(
        (status = 200, description = "Identity and device created", body = RegisterResponse),
        (status = 403, description = "Invite required or invalid")
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<RegisterResponse>, ApiError> {
    if !state.open_registration {
        match &body.invite_token {
            None => return Err(ApiError::Forbidden("invite_required".to_string())),
            Some(tok) => {
                let invite = state
                    .db
                    .get_invite_token(tok)
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?
                    .ok_or_else(|| ApiError::Forbidden("invalid_invite".to_string()))?;
                if invite.revoked_at.is_some() {
                    return Err(ApiError::Forbidden("invite_revoked".to_string()));
                }
            }
        }
    }

    let identity_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();
    let friendly_name = format!("user-{}", &identity_id.to_string()[..8]);

    state
        .db
        .insert_identity(&identity_id.to_string(), &friendly_name)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let device = Device {
        id: device_id,
        identity_id,
        pub_key_x25519: String::new(),
        pub_key_ed25519: String::new(),
        name: "web".to_string(),
        last_seen: Utc::now(),
    };
    state
        .db
        .insert_device(&device)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let claims = make_claims(&device_id.to_string(), &identity_id.to_string());
    let jwt = sign_token(&claims, &state.signing_key_pem)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(RegisterResponse {
        jwt,
        identity_id: identity_id.to_string(),
        device_id: device_id.to_string(),
    }))
}
