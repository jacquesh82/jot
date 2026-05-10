pub mod boards;
pub mod devices;
pub mod links;
pub mod notes;

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
