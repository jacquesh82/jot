use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;

pub struct NoteShare {
    pub note_id: String,
    pub owner_identity_id: String,
    pub shared_with_id: String,
    pub encrypted_dek: Option<Vec<u8>>,
    pub created_at: String,
}

pub struct SharedNoteRow {
    pub note_id: String,
    pub note_type: String,
    pub board_id: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
    pub encrypted_dek: Option<Vec<u8>>,
}

impl Db {
    pub async fn share_note(
        &self,
        note_id: &str,
        owner_identity_id: &str,
        shared_with_id: &str,
        encrypted_dek: Option<Vec<u8>>,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO note_shares \
             (note_id, owner_identity_id, shared_with_id, encrypted_dek, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(note_id)
        .bind(owner_identity_id)
        .bind(shared_with_id)
        .bind(encrypted_dek)
        .bind(&now)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_note_shares(
        &self,
        note_id: &str,
    ) -> Result<Vec<NoteShare>, StorageError> {
        let rows = sqlx::query(
            "SELECT note_id, owner_identity_id, shared_with_id, encrypted_dek, created_at \
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
                    i.friendly_name AS owner_friendly_name, ns.encrypted_dek \
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
            .map(|r| SharedNoteRow {
                note_id: r.get("note_id"),
                note_type: r.get("note_type"),
                board_id: r.get("board_id"),
                owner_identity_id: r.get("owner_identity_id"),
                owner_friendly_name: r.get("owner_friendly_name"),
                encrypted_dek: r.get("encrypted_dek"),
            })
            .collect())
    }

    pub async fn delete_share(
        &self,
        note_id: &str,
        shared_with_id: &str,
    ) -> Result<bool, StorageError> {
        let r =
            sqlx::query("DELETE FROM note_shares WHERE note_id = ? AND shared_with_id = ?")
                .bind(note_id)
                .bind(shared_with_id)
                .execute(&self.0)
                .await?;
        Ok(r.rows_affected() > 0)
    }
}
