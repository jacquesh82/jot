use crate::auth::middleware::AuthenticatedDevice;
use crate::state::{AppState, WsEvent};
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use base64::Engine;
use chrono::Utc;
use jot_core::models::{Block, BlockType};
use serde::{Deserialize, Serialize};
use storage::db::shares::permission_allows;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct BlockDto {
    pub id: String,
    pub note_id: String,
    pub parent_block_id: Option<String>,
    pub position: f64,
    pub block_type: String,
    /// Base64-encoded content bytes.
    pub content: String,
    /// Base64-encoded metadata bytes, if any.
    pub metadata: Option<String>,
    pub collapsed: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn to_dto(b: &Block) -> BlockDto {
    let b64 = base64::engine::general_purpose::STANDARD;
    BlockDto {
        id: b.id.to_string(),
        note_id: b.note_id.to_string(),
        parent_block_id: b.parent_block_id.map(|p| p.to_string()),
        position: b.position,
        block_type: b.block_type.as_str().to_string(),
        content: b64.encode(&b.content),
        metadata: b.metadata.as_ref().map(|m| b64.encode(m)),
        collapsed: b.collapsed,
        created_at: b.created_at.to_rfc3339(),
        updated_at: b.updated_at.to_rfc3339(),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBlockBody {
    pub parent_id: Option<Uuid>,
    pub position: Option<f64>,
    pub block_type: String,
    pub content_b64: String,
    pub metadata_b64: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct PatchBlockBody {
    pub block_type: Option<String>,
    pub content_b64: Option<String>,
    pub metadata_b64: Option<String>,
    pub collapsed: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct MoveBlockBody {
    pub new_parent_id: Option<Uuid>,
    pub new_position: f64,
}

fn decode_b64(s: &str) -> Result<Vec<u8>, ApiError> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|_| ApiError::BadRequest("invalid base64".into()))
}

async fn require_write(state: &AppState, note_id: Uuid, identity: &str) -> Result<(), ApiError> {
    let perm = state
        .db
        .note_permission_for(&note_id.to_string(), identity)
        .await?;
    if !permission_allows(&perm, "write") {
        return Err(ApiError::Forbidden("no write permission on note".into()));
    }
    Ok(())
}

pub async fn list_blocks(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(note_id): Path<Uuid>,
) -> Result<Json<Vec<BlockDto>>, ApiError> {
    let perm = state
        .db
        .note_permission_for(&note_id.to_string(), &auth.0.identity_id)
        .await?;
    if !permission_allows(&perm, "read") {
        return Err(ApiError::Forbidden("no read permission on note".into()));
    }
    let blocks = state.db.list_blocks_for_note(note_id).await?;
    Ok(Json(blocks.iter().map(to_dto).collect()))
}

pub async fn create_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(note_id): Path<Uuid>,
    Json(body): Json<CreateBlockBody>,
) -> Result<(StatusCode, Json<BlockDto>), ApiError> {
    require_write(&state, note_id, &auth.0.identity_id).await?;
    let now = Utc::now();
    let position = match body.position {
        Some(p) => p,
        None => state.db.max_position(note_id, body.parent_id).await? + 1.0,
    };
    let b = Block {
        id: Uuid::new_v4(),
        note_id,
        parent_block_id: body.parent_id,
        position,
        block_type: BlockType::from_str(&body.block_type),
        content: decode_b64(&body.content_b64)?,
        metadata: body.metadata_b64.as_deref().map(decode_b64).transpose()?,
        collapsed: false,
        created_at: now,
        updated_at: now,
    };
    state.db.insert_block(&b).await?;
    let _ = state.ws_tx.send(WsEvent::BlockCreated {
        note_id: note_id.to_string(),
        block_id: b.id.to_string(),
    });
    Ok((StatusCode::CREATED, Json(to_dto(&b))))
}

pub async fn get_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<BlockDto>, ApiError> {
    let b = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    let perm = state
        .db
        .note_permission_for(&b.note_id.to_string(), &auth.0.identity_id)
        .await?;
    if !permission_allows(&perm, "read") {
        return Err(ApiError::Forbidden(
            "no read permission on owning note".into(),
        ));
    }
    Ok(Json(to_dto(&b)))
}

pub async fn patch_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchBlockBody>,
) -> Result<Json<BlockDto>, ApiError> {
    let existing = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    require_write(&state, existing.note_id, &auth.0.identity_id).await?;
    let new_type = body
        .block_type
        .as_deref()
        .map(BlockType::from_str)
        .unwrap_or(existing.block_type);
    let new_content = match body.content_b64 {
        Some(s) => decode_b64(&s)?,
        None => existing.content.clone(),
    };
    let new_meta = match body.metadata_b64 {
        Some(s) => Some(decode_b64(&s)?),
        None => existing.metadata.clone(),
    };
    state
        .db
        .update_block_content(id, &new_content, new_meta.as_deref(), new_type)
        .await?;
    if let Some(c) = body.collapsed {
        state.db.set_block_collapsed(id, c).await?;
    }
    let refreshed = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    let _ = state.ws_tx.send(WsEvent::BlockUpdated {
        note_id: existing.note_id.to_string(),
        block_id: id.to_string(),
    });
    Ok(Json(to_dto(&refreshed)))
}

pub async fn delete_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let existing = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    let perm = state
        .db
        .note_permission_for(&existing.note_id.to_string(), &auth.0.identity_id)
        .await?;
    if !permission_allows(&perm, "delete") {
        return Err(ApiError::Forbidden("no delete permission on note".into()));
    }
    state.db.delete_block(id).await?;
    let _ = state.ws_tx.send(WsEvent::BlockDeleted {
        note_id: existing.note_id.to_string(),
        block_id: id.to_string(),
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveBlockBody>,
) -> Result<StatusCode, ApiError> {
    let existing = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    require_write(&state, existing.note_id, &auth.0.identity_id).await?;
    state
        .db
        .move_block(id, body.new_parent_id, body.new_position)
        .await?;
    let _ = state.ws_tx.send(WsEvent::BlockMoved {
        note_id: existing.note_id.to_string(),
        block_id: id.to_string(),
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn indent_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let b = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    require_write(&state, b.note_id, &auth.0.identity_id).await?;
    let prev = state
        .db
        .previous_sibling(b.note_id, b.parent_block_id, b.position)
        .await?
        .ok_or_else(|| ApiError::BadRequest("no preceding sibling to indent under".into()))?;
    let max_under_prev = state.db.max_position(b.note_id, Some(prev.id)).await?;
    state
        .db
        .move_block(id, Some(prev.id), max_under_prev + 1.0)
        .await?;
    let _ = state.ws_tx.send(WsEvent::BlockMoved {
        note_id: b.note_id.to_string(),
        block_id: id.to_string(),
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn outdent_block(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let b = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    require_write(&state, b.note_id, &auth.0.identity_id).await?;
    let parent_id = b
        .parent_block_id
        .ok_or_else(|| ApiError::BadRequest("block is already at root".into()))?;
    let parent = state.db.get_block(parent_id).await?.ok_or(ApiError::NotFound)?;
    let next_pos = parent.position + 0.5;
    state
        .db
        .move_block(id, parent.parent_block_id, next_pos)
        .await?;
    let _ = state.ws_tx.send(WsEvent::BlockMoved {
        note_id: b.note_id.to_string(),
        block_id: id.to_string(),
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn block_backlinks(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<storage::db::block_links::BackLinkRow>>, ApiError> {
    let mut visible = Vec::new();
    for l in state.db.backlinks_to_block(id).await? {
        if let Some(src) = state.db.get_block(l.source_block_id).await? {
            let perm = state
                .db
                .note_permission_for(src.note_id.to_string().as_str(), &auth.0.identity_id)
                .await?;
            if permission_allows(&perm, "read") {
                visible.push(storage::db::block_links::BackLinkRow {
                    source_block_id: l.source_block_id.to_string(),
                    source_note_id: src.note_id.to_string(),
                    link_kind: l.link_kind.as_str().to_string(),
                });
            }
        }
    }
    Ok(Json(visible))
}

pub async fn note_backlinks(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<storage::db::block_links::BackLinkRow>>, ApiError> {
    let mut visible = Vec::new();
    for l in state.db.backlinks_to_note(id).await? {
        if let Some(src) = state.db.get_block(l.source_block_id).await? {
            let perm = state
                .db
                .note_permission_for(src.note_id.to_string().as_str(), &auth.0.identity_id)
                .await?;
            if permission_allows(&perm, "read") {
                visible.push(storage::db::block_links::BackLinkRow {
                    source_block_id: l.source_block_id.to_string(),
                    source_note_id: src.note_id.to_string(),
                    link_kind: l.link_kind.as_str().to_string(),
                });
            }
        }
    }
    Ok(Json(visible))
}

#[derive(Deserialize, ToSchema)]
pub struct PutLinksBody {
    pub links: Vec<LinkInput>,
}

#[derive(Deserialize, ToSchema)]
pub struct LinkInput {
    pub target_kind: String,
    pub target_id: String,
    pub link_kind: String,
}

pub async fn put_block_links(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(id): Path<Uuid>,
    Json(body): Json<PutLinksBody>,
) -> Result<StatusCode, ApiError> {
    let b = state.db.get_block(id).await?.ok_or(ApiError::NotFound)?;
    require_write(&state, b.note_id, &auth.0.identity_id).await?;
    let now = Utc::now();
    let links: Vec<_> = body
        .links
        .into_iter()
        .map(|l| jot_core::models::BlockLink {
            id: Uuid::new_v4(),
            source_block_id: id,
            target_kind: jot_core::models::TargetKind::from_str(&l.target_kind)
                .unwrap_or(jot_core::models::TargetKind::Note),
            target_id: l.target_id,
            link_kind: jot_core::models::LinkKind::from_str(&l.link_kind)
                .unwrap_or(jot_core::models::LinkKind::PageRef),
            created_at: now,
        })
        .collect();
    state.db.replace_links_for_block(id, &links).await?;
    Ok(StatusCode::NO_CONTENT)
}
