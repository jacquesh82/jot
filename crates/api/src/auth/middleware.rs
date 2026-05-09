use crate::auth::{verify_token, DeviceClaims};
use crate::ApiError;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};

pub struct AuthenticatedDevice(pub DeviceClaims);

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
        Ok(AuthenticatedDevice(claims))
    }
}
