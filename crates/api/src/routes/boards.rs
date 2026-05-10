use crate::auth::middleware::AuthenticatedDevice;
use crate::state::{AppState, WsEvent};
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use jot_core::models::Board;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateBoardBody {
    pub name: String,
    pub position: Option<i32>,
}

#[derive(Deserialize)]
pub struct PatchBoardBody {
    pub name: Option<String>,
    pub position: Option<i32>,
}

#[derive(Deserialize)]
pub struct ReorderItem {
    pub note_id: Uuid,
    pub position: i32,
}

#[derive(Serialize)]
pub struct BoardResponse {
    pub id: String,
    pub identity_id: String,
    pub name: String,
    pub position: i32,
    pub created_at: String,
}

fn to_response(b: &Board) -> BoardResponse {
    BoardResponse {
        id: b.id.to_string(),
        identity_id: b.identity_id.to_string(),
        name: b.name.clone(),
        position: b.position,
        created_at: b.created_at.to_rfc3339(),
    }
}

pub async fn list_boards(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<Vec<BoardResponse>>, ApiError> {
    let identity_id = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::Internal("invalid identity_id in token".into()))?;
    let boards = state.db.list_boards(identity_id).await?;
    Ok(Json(boards.iter().map(to_response).collect()))
}

pub async fn create_board(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Json(body): Json<CreateBoardBody>,
) -> Result<(StatusCode, Json<BoardResponse>), ApiError> {
    let identity_id = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::Internal("invalid identity_id in token".into()))?;
    let board = Board {
        id: Uuid::new_v4(),
        identity_id,
        name: body.name,
        position: body.position.unwrap_or(0),
        created_at: Utc::now(),
    };
    state.db.insert_board(&board).await?;
    let _ = state.ws_tx.send(WsEvent::BoardUpdated {
        id: board.id.to_string(),
    });
    Ok((StatusCode::CREATED, Json(to_response(&board))))
}

pub async fn patch_board(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchBoardBody>,
) -> Result<StatusCode, ApiError> {
    if let Some(name) = body.name {
        state.db.update_board_name(id, &name).await?;
    }
    if let Some(pos) = body.position {
        state.db.update_board_position(id, pos).await?;
    }
    let _ = state
        .ws_tx
        .send(WsEvent::BoardUpdated { id: id.to_string() });
    Ok(StatusCode::OK)
}

pub async fn delete_board(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.db.delete_board(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reorder_board(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(_id): Path<Uuid>,
    Json(items): Json<Vec<ReorderItem>>,
) -> Result<StatusCode, ApiError> {
    for item in &items {
        state
            .db
            .update_note_position(item.note_id, item.position)
            .await?;
    }
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::{test_app_with_state, test_token};
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn create_and_list_boards() {
        let (app, state) = test_app_with_state().await;
        let (token, ..) = test_token(&state).await;

        let create_body = json!({ "name": "My Board", "position": 0 });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/boards")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(serde_json::to_vec(&create_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/boards")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let boards: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(boards.as_array().unwrap().len(), 1);
    }
}
