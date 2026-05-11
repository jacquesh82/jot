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

#[derive(Serialize, ToSchema)]
pub struct IdentityResponse {
    pub id: String,
    pub friendly_name: String,
    /// Hex-encoded X25519 public key, if registered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key_x25519: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterPubkeyBody {
    /// Hex-encoded 32-byte X25519 public key
    pub public_key_x25519: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateNameBody {
    /// 1–40 characters, must be unique across all identities
    pub friendly_name: String,
}

#[utoipa::path(
    get,
    path = "/identity/me",
    tag = "identity",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current identity", body = IdentityResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
) -> Result<Json<IdentityResponse>, ApiError> {
    let identity = state
        .db
        .get_identity_by_id(&claims.identity_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(IdentityResponse {
        id: identity.id.clone(),
        friendly_name: identity.friendly_name,
        public_key_x25519: identity
            .public_key_x25519
            .as_deref()
            .map(hex::encode),
    }))
}

#[utoipa::path(
    patch,
    path = "/identity/me",
    tag = "identity",
    security(("bearer_auth" = [])),
    request_body = UpdateNameBody,
    responses(
        (status = 200, description = "Identity updated", body = IdentityResponse),
        (status = 400, description = "Name empty, too long, or already taken"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Json(body): Json<UpdateNameBody>,
) -> Result<Json<IdentityResponse>, ApiError> {
    let name = body.friendly_name.trim().to_string();
    if name.is_empty() || name.len() > 40 {
        return Err(ApiError::BadRequest(
            "friendly_name must be 1–40 characters".into(),
        ));
    }
    if let Some(existing) = state.db.get_identity_by_name(&name).await? {
        if existing.id != claims.identity_id {
            return Err(ApiError::BadRequest("friendly_name already taken".into()));
        }
    }
    let updated = state
        .db
        .update_identity_name(&claims.identity_id, &name)
        .await?;
    if !updated {
        state.db.insert_identity(&claims.identity_id, &name).await?;
    }
    let identity = state
        .db
        .get_identity_by_id(&claims.identity_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(IdentityResponse {
        id: claims.identity_id,
        friendly_name: name,
        public_key_x25519: identity
            .public_key_x25519
            .as_deref()
            .map(hex::encode),
    }))
}

#[utoipa::path(
    put,
    path = "/identity/me/pubkey",
    tag = "identity",
    security(("bearer_auth" = [])),
    request_body = RegisterPubkeyBody,
    responses(
        (status = 204, description = "Public key registered"),
        (status = 400, description = "Invalid key format"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn set_pubkey(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Json(body): Json<RegisterPubkeyBody>,
) -> Result<StatusCode, ApiError> {
    let bytes = hex::decode(&body.public_key_x25519)
        .map_err(|_| ApiError::BadRequest("public_key_x25519 must be hex-encoded".into()))?;
    if bytes.len() != 32 {
        return Err(ApiError::BadRequest(
            "public_key_x25519 must be 32 bytes".into(),
        ));
    }
    state
        .db
        .set_identity_pubkey(&claims.identity_id, &bytes)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/identity/contacts",
    tag = "identity",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Recent contacts (identities shared with)", body = Vec<IdentityResponse>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_recent_contacts(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
) -> Result<Json<Vec<IdentityResponse>>, ApiError> {
    let contacts = state.db.get_recent_contacts(&claims.identity_id).await?;
    Ok(Json(
        contacts
            .into_iter()
            .map(|c| IdentityResponse {
                id: c.id,
                friendly_name: c.friendly_name,
                public_key_x25519: c.public_key_x25519.as_deref().map(hex::encode),
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/identity/lookup/{name}",
    tag = "identity",
    security(("bearer_auth" = [])),
    params(("name" = String, Path, description = "Friendly name to look up (case-insensitive)")),
    responses(
        (status = 200, description = "Identity found", body = IdentityResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn lookup_by_name(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(name): Path<String>,
) -> Result<Json<IdentityResponse>, ApiError> {
    let identity = state
        .db
        .get_identity_by_name(&name)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(IdentityResponse {
        id: identity.id,
        friendly_name: identity.friendly_name.clone(),
        public_key_x25519: identity.public_key_x25519.as_deref().map(hex::encode),
    }))
}

#[derive(Serialize, ToSchema)]
pub struct PrivkeyResponse {
    /// Hex-encoded raw 32-byte X25519 private key — only served to authenticated clients of this local server.
    pub private_key_x25519: String,
}

#[utoipa::path(
    get,
    path = "/identity/me/privkey",
    tag = "identity",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "X25519 private key for this server's identity", body = PrivkeyResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_identity(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
) -> Result<StatusCode, ApiError> {
    let blob_keys = state
        .db
        .delete_identity_cascade(&claims.identity_id)
        .await?;
    for key in &blob_keys {
        state.blobs.delete(key).await.ok();
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_privkey(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
) -> Json<PrivkeyResponse> {
    Json(PrivkeyResponse {
        private_key_x25519: hex::encode(state.identity_privkey),
    })
}
