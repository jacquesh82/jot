pub mod boards;
pub mod devices;
pub mod links;
pub mod notes;

use crate::StorageError;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub struct Db(pub(crate) SqlitePool);

impl Db {
    pub async fn connect(url: &str) -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
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

    #[tokio::test]
    async fn connect_and_migrate() {
        let db = test_db().await;
        let _ = db;
    }
}
