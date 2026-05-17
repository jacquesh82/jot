use crate::auth::{
    make_claims,
    middleware::{AuthenticatedDevice, OptionalDevice},
    sign_token,
};
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use jot_core::models::{Device, LinkSession, LinkStatus};
use rand::Rng;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct InitLinkResponse {
    pub token: String,
    /// 4-digit PIN code to display to the user
    pub code: String,
    pub expires_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct LinkInfoResponse {
    pub token: String,
    pub code: String,
    /// "pending" | "confirmed" | "expired"
    pub status: String,
    pub expires_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct LinkStatusResponse {
    /// "pending" | "confirmed" | "expired"
    pub status: String,
    /// JWT for the new device, present only when status = "confirmed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ConfirmLinkBody {
    pub token: String,
    pub device_name: Option<String>,
}

#[utoipa::path(
    post,
    path = "/link/init",
    tag = "link",
    responses(
        (status = 201, description = "Link session created", body = InitLinkResponse)
    )
)]
pub async fn init_link(
    State(state): State<AppState>,
    OptionalDevice(caller): OptionalDevice,
) -> Result<(StatusCode, Json<InitLinkResponse>), ApiError> {
    let token = Uuid::new_v4().to_string();
    let code = format!("{:04}", rand::thread_rng().gen_range(0..10000));
    let expires_at = Utc::now() + chrono::Duration::minutes(5);

    let session = if let Some(claims) = caller {
        // Authenticated caller (e.g. WhoamiPage): pre-confirm with new device JWT so the
        // new (unauthenticated) CLI device can pick it up via link_status without needing auth.
        let identity_id = Uuid::parse_str(&claims.identity_id)
            .map_err(|_| ApiError::Internal("invalid identity_id in token".into()))?;
        let new_device_id = Uuid::new_v4();
        let device = Device {
            id: new_device_id,
            identity_id,
            pub_key_x25519: String::new(),
            pub_key_ed25519: String::new(),
            name: "CLI".to_string(),
            last_seen: Utc::now(),
        };
        state.db.insert_device(&device).await?;
        let new_claims = make_claims(&new_device_id.to_string(), &claims.identity_id);
        let jwt = sign_token(&new_claims, &state.signing_key_pem)?;
        LinkSession {
            token: token.clone(),
            code: code.clone(),
            status: LinkStatus::Confirmed,
            pub_key_initiator: String::new(),
            encrypted_symkey: Some(jwt.into_bytes()),
            expires_at,
        }
    } else {
        LinkSession {
            token: token.clone(),
            code: code.clone(),
            status: LinkStatus::Pending,
            pub_key_initiator: String::new(),
            encrypted_symkey: None,
            expires_at,
        }
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

#[utoipa::path(
    get,
    path = "/link/{token}",
    tag = "link",
    params(("token" = String, Path, description = "Link session token")),
    responses(
        (status = 200, description = "Link session details", body = LinkInfoResponse),
        (status = 404, description = "Session not found")
    )
)]
pub async fn get_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<LinkInfoResponse>, ApiError> {
    let link = state.db.get_link(&token).await?.ok_or(ApiError::NotFound)?;
    let status = match link.status {
        LinkStatus::Pending => "pending",
        LinkStatus::Confirmed => "confirmed",
        LinkStatus::Expired => "expired",
    };
    Ok(Json(LinkInfoResponse {
        token: link.token,
        code: link.code,
        status: status.to_string(),
        expires_at: link.expires_at.to_rfc3339(),
    }))
}

#[utoipa::path(
    post,
    path = "/link/confirm",
    tag = "link",
    security(("bearer_auth" = [])),
    request_body = ConfirmLinkBody,
    responses(
        (status = 200, description = "Link confirmed, new device created"),
        (status = 400, description = "Session not pending"),
        (status = 404, description = "Session not found")
    )
)]
pub async fn confirm_link(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Json(body): Json<ConfirmLinkBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let link = state
        .db
        .get_link(&body.token)
        .await?
        .ok_or(ApiError::NotFound)?;

    if link.status != LinkStatus::Pending {
        return Err(ApiError::BadRequest("link session is not pending".into()));
    }

    let identity_id = Uuid::parse_str(&claims.identity_id)
        .map_err(|_| ApiError::Internal("invalid identity_id in token".into()))?;

    let new_device_id = Uuid::new_v4();
    let device_name = body
        .device_name
        .unwrap_or_else(|| "Web Browser".to_string());

    let device = Device {
        id: new_device_id,
        identity_id,
        pub_key_x25519: String::new(),
        pub_key_ed25519: String::new(),
        name: device_name,
        last_seen: Utc::now(),
    };
    state.db.insert_device(&device).await?;

    let new_claims = make_claims(&new_device_id.to_string(), &claims.identity_id);
    let jwt = sign_token(&new_claims, &state.signing_key_pem)?;

    state
        .db
        .confirm_link(&body.token, jwt.as_bytes().to_vec())
        .await?;

    Ok(Json(serde_json::json!({ "status": "confirmed" })))
}

#[utoipa::path(
    get,
    path = "/link/status/{token}",
    tag = "link",
    params(("token" = String, Path, description = "Link session token")),
    responses(
        (status = 200, description = "Link session status", body = LinkStatusResponse),
        (status = 404, description = "Session not found")
    )
)]
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

    let jwt = if link.status == LinkStatus::Confirmed {
        link.encrypted_symkey
            .as_deref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .map(str::to_owned)
    } else {
        None
    };

    Ok(Json(LinkStatusResponse { status, jwt }))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::{test_app_with_state, test_token};
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn link_flow_init_confirm_status() {
        let (app, state) = test_app_with_state().await;

        let (jwt, ..) = test_token(&state).await;

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

        let confirm_body = serde_json::json!({ "token": token });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/link/confirm")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {jwt}"))
                    .body(Body::from(serde_json::to_vec(&confirm_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

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
        assert!(
            json["jwt"].as_str().is_some(),
            "link_status should return a jwt when confirmed"
        );
    }
}
