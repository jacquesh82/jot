use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct TagDto {
    pub name: String,
    pub color: Option<String>,
}

pub async fn list_tags(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<Vec<TagDto>>, ApiError> {
    let id = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity id".into()))?;
    let tags = state.db.list_tags(id).await?;
    Ok(Json(
        tags.into_iter()
            .map(|t| TagDto {
                name: t.name,
                color: t.color,
            })
            .collect(),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct PutTagBody {
    pub color: Option<String>,
}

pub async fn put_tag(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(name): Path<String>,
    Json(body): Json<PutTagBody>,
) -> Result<axum::http::StatusCode, ApiError> {
    let id = Uuid::parse_str(&auth.0.identity_id)
        .map_err(|_| ApiError::BadRequest("bad identity id".into()))?;
    state
        .db
        .upsert_tag(&name, id, body.color.as_deref())
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn blocks_with_tag(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(name): Path<String>,
) -> Result<Json<Vec<String>>, ApiError> {
    let ids = state.db.blocks_with_tag(&name).await?;
    Ok(Json(ids.into_iter().map(|i| i.to_string()).collect()))
}
