use crate::auth::verify_token;
use crate::state::AppState;
use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if verify_token(&query.token, &state.verifying_key_pem).is_err() {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }

    let mut rx = state.ws_tx.subscribe();

    ws.on_upgrade(move |mut socket| async move {
        while let Ok(event) = rx.recv().await {
            let msg = serde_json::to_string(&event).unwrap_or_default();
            if socket
                .send(axum::extract::ws::Message::Text(msg))
                .await
                .is_err()
            {
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_app_with_state;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn ws_without_token_param_not_upgraded() {
        let app = test_app_with_state().await.0;
        let resp = app
            .oneshot(Request::builder().uri("/ws").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
    }

    #[tokio::test]
    async fn ws_with_invalid_token_not_upgraded() {
        let app = test_app_with_state().await.0;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ws?token=invalid.jwt.token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
    }
}
