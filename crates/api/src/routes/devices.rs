use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use jot_core::models::Device;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub name: String,
    pub pub_key_ed25519: String,
    pub last_seen: String,
}

fn to_response(d: &Device) -> DeviceResponse {
    DeviceResponse {
        id: d.id.to_string(),
        name: d.name.clone(),
        pub_key_ed25519: d.pub_key_ed25519.clone(),
        last_seen: d.last_seen.to_rfc3339(),
    }
}

#[derive(Deserialize)]
pub struct RenameBody {
    pub name: String,
}

pub async fn list_devices(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<Vec<DeviceResponse>>, ApiError> {
    let identity_id = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::Internal("invalid identity_id".into()))?;
    let devices = state.db.list_devices(identity_id).await?;
    Ok(Json(devices.iter().map(to_response).collect()))
}

pub async fn delete_device(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.db.delete_device(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn rename_device(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<RenameBody>,
) -> Result<StatusCode, ApiError> {
    state.db.rename_device(id, &body.name).await?;
    Ok(StatusCode::OK)
}
