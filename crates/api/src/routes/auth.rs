use crate::auth::{make_claims, sign_token};
use crate::state::AppState;
use crate::ApiError;
use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use jot_core::models::Device;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RegisterIdentityBody {
    pub uuid: String,
    pub pseudo: String,
    pub avatar: String,
}

#[derive(Deserialize)]
pub struct RegisterDeviceBody {
    pub device_id: String,
    pub identity_id: String,
    pub pub_key_x25519: String,
    pub pub_key_ed25519: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct RegisterDeviceResponse {
    pub token: String,
}

pub async fn register_identity(
    Json(body): Json<RegisterIdentityBody>,
) -> Result<StatusCode, ApiError> {
    let _ = body;
    Ok(StatusCode::CREATED)
}

pub async fn register_device(
    State(state): State<AppState>,
    Json(body): Json<RegisterDeviceBody>,
) -> Result<(StatusCode, Json<RegisterDeviceResponse>), ApiError> {
    let device_id = Uuid::parse_str(&body.device_id)
        .map_err(|_| ApiError::BadRequest("invalid device_id".into()))?;
    let identity_id = Uuid::parse_str(&body.identity_id)
        .map_err(|_| ApiError::BadRequest("invalid identity_id".into()))?;

    let device = Device {
        id: device_id,
        identity_id,
        pub_key_x25519: body.pub_key_x25519,
        pub_key_ed25519: body.pub_key_ed25519,
        name: body.name,
        last_seen: Utc::now(),
    };

    state.db.insert_device(&device).await?;

    let claims = make_claims(&body.device_id, &body.identity_id);
    let token = sign_token(&claims, &state.signing_key_pem)?;

    Ok((StatusCode::CREATED, Json(RegisterDeviceResponse { token })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_app;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_ok() {
        let app = test_app().await;
        let resp = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn register_device_returns_jwt() {
        let app = test_app().await;
        let body = json!({
            "device_id": Uuid::new_v4().to_string(),
            "identity_id": Uuid::new_v4().to_string(),
            "pub_key_x25519": "aabb",
            "pub_key_ed25519": "ccdd",
            "name": "Test Device"
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/auth/device")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["token"].as_str().is_some());
    }
}
