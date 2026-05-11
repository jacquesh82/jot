pub mod board_keys;
pub mod board_shares;
pub mod boards;
pub mod devices;
pub mod identity;
pub mod invites;
pub mod links;
pub mod notes;
pub mod shares;

use crate::StorageError;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;

pub struct Db(pub(crate) SqlitePool);

impl Db {
    pub async fn connect(url: &str) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        Ok(Db(pool))
    }

    pub async fn migrate(&self) -> Result<(), StorageError> {
        sqlx::migrate!("./migrations").run(&self.0).await?;
        Ok(())
    }

    /// Run migrations and return (version_before, version_after).
    /// Callers can compare the two values to decide whether to log a diff.
    pub async fn migrate_with_version(&self) -> Result<(i64, i64), StorageError> {
        let before = self.schema_version().await?;
        sqlx::migrate!("./migrations").run(&self.0).await?;
        let after = self.schema_version().await?;
        Ok((before, after))
    }

    /// Returns the version number of the highest successfully applied migration,
    /// or 0 if no migrations have been run yet.
    pub async fn schema_version(&self) -> Result<i64, StorageError> {
        // _sqlx_migrations is created by sqlx::migrate! on first run.
        // It may not exist if the DB was never migrated.
        let exists: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
        )
        .fetch_one(&self.0)
        .await
        .unwrap_or(false);

        if !exists {
            return Ok(0);
        }

        let version: Option<i64> =
            sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success = TRUE")
                .fetch_optional(&self.0)
                .await?
                .flatten();

        Ok(version.unwrap_or(0))
    }
}

#[cfg(test)]
pub(crate) async fn test_db() -> Db {
    let db = Db::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    db
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn connect_and_migrate_memory() {
        let db = test_db().await;
        let _ = db;
    }

    #[tokio::test]
    async fn connect_and_migrate_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        // File does not exist yet — must be created by connect()
        assert!(!db_path.exists());
        let url = format!("sqlite://{}", db_path.display());
        let db = Db::connect(&url).await.unwrap();
        db.migrate().await.unwrap();
        assert!(db_path.exists());
    }
}
