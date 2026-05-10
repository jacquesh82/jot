use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;

pub struct Identity {
    pub id: String,
    pub friendly_name: String,
    pub created_at: String,
}

impl Db {
    pub async fn insert_identity(&self, id: &str, friendly_name: &str) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR IGNORE INTO identities (id, friendly_name, created_at) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(friendly_name)
        .bind(&now)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_identity_by_id(&self, id: &str) -> Result<Option<Identity>, StorageError> {
        let row = sqlx::query("SELECT id, friendly_name, created_at FROM identities WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.0)
            .await?;
        Ok(row.map(|r| Identity {
            id: r.get("id"),
            friendly_name: r.get("friendly_name"),
            created_at: r.get("created_at"),
        }))
    }

    pub async fn get_identity_by_name(&self, name: &str) -> Result<Option<Identity>, StorageError> {
        let row = sqlx::query(
            "SELECT id, friendly_name, created_at FROM identities WHERE LOWER(friendly_name) = LOWER(?)",
        )
        .bind(name)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| Identity {
            id: r.get("id"),
            friendly_name: r.get("friendly_name"),
            created_at: r.get("created_at"),
        }))
    }

    pub async fn get_recent_contacts(
        &self,
        identity_id: &str,
    ) -> Result<Vec<Identity>, StorageError> {
        let rows = sqlx::query(
            "SELECT DISTINCT i.id, i.friendly_name, i.created_at \
             FROM board_shares bs \
             JOIN identities i ON bs.shared_with_id = i.id \
             WHERE bs.owner_identity_id = ? \
             ORDER BY bs.created_at DESC \
             LIMIT 10",
        )
        .bind(identity_id)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| Identity {
                id: r.get("id"),
                friendly_name: r.get("friendly_name"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn update_identity_name(&self, id: &str, name: &str) -> Result<bool, StorageError> {
        let r = sqlx::query("UPDATE identities SET friendly_name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}
