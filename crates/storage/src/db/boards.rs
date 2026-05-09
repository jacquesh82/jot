use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::Board;
use sqlx::Row;
use uuid::Uuid;

fn board_from_row(row: &sqlx::sqlite::SqliteRow) -> Board {
    let id_str: String = row.get("id");
    let identity_str: String = row.get("identity_id");
    let created_str: String = row.get("created_at");
    Board {
        id: Uuid::parse_str(&id_str).unwrap(),
        identity_id: Uuid::parse_str(&identity_str).unwrap(),
        name: row.get("name"),
        position: row.get("position"),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&Utc),
    }
}

impl Db {
    pub async fn insert_board(&self, board: &Board) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(board.id.to_string())
        .bind(board.identity_id.to_string())
        .bind(&board.name)
        .bind(board.position)
        .bind(board.created_at.to_rfc3339())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_board(&self, id: Uuid) -> Result<Option<Board>, StorageError> {
        let row = sqlx::query(
            "SELECT id, identity_id, name, position, created_at FROM boards WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| board_from_row(&r)))
    }

    pub async fn list_boards(&self, identity_id: Uuid) -> Result<Vec<Board>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, identity_id, name, position, created_at FROM boards WHERE identity_id = ? ORDER BY position ASC",
        )
        .bind(identity_id.to_string())
        .fetch_all(&self.0)
        .await?;
        Ok(rows.iter().map(board_from_row).collect())
    }

    pub async fn delete_board(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM boards WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_board_name(&self, id: Uuid, name: &str) -> Result<(), StorageError> {
        sqlx::query("UPDATE boards SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_board_position(&self, id: Uuid, position: i32) -> Result<(), StorageError> {
        sqlx::query("UPDATE boards SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;

    fn make_board(identity_id: Uuid) -> Board {
        Board {
            id: Uuid::new_v4(),
            identity_id,
            name: "Test Board".to_string(),
            position: 0,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn insert_and_get_round_trip() {
        let db = test_db().await;
        let identity = Uuid::new_v4();
        let board = make_board(identity);
        db.insert_board(&board).await.unwrap();
        let fetched = db.get_board(board.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, board.id);
        assert_eq!(fetched.name, board.name);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let db = test_db().await;
        assert!(db.get_board(Uuid::new_v4()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_boards_filters_by_identity() {
        let db = test_db().await;
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        db.insert_board(&make_board(id1)).await.unwrap();
        db.insert_board(&make_board(id1)).await.unwrap();
        db.insert_board(&make_board(id2)).await.unwrap();
        assert_eq!(db.list_boards(id1).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn update_name_and_position() {
        let db = test_db().await;
        let board = make_board(Uuid::new_v4());
        db.insert_board(&board).await.unwrap();
        db.update_board_name(board.id, "Renamed").await.unwrap();
        db.update_board_position(board.id, 3).await.unwrap();
        let fetched = db.get_board(board.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Renamed");
        assert_eq!(fetched.position, 3);
    }

    #[tokio::test]
    async fn delete_board() {
        let db = test_db().await;
        let board = make_board(Uuid::new_v4());
        db.insert_board(&board).await.unwrap();
        db.delete_board(board.id).await.unwrap();
        assert!(db.get_board(board.id).await.unwrap().is_none());
    }
}
