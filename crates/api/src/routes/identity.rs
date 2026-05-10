use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct IdentityResponse {
    pub id: String,
    pub friendly_name: String,
}

#[derive(Deserialize)]
pub struct UpdateNameBody {
    pub friendly_name: String,
}

/// GET /identity/me — returns the current identity's friendly name
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
        id: identity.id,
        friendly_name: identity.friendly_name,
    }))
}

/// PATCH /identity/me — update friendly name (must be unique)
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
    // Check uniqueness (another identity already using that name)
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
        // Identity row doesn't exist yet — create it
        state
            .db
            .insert_identity(&claims.identity_id, &name)
            .await?;
    }
    Ok(Json(IdentityResponse {
        id: claims.identity_id,
        friendly_name: name,
    }))
}

/// GET /identity/contacts — recent identities this user has shared boards with
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
            })
            .collect(),
    ))
}

/// GET /identity/lookup/:name — find an identity by friendly name (for sharing)
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
        friendly_name: identity.friendly_name,
    }))
}
