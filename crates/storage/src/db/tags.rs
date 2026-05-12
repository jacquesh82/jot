use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::Tag;
use sqlx::Row;
use uuid::Uuid;

impl Db {
    pub async fn upsert_tag(
        &self,
        name: &str,
        identity: Uuid,
        color: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO tags (name, identity_id, color, created_at) VALUES (?, ?, ?, ?)
             ON CONFLICT(name, identity_id) DO UPDATE SET color = excluded.color",
        )
        .bind(name)
        .bind(identity.to_string())
        .bind(color)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn list_tags(&self, identity: Uuid) -> Result<Vec<Tag>, StorageError> {
        let rows = sqlx::query(
            "SELECT name, identity_id, color, created_at FROM tags WHERE identity_id = ? ORDER BY name",
        )
        .bind(identity.to_string())
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let name: String = r.get("name");
                let id: String = r.get("identity_id");
                let color: Option<String> = r.get("color");
                let created: String = r.get("created_at");
                Tag {
                    name,
                    identity_id: Uuid::parse_str(&id).unwrap(),
                    color,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created)
                        .unwrap()
                        .with_timezone(&Utc),
                }
            })
            .collect())
    }

    pub async fn delete_tag(&self, name: &str, identity: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM tags WHERE name = ? AND identity_id = ?")
            .bind(name)
            .bind(identity.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;

    /// Helper to insert an identity row that satisfies the FK from tags.
    /// The identities table (migration 0002) requires: id, friendly_name (UNIQUE NOT NULL),
    /// created_at (NOT NULL).
    async fn seed_identity(db: &Db) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO identities (id, friendly_name, created_at) VALUES (?, ?, ?)")
            .bind(id.to_string())
            .bind(format!("ident-{}", id))
            .bind(Utc::now().to_rfc3339())
            .execute(&db.0)
            .await
            .expect("insert identity");
        id
    }

    #[tokio::test]
    async fn upsert_and_list() {
        let db = test_db().await;
        let id = seed_identity(&db).await;
        db.upsert_tag("projet-x", id, Some("#ff0")).await.unwrap();
        db.upsert_tag("projet-x", id, Some("#0f0")).await.unwrap();
        let tags = db.list_tags(id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].color.as_deref(), Some("#0f0"));
    }

    #[tokio::test]
    async fn delete_tag_removes_row() {
        let db = test_db().await;
        let id = seed_identity(&db).await;
        db.upsert_tag("todo", id, None).await.unwrap();
        assert_eq!(db.list_tags(id).await.unwrap().len(), 1);
        db.delete_tag("todo", id).await.unwrap();
        assert_eq!(db.list_tags(id).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn list_tags_scoped_per_identity() {
        let db = test_db().await;
        let a = seed_identity(&db).await;
        let b = seed_identity(&db).await;
        db.upsert_tag("a-tag", a, None).await.unwrap();
        db.upsert_tag("b-tag", b, None).await.unwrap();
        let ta = db.list_tags(a).await.unwrap();
        assert_eq!(ta.len(), 1);
        assert_eq!(ta[0].name, "a-tag");
    }
}
