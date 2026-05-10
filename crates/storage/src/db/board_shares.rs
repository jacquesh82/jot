use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;

pub struct BoardShareEntry {
    pub board_id: String,
    pub shared_with_id: String,
    pub shared_with_name: Option<String>,
    pub created_at: String,
}

pub struct SharedBoardRow {
    pub board_id: String,
    pub board_name: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
}

impl Db {
    pub async fn share_board(
        &self,
        board_id: &str,
        owner_identity_id: &str,
        shared_with_id: &str,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR IGNORE INTO board_shares \
             (board_id, owner_identity_id, shared_with_id, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(board_id)
        .bind(owner_identity_id)
        .bind(shared_with_id)
        .bind(&now)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_board_shares(
        &self,
        board_id: &str,
    ) -> Result<Vec<BoardShareEntry>, StorageError> {
        let rows = sqlx::query(
            "SELECT bs.board_id, bs.shared_with_id, i.friendly_name AS shared_with_name, bs.created_at \
             FROM board_shares bs \
             LEFT JOIN identities i ON i.id = bs.shared_with_id \
             WHERE bs.board_id = ?",
        )
        .bind(board_id)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| BoardShareEntry {
                board_id: r.get("board_id"),
                shared_with_id: r.get("shared_with_id"),
                shared_with_name: r.get("shared_with_name"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn get_boards_shared_with_me(
        &self,
        identity_id: &str,
    ) -> Result<Vec<SharedBoardRow>, StorageError> {
        let rows = sqlx::query(
            "SELECT bs.board_id, b.name AS board_name, bs.owner_identity_id, \
                    i.friendly_name AS owner_friendly_name \
             FROM board_shares bs \
             JOIN boards b ON b.id = bs.board_id \
             LEFT JOIN identities i ON i.id = bs.owner_identity_id \
             WHERE bs.shared_with_id = ?",
        )
        .bind(identity_id)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| SharedBoardRow {
                board_id: r.get("board_id"),
                board_name: r.get("board_name"),
                owner_identity_id: r.get("owner_identity_id"),
                owner_friendly_name: r.get("owner_friendly_name"),
            })
            .collect())
    }

    pub async fn delete_board_share(
        &self,
        board_id: &str,
        shared_with_id: &str,
    ) -> Result<bool, StorageError> {
        let r = sqlx::query("DELETE FROM board_shares WHERE board_id = ? AND shared_with_id = ?")
            .bind(board_id)
            .bind(shared_with_id)
            .execute(&self.0)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Returns true if identity_id owns or has been shared the board.
    pub async fn can_access_board(
        &self,
        board_id: &str,
        identity_id: &str,
    ) -> Result<bool, StorageError> {
        let row = sqlx::query(
            "SELECT 1 FROM boards WHERE id = ? AND identity_id = ? \
             UNION ALL \
             SELECT 1 FROM board_shares WHERE board_id = ? AND shared_with_id = ? \
             LIMIT 1",
        )
        .bind(board_id)
        .bind(identity_id)
        .bind(board_id)
        .bind(identity_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.is_some())
    }

    /// Returns true if identity_id owns the board.
    pub async fn owns_board(
        &self,
        board_id: &str,
        identity_id: &str,
    ) -> Result<bool, StorageError> {
        let row = sqlx::query("SELECT 1 FROM boards WHERE id = ? AND identity_id = ?")
            .bind(board_id)
            .bind(identity_id)
            .fetch_optional(&self.0)
            .await?;
        Ok(row.is_some())
    }
}
