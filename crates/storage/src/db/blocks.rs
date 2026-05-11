use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::{Block, BlockType};
use sqlx::Row;
use uuid::Uuid;

fn row_to_block(row: &sqlx::sqlite::SqliteRow) -> Block {
    let id: String = row.get("id");
    let note_id: String = row.get("note_id");
    let parent: Option<String> = row.get("parent_block_id");
    let bt: String = row.get("block_type");
    let created: String = row.get("created_at");
    let updated: String = row.get("updated_at");
    let collapsed: i64 = row.get("collapsed");
    Block {
        id: Uuid::parse_str(&id).unwrap(),
        note_id: Uuid::parse_str(&note_id).unwrap(),
        parent_block_id: parent.and_then(|s| Uuid::parse_str(&s).ok()),
        position: row.get("position"),
        block_type: BlockType::from_str(&bt),
        content: row.get("content"),
        metadata: row.get("metadata"),
        collapsed: collapsed != 0,
        created_at: chrono::DateTime::parse_from_rfc3339(&created).unwrap().with_timezone(&Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated).unwrap().with_timezone(&Utc),
    }
}

impl Db {
    pub async fn insert_block(&self, b: &Block) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO blocks (id, note_id, parent_block_id, position, block_type, content, metadata, collapsed, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(b.id.to_string())
        .bind(b.note_id.to_string())
        .bind(b.parent_block_id.map(|p| p.to_string()))
        .bind(b.position)
        .bind(b.block_type.as_str())
        .bind(&b.content)
        .bind(b.metadata.as_deref())
        .bind(if b.collapsed { 1i64 } else { 0i64 })
        .bind(b.created_at.to_rfc3339())
        .bind(b.updated_at.to_rfc3339())
        .execute(&self.0).await?;
        Ok(())
    }

    pub async fn get_block(&self, id: Uuid) -> Result<Option<Block>, StorageError> {
        let row = sqlx::query("SELECT * FROM blocks WHERE id = ?")
            .bind(id.to_string()).fetch_optional(&self.0).await?;
        Ok(row.map(|r| row_to_block(&r)))
    }

    pub async fn list_blocks_for_note(&self, note_id: Uuid) -> Result<Vec<Block>, StorageError> {
        let rows = sqlx::query(
            "SELECT * FROM blocks WHERE note_id = ? ORDER BY COALESCE(parent_block_id,''), position ASC"
        ).bind(note_id.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(row_to_block).collect())
    }

    pub async fn update_block_content(&self, id: Uuid, content: &[u8], metadata: Option<&[u8]>, block_type: BlockType) -> Result<(), StorageError> {
        sqlx::query("UPDATE blocks SET content = ?, metadata = ?, block_type = ?, updated_at = ? WHERE id = ?")
            .bind(content).bind(metadata).bind(block_type.as_str())
            .bind(Utc::now().to_rfc3339()).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn move_block(&self, id: Uuid, new_parent: Option<Uuid>, new_position: f64) -> Result<(), StorageError> {
        sqlx::query("UPDATE blocks SET parent_block_id = ?, position = ?, updated_at = ? WHERE id = ?")
            .bind(new_parent.map(|p| p.to_string())).bind(new_position)
            .bind(Utc::now().to_rfc3339()).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn set_block_collapsed(&self, id: Uuid, collapsed: bool) -> Result<(), StorageError> {
        sqlx::query("UPDATE blocks SET collapsed = ? WHERE id = ?")
            .bind(if collapsed { 1i64 } else { 0i64 }).bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn delete_block(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM blocks WHERE id = ?").bind(id.to_string())
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn max_position(&self, note_id: Uuid, parent: Option<Uuid>) -> Result<f64, StorageError> {
        let row = sqlx::query(
            "SELECT COALESCE(MAX(position), 0.0) AS m FROM blocks WHERE note_id = ? AND parent_block_id IS ?"
        )
        .bind(note_id.to_string())
        .bind(parent.map(|p| p.to_string()))
        .fetch_one(&self.0).await?;
        Ok(row.get::<f64, _>("m"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;
    use jot_core::models::BlockType;

    async fn seed_note(db: &Db) -> (Uuid, Uuid) {
        let board = Uuid::new_v4();
        let note = Uuid::new_v4();
        sqlx::query("INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?,?,?,?,?)")
            .bind(board.to_string()).bind(Uuid::new_v4().to_string()).bind("b").bind(0i32).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        sqlx::query("INSERT INTO notes (id, note_type, content, color, board_id, position, blob_key, size, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(note.to_string()).bind("text").bind(b"".to_vec()).bind("#FFF").bind(board.to_string()).bind(0i32).bind(Uuid::new_v4().to_string()).bind(0i64)
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        (board, note)
    }

    fn make_block(note_id: Uuid, parent: Option<Uuid>, pos: f64) -> Block {
        let now = Utc::now();
        Block {
            id: Uuid::new_v4(), note_id, parent_block_id: parent, position: pos,
            block_type: BlockType::Text, content: b"hello".to_vec(), metadata: None, collapsed: false,
            created_at: now, updated_at: now,
        }
    }

    #[tokio::test]
    async fn insert_and_list_blocks() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        db.insert_block(&make_block(n, None, 1.0)).await.unwrap();
        db.insert_block(&make_block(n, None, 2.0)).await.unwrap();
        let blocks = db.list_blocks_for_note(n).await.unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[tokio::test]
    async fn cascade_delete_subtree() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        let parent = make_block(n, None, 1.0);
        db.insert_block(&parent).await.unwrap();
        let child = make_block(n, Some(parent.id), 1.0);
        db.insert_block(&child).await.unwrap();
        db.delete_block(parent.id).await.unwrap();
        assert_eq!(db.list_blocks_for_note(n).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn move_block_reparents() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        let a = make_block(n, None, 1.0);
        let b = make_block(n, None, 2.0);
        db.insert_block(&a).await.unwrap();
        db.insert_block(&b).await.unwrap();
        db.move_block(b.id, Some(a.id), 1.0).await.unwrap();
        let fetched = db.get_block(b.id).await.unwrap().unwrap();
        assert_eq!(fetched.parent_block_id, Some(a.id));
    }

    #[tokio::test]
    async fn max_position_with_no_siblings_is_zero() {
        let db = test_db().await;
        let (_b, n) = seed_note(&db).await;
        assert_eq!(db.max_position(n, None).await.unwrap(), 0.0);
    }
}
