use crate::auth::{verify_token, DeviceClaims};
use crate::state::AppState;
use crate::ApiError;
use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

pub struct AuthenticatedDevice(pub DeviceClaims);
pub struct OptionalDevice(pub Option<DeviceClaims>);

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedDevice {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;

        let claims = verify_token(token, &state.verifying_key_pem)?;

        // Reject tokens belonging to deleted devices.
        let device_id = claims.sub.parse().map_err(|_| ApiError::Unauthorized)?;
        state
            .db
            .get_device(device_id)
            .await
            .map_err(|_| ApiError::Unauthorized)?
            .ok_or(ApiError::Unauthorized)?;

        Ok(AuthenticatedDevice(claims))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for OptionalDevice {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthenticatedDevice::from_request_parts(parts, state).await {
            Ok(AuthenticatedDevice(claims)) => Ok(OptionalDevice(Some(claims))),
            Err(_) => Ok(OptionalDevice(None)),
        }
    }
}
