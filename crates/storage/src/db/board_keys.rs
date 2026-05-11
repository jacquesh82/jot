use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;

impl Db {
    /// Store (or replace) an encrypted BEK for a board member.
    pub async fn put_board_key(
        &self,
        board_id: &str,
        identity_id: &str,
        encrypted_bek: Vec<u8>,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO board_keys \
             (board_id, identity_id, encrypted_bek, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(board_id)
        .bind(identity_id)
        .bind(encrypted_bek)
        .bind(&now)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    /// Retrieve the encrypted BEK for a board member.
    pub async fn get_board_key(
        &self,
        board_id: &str,
        identity_id: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let row = sqlx::query(
            "SELECT encrypted_bek FROM board_keys WHERE board_id = ? AND identity_id = ?",
        )
        .bind(board_id)
        .bind(identity_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| r.get("encrypted_bek")))
    }

    /// Return true if an encrypted BEK exists for this identity in this board.
    pub async fn has_board_key(
        &self,
        board_id: &str,
        identity_id: &str,
    ) -> Result<bool, StorageError> {
        let row = sqlx::query(
            "SELECT 1 FROM board_keys WHERE board_id = ? AND identity_id = ?",
        )
        .bind(board_id)
        .bind(identity_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.is_some())
    }

    /// Remove a member's BEK (revoke board access).
    pub async fn delete_board_key(
        &self,
        board_id: &str,
        identity_id: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM board_keys WHERE board_id = ? AND identity_id = ?")
            .bind(board_id)
            .bind(identity_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
