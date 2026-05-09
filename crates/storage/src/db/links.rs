use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::{LinkSession, LinkStatus};
use sqlx::Row;

fn link_from_row(row: &sqlx::sqlite::SqliteRow) -> LinkSession {
    let status_str: String = row.get("status");
    let expires_str: String = row.get("expires_at");
    LinkSession {
        token: row.get("token"),
        code: row.get("code"),
        status: match status_str.as_str() {
            "confirmed" => LinkStatus::Confirmed,
            "expired" => LinkStatus::Expired,
            _ => LinkStatus::Pending,
        },
        pub_key_initiator: row.get("pub_key_initiator"),
        encrypted_symkey: row.get("encrypted_symkey"),
        expires_at: chrono::DateTime::parse_from_rfc3339(&expires_str)
            .unwrap()
            .with_timezone(&Utc),
    }
}

impl Db {
    pub async fn insert_link(&self, link: &LinkSession) -> Result<(), StorageError> {
        let status = match link.status {
            LinkStatus::Pending => "pending",
            LinkStatus::Confirmed => "confirmed",
            LinkStatus::Expired => "expired",
        };
        sqlx::query(
            "INSERT INTO link_sessions (token, code, status, pub_key_initiator, encrypted_symkey, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&link.token)
        .bind(&link.code)
        .bind(status)
        .bind(&link.pub_key_initiator)
        .bind(&link.encrypted_symkey)
        .bind(link.expires_at.to_rfc3339())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_link(&self, token: &str) -> Result<Option<LinkSession>, StorageError> {
        let row = sqlx::query(
            "SELECT token, code, status, pub_key_initiator, encrypted_symkey, expires_at
             FROM link_sessions WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| link_from_row(&r)))
    }

    pub async fn confirm_link(
        &self,
        token: &str,
        encrypted_symkey: Vec<u8>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE link_sessions SET status = 'confirmed', encrypted_symkey = ? WHERE token = ?",
        )
        .bind(encrypted_symkey)
        .bind(token)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn expire_link(&self, token: &str) -> Result<(), StorageError> {
        sqlx::query("UPDATE link_sessions SET status = 'expired' WHERE token = ?")
            .bind(token)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;
    use uuid::Uuid;

    fn make_link() -> LinkSession {
        LinkSession {
            token: Uuid::new_v4().to_string(),
            code: "7842".to_string(),
            status: LinkStatus::Pending,
            pub_key_initiator: "aabbcc".to_string(),
            encrypted_symkey: None,
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        }
    }

    #[tokio::test]
    async fn insert_and_get_round_trip() {
        let db = test_db().await;
        let link = make_link();
        db.insert_link(&link).await.unwrap();
        let fetched = db.get_link(&link.token).await.unwrap().unwrap();
        assert_eq!(fetched.token, link.token);
        assert_eq!(fetched.status, LinkStatus::Pending);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let db = test_db().await;
        assert!(db.get_link("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn confirm_link_sets_status_and_symkey() {
        let db = test_db().await;
        let link = make_link();
        db.insert_link(&link).await.unwrap();
        let symkey = vec![1u8, 2, 3, 4];
        db.confirm_link(&link.token, symkey.clone()).await.unwrap();
        let fetched = db.get_link(&link.token).await.unwrap().unwrap();
        assert_eq!(fetched.status, LinkStatus::Confirmed);
        assert_eq!(fetched.encrypted_symkey, Some(symkey));
    }

    #[tokio::test]
    async fn expire_link_sets_status() {
        let db = test_db().await;
        let link = make_link();
        db.insert_link(&link).await.unwrap();
        db.expire_link(&link.token).await.unwrap();
        let fetched = db.get_link(&link.token).await.unwrap().unwrap();
        assert_eq!(fetched.status, LinkStatus::Expired);
    }
}
