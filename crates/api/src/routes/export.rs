use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{extract::State, Json};
use chrono::Utc;
use jot_core::models::NoteType;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/export",
    tag = "export",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Full data export as JSON (all boards and notes with content)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Identity not found")
    )
)]
pub async fn export_data(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> Result<Json<serde_json::Value>, ApiError> {
    let identity_id = Uuid::parse_str(&auth.0.identity_id).map_err(|_| ApiError::Unauthorized)?;
    let identity = state
        .db
        .get_identity_by_id(&auth.0.identity_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let boards = state.db.list_boards(identity_id).await?;

    let mut board_exports = Vec::new();
    for board in &boards {
        let notes = state.db.list_notes(board.id).await?;
        let mut note_exports = Vec::new();
        for note in &notes {
            let content = state.blobs.get(&note.blob_key).await.unwrap_or_default();
            let content_str = String::from_utf8_lossy(&content).to_string();
            note_exports.push(serde_json::json!({
                "id": note.id.to_string(),
                "type": match note.note_type {
                    NoteType::Text => "text",
                    NoteType::Voice => "voice",
                    NoteType::Image => "image",
                },
                "content": content_str,
                "created_at": note.created_at.to_rfc3339(),
                "updated_at": note.updated_at.to_rfc3339(),
            }));
        }
        board_exports.push(serde_json::json!({
            "id": board.id.to_string(),
            "name": board.name,
            "notes": note_exports,
        }));
    }

    Ok(Json(serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "identity": {
            "id": identity.id,
            "friendly_name": identity.friendly_name,
        },
        "boards": board_exports,
    })))
}
