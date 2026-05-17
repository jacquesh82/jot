use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::{BlockLink, LinkKind, TargetKind};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackLinkRow {
    pub source_block_id: String,
    pub source_note_id: String,
    pub link_kind: String,
}

fn row_to_link(row: &sqlx::sqlite::SqliteRow) -> BlockLink {
    let id: String = row.get("id");
    let src: String = row.get("source_block_id");
    let tk: String = row.get("target_kind");
    let lk: String = row.get("link_kind");
    let created: String = row.get("created_at");
    BlockLink {
        id: Uuid::parse_str(&id).unwrap(),
        source_block_id: Uuid::parse_str(&src).unwrap(),
        target_kind: TargetKind::from_str(&tk).unwrap_or(TargetKind::Note),
        target_id: row.get("target_id"),
        link_kind: LinkKind::from_str(&lk).unwrap_or(LinkKind::PageRef),
        created_at: chrono::DateTime::parse_from_rfc3339(&created)
            .unwrap()
            .with_timezone(&Utc),
    }
}

impl Db {
    pub async fn replace_links_for_block(
        &self,
        source: Uuid,
        links: &[BlockLink],
    ) -> Result<(), StorageError> {
        let mut tx = self.0.begin().await?;
        sqlx::query("DELETE FROM block_links WHERE source_block_id = ?")
            .bind(source.to_string())
            .execute(&mut *tx)
            .await?;
        for l in links {
            sqlx::query(
                "INSERT INTO block_links (id, source_block_id, target_kind, target_id, link_kind, created_at)
                 VALUES (?,?,?,?,?,?)"
            )
            .bind(l.id.to_string()).bind(source.to_string())
            .bind(l.target_kind.as_str()).bind(&l.target_id)
            .bind(l.link_kind.as_str()).bind(l.created_at.to_rfc3339())
            .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_links_from(&self, source: Uuid) -> Result<Vec<BlockLink>, StorageError> {
        let rows = sqlx::query("SELECT * FROM block_links WHERE source_block_id = ?")
            .bind(source.to_string())
            .fetch_all(&self.0)
            .await?;
        Ok(rows.iter().map(row_to_link).collect())
    }

    pub async fn backlinks_to_block(&self, target: Uuid) -> Result<Vec<BlockLink>, StorageError> {
        let rows = sqlx::query(
            "SELECT * FROM block_links WHERE target_kind = 'block' AND target_id = ? ORDER BY created_at ASC"
        ).bind(target.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(row_to_link).collect())
    }

    pub async fn backlinks_to_note(&self, target: Uuid) -> Result<Vec<BlockLink>, StorageError> {
        let rows = sqlx::query(
            "SELECT * FROM block_links WHERE target_kind = 'note' AND target_id = ? ORDER BY created_at ASC"
        ).bind(target.to_string()).fetch_all(&self.0).await?;
        Ok(rows.iter().map(row_to_link).collect())
    }

    pub async fn blocks_with_tag(&self, name: &str) -> Result<Vec<Uuid>, StorageError> {
        let rows = sqlx::query(
            "SELECT DISTINCT source_block_id FROM block_links WHERE target_kind = 'tag' AND target_id = ?"
        ).bind(name).fetch_all(&self.0).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let s: String = r.get("source_block_id");
                Uuid::parse_str(&s).unwrap()
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;

    #[tokio::test]
    async fn replace_links_is_idempotent() {
        let db = test_db().await;
        let src = Uuid::new_v4();
        let board = Uuid::new_v4();
        let note = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO boards (id, identity_id, name, position, created_at) VALUES (?,?,?,?,?)",
        )
        .bind(board.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind("b")
        .bind(0i32)
        .bind(Utc::now().to_rfc3339())
        .execute(&db.0)
        .await
        .unwrap();
        sqlx::query("INSERT INTO notes (id, note_type, content, color, board_id, position, blob_key, size, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(note.to_string()).bind("text").bind(b"".to_vec()).bind("#FFF").bind(board.to_string()).bind(0i32).bind(Uuid::new_v4().to_string()).bind(0i64)
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();
        sqlx::query("INSERT INTO blocks (id, note_id, position, block_type, content, created_at, updated_at) VALUES (?,?,?,?,?,?,?)")
            .bind(src.to_string()).bind(note.to_string()).bind(1.0f64).bind("text").bind(b"x".to_vec())
            .bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(&db.0).await.unwrap();

        let now = Utc::now();
        let mk = |tk: TargetKind, tid: &str, lk: LinkKind| BlockLink {
            id: Uuid::new_v4(),
            source_block_id: src,
            target_kind: tk,
            target_id: tid.into(),
            link_kind: lk,
            created_at: now,
        };
        db.replace_links_for_block(src, &[mk(TargetKind::Tag, "todo", LinkKind::Tag)])
            .await
            .unwrap();
        db.replace_links_for_block(src, &[mk(TargetKind::Tag, "later", LinkKind::Tag)])
            .await
            .unwrap();
        let links = db.list_links_from(src).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_id, "later");
    }
}
