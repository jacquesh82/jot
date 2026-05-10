use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct InviteToken {
    pub token: String,
    pub created_by: String,
    pub label: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

impl Db {
    pub async fn create_invite_token(
        &self,
        token: &str,
        created_by: &str,
        label: &str,
    ) -> Result<InviteToken, StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO invite_tokens (token, created_by, label, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(token)
        .bind(created_by)
        .bind(label)
        .bind(&now)
        .execute(&self.0)
        .await?;
        Ok(InviteToken {
            token: token.to_string(),
            created_by: created_by.to_string(),
            label: label.to_string(),
            created_at: now,
            revoked_at: None,
        })
    }

    pub async fn list_invite_tokens(
        &self,
        created_by: &str,
    ) -> Result<Vec<InviteToken>, StorageError> {
        let rows = sqlx::query(
            "SELECT token, created_by, label, created_at, revoked_at FROM invite_tokens WHERE created_by = ? ORDER BY created_at DESC",
        )
        .bind(created_by)
        .fetch_all(&self.0)
        .await?;
        Ok(rows
            .iter()
            .map(|r| InviteToken {
                token: r.get("token"),
                created_by: r.get("created_by"),
                label: r.get("label"),
                created_at: r.get("created_at"),
                revoked_at: r.get("revoked_at"),
            })
            .collect())
    }

    pub async fn get_invite_token(&self, token: &str) -> Result<Option<InviteToken>, StorageError> {
        let row = sqlx::query(
            "SELECT token, created_by, label, created_at, revoked_at FROM invite_tokens WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| InviteToken {
            token: r.get("token"),
            created_by: r.get("created_by"),
            label: r.get("label"),
            created_at: r.get("created_at"),
            revoked_at: r.get("revoked_at"),
        }))
    }

    pub async fn revoke_invite_token(
        &self,
        token: &str,
        revoked_by: &str,
    ) -> Result<bool, StorageError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE invite_tokens SET revoked_at = ? WHERE token = ? AND created_by = ? AND revoked_at IS NULL",
        )
        .bind(&now)
        .bind(token)
        .bind(revoked_by)
        .execute(&self.0)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
