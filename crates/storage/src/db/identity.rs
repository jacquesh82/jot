use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;

pub struct Identity {
    pub id: String,
    pub friendly_name: String,
    pub created_at: String,
    pub public_key_x25519: Option<Vec<u8>>,
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
        let row = sqlx::query(
            "SELECT id, friendly_name, created_at, public_key_x25519 FROM identities WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| Identity {
            id: r.get("id"),
            friendly_name: r.get("friendly_name"),
            created_at: r.get("created_at"),
            public_key_x25519: r.get("public_key_x25519"),
        }))
    }

    pub async fn get_identity_by_name(&self, name: &str) -> Result<Option<Identity>, StorageError> {
        let row = sqlx::query(
            "SELECT id, friendly_name, created_at, public_key_x25519 FROM identities \
             WHERE LOWER(friendly_name) = LOWER(?)",
        )
        .bind(name)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| Identity {
            id: r.get("id"),
            friendly_name: r.get("friendly_name"),
            created_at: r.get("created_at"),
            public_key_x25519: r.get("public_key_x25519"),
        }))
    }

    pub async fn get_recent_contacts(
        &self,
        identity_id: &str,
    ) -> Result<Vec<Identity>, StorageError> {
        let rows = sqlx::query(
            "SELECT DISTINCT i.id, i.friendly_name, i.created_at, i.public_key_x25519 \
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
                public_key_x25519: r.get("public_key_x25519"),
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

    /// Delete all data owned by this identity and return the blob keys that must be purged
    /// from the blob store by the caller.
    pub async fn delete_identity_cascade(
        &self,
        identity_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        // Collect blob keys for all notes in owned boards.
        let blob_keys: Vec<String> = sqlx::query(
            "SELECT n.blob_key FROM notes n \
             JOIN boards b ON n.board_id = b.id \
             WHERE b.identity_id = ?",
        )
        .bind(identity_id)
        .fetch_all(&self.0)
        .await?
        .into_iter()
        .map(|r| r.get::<String, _>("blob_key"))
        .collect();

        // Delete in dependency order.
        sqlx::query(
            "DELETE FROM note_shares WHERE note_id IN \
             (SELECT n.id FROM notes n JOIN boards b ON n.board_id = b.id WHERE b.identity_id = ?)",
        )
        .bind(identity_id)
        .execute(&self.0)
        .await?;

        sqlx::query(
            "DELETE FROM board_keys WHERE board_id IN \
             (SELECT id FROM boards WHERE identity_id = ?)",
        )
        .bind(identity_id)
        .execute(&self.0)
        .await?;

        sqlx::query(
            "DELETE FROM board_shares WHERE board_id IN \
             (SELECT id FROM boards WHERE identity_id = ?)",
        )
        .bind(identity_id)
        .execute(&self.0)
        .await?;

        sqlx::query(
            "DELETE FROM notes WHERE board_id IN \
             (SELECT id FROM boards WHERE identity_id = ?)",
        )
        .bind(identity_id)
        .execute(&self.0)
        .await?;

        sqlx::query("DELETE FROM boards WHERE identity_id = ?")
            .bind(identity_id)
            .execute(&self.0)
            .await?;

        sqlx::query("DELETE FROM devices WHERE identity_id = ?")
            .bind(identity_id)
            .execute(&self.0)
            .await?;

        sqlx::query("DELETE FROM identities WHERE id = ?")
            .bind(identity_id)
            .execute(&self.0)
            .await?;

        Ok(blob_keys)
    }

    pub async fn set_identity_pubkey(&self, id: &str, pubkey: &[u8]) -> Result<(), StorageError> {
        sqlx::query("UPDATE identities SET public_key_x25519 = ? WHERE id = ?")
            .bind(pubkey)
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
