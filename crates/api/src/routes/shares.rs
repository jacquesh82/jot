use crate::auth::middleware::AuthenticatedDevice;
use crate::state::AppState;
use crate::ApiError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct PutDekBody {
    /// Hex-encoded encrypted DEK (nonce || ciphertext)
    pub encrypted_dek: String,
}

#[derive(Serialize, ToSchema)]
pub struct DekResponse {
    /// Hex-encoded encrypted DEK (nonce || ciphertext)
    pub encrypted_dek: String,
    /// Hex-encoded X25519 public key of the note owner (needed by recipient to derive wrap key)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_pubkey_x25519: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ShareEntry {
    pub shared_with_id: String,
    pub shared_with_name: Option<String>,
    pub created_at: String,
    /// Permission level: "read" | "write" | "delete"
    pub permission: String,
    /// Hex-encoded X25519 public key of the recipient (needed by owner for DEK rotation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key_x25519: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ShareBody {
    /// Target identity UUID or friendly name
    pub target: String,
    /// Hex-encoded DEK re-encrypted for the recipient with ECDH(owner_priv, recipient_pub)→HKDF
    pub encrypted_dek_for_recipient: Option<String>,
    /// Permission level granted to the recipient: "read" | "write" | "delete" (default: "read")
    pub permission: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SharedNoteEntry {
    pub note_id: String,
    pub note_type: String,
    pub board_id: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
    /// Plaintext snippet stored by the owner (may be absent for old notes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

#[utoipa::path(
    get,
    path = "/notes/shared",
    tag = "shares",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Notes shared with the current identity", body = Vec<SharedNoteEntry>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_shared_with_me(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
) -> Result<Json<Vec<SharedNoteEntry>>, ApiError> {
    let rows = state.db.get_shared_with_me(&claims.identity_id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| SharedNoteEntry {
                note_id: r.note_id,
                note_type: r.note_type,
                board_id: r.board_id,
                owner_identity_id: r.owner_identity_id,
                owner_friendly_name: r.owner_friendly_name,
                snippet: r.snippet,
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/notes/{id}/shares",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Note ID")),
    responses(
        (status = 200, description = "List of share entries for this note", body = Vec<ShareEntry>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found")
    )
)]
pub async fn list_shares(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
) -> Result<Json<Vec<ShareEntry>>, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state
        .db
        .get_note(note_uuid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let board = state
        .db
        .get_board(note.board_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if board.identity_id.to_string() != claims.identity_id {
        return Err(ApiError::Unauthorized);
    }
    let owner_id = board.identity_id.to_string();
    let shares = state.db.get_note_shares(&note_id).await?;
    let mut entries = Vec::new();
    for s in shares {
        // Skip the owner's self-share (used internally to store the owner DEK).
        if s.shared_with_id == owner_id {
            continue;
        }
        let identity = state.db.get_identity_by_id(&s.shared_with_id).await?;
        let (name, pubkey) = identity
            .map(|i| (Some(i.friendly_name), i.public_key_x25519.map(|b| hex::encode(b))))
            .unwrap_or((None, None));
        entries.push(ShareEntry {
            shared_with_id: s.shared_with_id,
            shared_with_name: name,
            created_at: s.created_at,
            permission: s.permission,
            public_key_x25519: pubkey,
        });
    }
    Ok(Json(entries))
}

#[utoipa::path(
    post,
    path = "/notes/{id}/shares",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Note ID")),
    request_body = ShareBody,
    responses(
        (status = 201, description = "Note shared"),
        (status = 400, description = "Invalid target or sharing with yourself"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note or target identity not found")
    )
)]
pub async fn share_note(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
    Json(body): Json<ShareBody>,
) -> Result<StatusCode, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state
        .db
        .get_note(note_uuid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let board = state
        .db
        .get_board(note.board_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if board.identity_id.to_string() != claims.identity_id {
        return Err(ApiError::Unauthorized);
    }
    let target_id = if let Ok(uuid) = Uuid::parse_str(&body.target) {
        uuid.to_string()
    } else {
        state
            .db
            .get_identity_by_name(&body.target)
            .await?
            .ok_or_else(|| ApiError::BadRequest("identity not found".into()))?
            .id
    };
    if target_id == claims.identity_id {
        return Err(ApiError::BadRequest("cannot share with yourself".into()));
    }
    let permission = body.permission.as_deref().unwrap_or("read").to_string();
    if !matches!(permission.as_str(), "read" | "write" | "delete") {
        return Err(ApiError::BadRequest(
            "permission must be 'read', 'write', or 'delete'".into(),
        ));
    }
    let dek_bytes = if let Some(hex_dek) = body.encrypted_dek_for_recipient {
        let b = hex::decode(&hex_dek)
            .map_err(|_| ApiError::BadRequest("encrypted_dek_for_recipient must be hex".into()))?;
        Some(b)
    } else {
        None
    };
    state
        .db
        .share_note(&note_id, &claims.identity_id, &target_id, dek_bytes, &permission)
        .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete,
    path = "/notes/{id}/shares/{identity_id}",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Note ID"),
        ("identity_id" = String, Path, description = "Identity to revoke access from")
    ),
    responses(
        (status = 204, description = "Share revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found")
    )
)]
pub async fn delete_share(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path((note_id, target_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state
        .db
        .get_note(note_uuid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let board = state
        .db
        .get_board(note.board_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let owner_id = board.identity_id.to_string();
    if owner_id != claims.identity_id {
        return Err(ApiError::Unauthorized);
    }
    // Prevent deleting the owner's own DEK entry (self-share).
    if target_id == owner_id {
        return Err(ApiError::BadRequest("cannot revoke the note owner's own access".into()));
    }
    state.db.delete_share(&note_id, &target_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/notes/{id}/dek",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Note ID")),
    request_body = PutDekBody,
    responses(
        (status = 204, description = "DEK stored"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Note not found")
    )
)]
pub async fn put_dek(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
    Json(body): Json<PutDekBody>,
) -> Result<StatusCode, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    state
        .db
        .get_note(note_uuid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let dek_bytes = hex::decode(&body.encrypted_dek)
        .map_err(|_| ApiError::BadRequest("encrypted_dek must be hex-encoded".into()))?;
    state
        .db
        .share_note(
            &note_id,
            &claims.identity_id,
            &claims.identity_id,
            Some(dek_bytes),
            "delete", // owner always has full permission on their own note
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/notes/{id}/dek",
    tag = "shares",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Note ID")),
    responses(
        (status = 200, description = "Encrypted DEK", body = DekResponse),
        (status = 404, description = "DEK not found for this identity"),
        (status = 401, description = "Unauthorized")
    )
)]
/// Owner pushes a rotated DEK for a specific recipient (used after revoke + re-encrypt).
pub async fn put_dek_for_identity(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path((note_id, identity_id)): Path<(String, String)>,
    Json(body): Json<PutDekBody>,
) -> Result<StatusCode, ApiError> {
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state.db.get_note(note_uuid).await?.ok_or(ApiError::NotFound)?;
    let board = state.db.get_board(note.board_id).await?.ok_or(ApiError::NotFound)?;
    if board.identity_id.to_string() != claims.identity_id {
        return Err(ApiError::Forbidden("only the note owner can push DEKs".into()));
    }
    let dek_bytes = hex::decode(&body.encrypted_dek)
        .map_err(|_| ApiError::BadRequest("encrypted_dek must be hex-encoded".into()))?;
    // Preserve the existing permission level during DEK rotation.
    let permission = state
        .db
        .get_note_share_permission(&note_id, &identity_id)
        .await?
        .unwrap_or_else(|| "read".to_string());
    state
        .db
        .share_note(&note_id, &claims.identity_id, &identity_id, Some(dek_bytes), &permission)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_dek(
    State(state): State<AppState>,
    AuthenticatedDevice(claims): AuthenticatedDevice,
    Path(note_id): Path<String>,
) -> Result<Json<DekResponse>, ApiError> {
    let dek = state
        .db
        .get_note_dek(&note_id, &claims.identity_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Include the owner's pubkey so the recipient can derive the correct ECDH wrap key.
    let note_uuid = note_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("invalid note id".into()))?;
    let note = state.db.get_note(note_uuid).await?.ok_or(ApiError::NotFound)?;
    let board = state.db.get_board(note.board_id).await?.ok_or(ApiError::NotFound)?;
    let owner_id = board.identity_id.to_string();
    let owner_pubkey = if owner_id != claims.identity_id {
        state
            .db
            .get_identity_by_id(&owner_id)
            .await?
            .and_then(|i| i.public_key_x25519)
            .map(|b| hex::encode(b))
    } else {
        None
    };

    Ok(Json(DekResponse {
        encrypted_dek: hex::encode(dek),
        owner_pubkey_x25519: owner_pubkey,
    }))
}
