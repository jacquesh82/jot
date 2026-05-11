use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::{Note, NoteType};
use sqlx::Row;
use uuid::Uuid;

fn note_type_str(nt: &NoteType) -> &'static str {
    match nt {
        NoteType::Text => "text",
        NoteType::Voice => "voice",
        NoteType::Image => "image",
    }
}

fn str_note_type(s: &str) -> NoteType {
    match s {
        "voice" => NoteType::Voice,
        "image" => NoteType::Image,
        _ => NoteType::Text,
    }
}

fn note_from_row(row: &sqlx::sqlite::SqliteRow) -> Note {
    let id_str: String = row.get("id");
    let board_str: String = row.get("board_id");
    let created_str: String = row.get("created_at");
    let updated_str: String = row.get("updated_at");
    let nt_str: String = row.get("note_type");
    Note {
        id: Uuid::parse_str(&id_str).unwrap(),
        note_type: str_note_type(&nt_str),
        content: row.get("content"),
        thumbnail: row.get("thumbnail"),
        duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u32),
        color: row.get("color"),
        board_id: Uuid::parse_str(&board_str).unwrap(),
        position: row.get("position"),
        blob_key: row.get("blob_key"),
        size: row.get("size"),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
            .unwrap()
            .with_timezone(&Utc),
    }
}

impl Db {
    pub async fn insert_note(&self, note: &Note) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO notes (id, note_type, content, thumbnail, duration_ms, color, board_id, position, blob_key, size, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(note.id.to_string())
        .bind(note_type_str(&note.note_type))
        .bind(&note.content)
        .bind(&note.thumbnail)
        .bind(note.duration_ms.map(|d| d as i64))
        .bind(&note.color)
        .bind(note.board_id.to_string())
        .bind(note.position)
        .bind(&note.blob_key)
        .bind(note.size)
        .bind(note.created_at.to_rfc3339())
        .bind(note.updated_at.to_rfc3339())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_note(&self, id: Uuid) -> Result<Option<Note>, StorageError> {
        let row = sqlx::query(
            "SELECT id, note_type, content, thumbnail, duration_ms, color, board_id, position, blob_key, size, created_at, updated_at
             FROM notes WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| note_from_row(&r)))
    }

    pub async fn list_notes(&self, board_id: Uuid) -> Result<Vec<Note>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, note_type, content, thumbnail, duration_ms, color, board_id, position, blob_key, size, created_at, updated_at
             FROM notes WHERE board_id = ? ORDER BY position ASC"
        )
        .bind(board_id.to_string())
        .fetch_all(&self.0)
        .await?;
        Ok(rows.iter().map(note_from_row).collect())
    }

    pub async fn delete_note(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_note_position(&self, id: Uuid, position: i32) -> Result<(), StorageError> {
        sqlx::query("UPDATE notes SET position = ?, updated_at = ? WHERE id = ?")
            .bind(position)
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_note_color(&self, id: Uuid, color: &str) -> Result<(), StorageError> {
        sqlx::query("UPDATE notes SET color = ?, updated_at = ? WHERE id = ?")
            .bind(color)
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_note_snippet(&self, id: Uuid, snippet: &[u8]) -> Result<(), StorageError> {
        sqlx::query("UPDATE notes SET content = ?, updated_at = ? WHERE id = ?")
            .bind(snippet)
            .bind(Utc::now().to_rfc3339())
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

    fn make_note(board_id: Uuid) -> Note {
        let now = Utc::now();
        Note {
            id: Uuid::new_v4(),
            note_type: NoteType::Text,
            content: b"encrypted blob".to_vec(),
            thumbnail: None,
            duration_ms: None,
            color: "#FFFFFF".to_string(),
            board_id,
            position: 0,
            blob_key: Uuid::new_v4().to_string(),
            size: 14,
            created_at: now,
            updated_at: now,
        }
    }

    async fn insert_board(db: &Db, board_id: Uuid) {
        sqlx::query(
            "INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(board_id.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind("Test Board")
        .bind(0i32)
        .bind(Utc::now().to_rfc3339())
        .execute(&db.0)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn insert_and_get_round_trip() {
        let db = test_db().await;
        let board_id = Uuid::new_v4();
        insert_board(&db, board_id).await;
        let note = make_note(board_id);
        db.insert_note(&note).await.unwrap();
        let fetched = db.get_note(note.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, note.id);
        assert_eq!(fetched.content, note.content);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let db = test_db().await;
        assert!(db.get_note(Uuid::new_v4()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_notes_filters_by_board() {
        let db = test_db().await;
        let b1 = Uuid::new_v4();
        let b2 = Uuid::new_v4();
        insert_board(&db, b1).await;
        insert_board(&db, b2).await;
        db.insert_note(&make_note(b1)).await.unwrap();
        db.insert_note(&make_note(b1)).await.unwrap();
        db.insert_note(&make_note(b2)).await.unwrap();
        assert_eq!(db.list_notes(b1).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn delete_note() {
        let db = test_db().await;
        let board_id = Uuid::new_v4();
        insert_board(&db, board_id).await;
        let note = make_note(board_id);
        db.insert_note(&note).await.unwrap();
        db.delete_note(note.id).await.unwrap();
        assert!(db.get_note(note.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn update_position_and_color() {
        let db = test_db().await;
        let board_id = Uuid::new_v4();
        insert_board(&db, board_id).await;
        let note = make_note(board_id);
        db.insert_note(&note).await.unwrap();
        db.update_note_position(note.id, 5).await.unwrap();
        db.update_note_color(note.id, "#FF0000").await.unwrap();
        let fetched = db.get_note(note.id).await.unwrap().unwrap();
        assert_eq!(fetched.position, 5);
        assert_eq!(fetched.color, "#FF0000");
    }
}
