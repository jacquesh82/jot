use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use jot_core::models::{LinkSession, LinkStatus};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct InitLinkResponse {
    pub token: String,
    pub code: String,
    pub expires_at: String,
}

#[derive(Serialize)]
pub struct LinkStatusResponse {
    pub status: String,
}

#[derive(Deserialize)]
pub struct ConfirmLinkBody {
    pub token: String,
    pub encrypted_symkey: String, // base64
}

pub async fn init_link(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<InitLinkResponse>), ApiError> {
    let token = Uuid::new_v4().to_string();
    let code = format!("{:04}", rand::thread_rng().gen_range(0..10000));
    let expires_at = Utc::now() + chrono::Duration::minutes(5);

    let session = LinkSession {
        token: token.clone(),
        code: code.clone(),
        status: LinkStatus::Pending,
        pub_key_initiator: String::new(),
        encrypted_symkey: None,
        expires_at,
    };
    state.db.insert_link(&session).await?;

    Ok((
        StatusCode::CREATED,
        Json(InitLinkResponse {
            token,
            code,
            expires_at: expires_at.to_rfc3339(),
        }),
    ))
}

pub async fn get_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let link = state.db.get_link(&token).await?.ok_or(ApiError::NotFound)?;
    let status = match link.status {
        LinkStatus::Pending => "pending",
        LinkStatus::Confirmed => "confirmed",
        LinkStatus::Expired => "expired",
    };
    Ok(Json(serde_json::json!({
        "token": link.token,
        "code": link.code,
        "status": status,
        "expires_at": link.expires_at.to_rfc3339(),
    })))
}

pub async fn confirm_link(
    State(state): State<AppState>,
    Json(body): Json<ConfirmLinkBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let symkey = STANDARD
        .decode(&body.encrypted_symkey)
        .map_err(|_| ApiError::BadRequest("invalid base64 encrypted_symkey".into()))?;
    state.db.confirm_link(&body.token, symkey).await?;
    Ok(Json(serde_json::json!({ "status": "confirmed" })))
}

pub async fn link_status(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<LinkStatusResponse>, ApiError> {
    let link = state.db.get_link(&token).await?.ok_or(ApiError::NotFound)?;
    let status = match link.status {
        LinkStatus::Pending => "pending",
        LinkStatus::Confirmed => "confirmed",
        LinkStatus::Expired => "expired",
    }
    .to_string();
    Ok(Json(LinkStatusResponse { status }))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_app;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use base64::{engine::general_purpose::STANDARD, Engine};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn link_flow_init_confirm_status() {
        let app = test_app().await;

        // Init
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/link/init")
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let token = json["token"].as_str().unwrap().to_string();

        // Confirm
        let symkey_b64 = STANDARD.encode(b"fake-encrypted-symkey");
        let confirm_body =
            serde_json::json!({ "token": token, "encrypted_symkey": symkey_b64 });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/link/confirm")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&confirm_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Status
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/link/status/{}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "confirmed");
    }
}
