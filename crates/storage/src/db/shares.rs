use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;

pub struct NoteShare {
    pub note_id: String,
    pub owner_identity_id: String,
    pub shared_with_id: String,
    pub encrypted_dek: Option<Vec<u8>>,
    pub permission: String,
    pub created_at: String,
}

pub struct SharedNoteRow {
    pub note_id: String,
    pub note_type: String,
    pub board_id: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
    pub encrypted_dek: Option<Vec<u8>>,
    pub snippet: Option<String>,
}

/// Returns true when `actual` permission satisfies `required`.
/// Hierarchy: delete ≥ write ≥ read.
pub fn permission_allows(actual: &str, required: &str) -> bool {
    match required {
        "read" => true,
        "write" => matches!(actual, "write" | "delete"),
        "delete" => actual == "delete",
        _ => false,
    }
}

impl Db {
    pub async fn share_note(
        &self,
        note_id: &str,
        owner_identity_id: &str,
        shared_with_id: &str,
        encrypted_dek: Option<Vec<u8>>,
        permission: &str,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO note_shares \
             (note_id, owner_identity_id, shared_with_id, encrypted_dek, permission, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(note_id)
        .bind(owner_identity_id)
        .bind(shared_with_id)
        .bind(encrypted_dek)
        .bind(permission)
        .bind(&now)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_note_shares(&self, note_id: &str) -> Result<Vec<NoteShare>, StorageError> {
        let rows = sqlx::query(
            "SELECT note_id, owner_identity_id, shared_with_id, encrypted_dek, permission, created_at \
             FROM note_shares WHERE note_id = ?",
        )
        .bind(note_id)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| NoteShare {
                note_id: r.get("note_id"),
                owner_identity_id: r.get("owner_identity_id"),
                shared_with_id: r.get("shared_with_id"),
                encrypted_dek: r.get("encrypted_dek"),
                permission: r.get("permission"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn get_shared_with_me(
        &self,
        identity_id: &str,
    ) -> Result<Vec<SharedNoteRow>, StorageError> {
        let rows = sqlx::query(
            "SELECT ns.note_id, n.note_type, n.board_id, ns.owner_identity_id, \
                    i.friendly_name AS owner_friendly_name, ns.encrypted_dek, n.content AS snippet \
             FROM note_shares ns \
             JOIN notes n ON n.id = ns.note_id \
             LEFT JOIN identities i ON i.id = ns.owner_identity_id \
             WHERE ns.shared_with_id = ?",
        )
        .bind(identity_id)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let snippet_bytes: Option<Vec<u8>> = r.get("snippet");
                SharedNoteRow {
                    note_id: r.get("note_id"),
                    note_type: r.get("note_type"),
                    board_id: r.get("board_id"),
                    owner_identity_id: r.get("owner_identity_id"),
                    owner_friendly_name: r.get("owner_friendly_name"),
                    encrypted_dek: r.get("encrypted_dek"),
                    snippet: snippet_bytes
                        .and_then(|b| String::from_utf8(b).ok())
                        .filter(|s| !s.is_empty()),
                }
            })
            .collect())
    }

    pub async fn list_shared_note_ids_for_board(
        &self,
        board_id: &str,
        owner_identity_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        let rows = sqlx::query(
            "SELECT DISTINCT ns.note_id FROM note_shares ns \
             JOIN notes n ON n.id = ns.note_id \
             WHERE n.board_id = ? AND ns.owner_identity_id = ?",
        )
        .bind(board_id)
        .bind(owner_identity_id)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| r.get::<String, _>("note_id"))
            .collect())
    }

    pub async fn delete_share(
        &self,
        note_id: &str,
        shared_with_id: &str,
    ) -> Result<bool, StorageError> {
        let r = sqlx::query("DELETE FROM note_shares WHERE note_id = ? AND shared_with_id = ?")
            .bind(note_id)
            .bind(shared_with_id)
            .execute(&self.0)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Return the set of note IDs that have a DEK for the given identity (i.e. are encrypted).
    pub async fn list_encrypted_note_ids_for_board(
        &self,
        board_id: &str,
        identity_id: &str,
    ) -> Result<std::collections::HashSet<String>, StorageError> {
        let rows = sqlx::query(
            "SELECT ns.note_id FROM note_shares ns \
             JOIN notes n ON n.id = ns.note_id \
             WHERE n.board_id = ? AND ns.shared_with_id = ?",
        )
        .bind(board_id)
        .bind(identity_id)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| r.get::<String, _>("note_id"))
            .collect())
    }

    /// Return the encrypted DEK for a given note/identity pair.
    pub async fn get_note_dek(
        &self,
        note_id: &str,
        identity_id: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let row = sqlx::query(
            "SELECT encrypted_dek FROM note_shares \
             WHERE note_id = ? AND shared_with_id = ?",
        )
        .bind(note_id)
        .bind(identity_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.and_then(|r| r.get("encrypted_dek")))
    }

    /// Resolve the effective permission a given identity has on a note.
    ///
    /// Resolution order:
    /// 1. Owner of the note's board -> "delete"
    /// 2. Individual note share -> share's permission ("read"|"write"|"delete")
    /// 3. Board share (member of the board the note belongs to) -> "read"
    /// 4. Otherwise -> "none"
    pub async fn note_permission_for(
        &self,
        note_id: &str,
        identity: &str,
    ) -> Result<String, StorageError> {
        let row = sqlx::query("SELECT board_id FROM notes WHERE id = ?")
            .bind(note_id)
            .fetch_optional(&self.0)
            .await?;
        let board_id: Option<String> = row.map(|r| r.get::<String, _>("board_id"));
        if let Some(ref bid) = board_id {
            if self.owns_board(bid, identity).await? {
                return Ok("delete".into());
            }
        }
        if let Some(p) = self.get_note_share_permission(note_id, identity).await? {
            return Ok(p);
        }
        if let Some(bid) = board_id {
            if self.can_access_board(&bid, identity).await? {
                return Ok("read".into());
            }
        }
        Ok("none".into())
    }

    /// Return the permission granted to identity_id on a note, or None if no share exists.
    pub async fn get_note_share_permission(
        &self,
        note_id: &str,
        identity_id: &str,
    ) -> Result<Option<String>, StorageError> {
        let row = sqlx::query(
            "SELECT permission FROM note_shares WHERE note_id = ? AND shared_with_id = ?",
        )
        .bind(note_id)
        .bind(identity_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| r.get("permission")))
    }
}
