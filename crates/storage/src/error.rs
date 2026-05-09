#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("blob not found: {0}")]
    BlobNotFound(String),
    #[error("blob I/O error: {0}")]
    BlobIo(#[from] std::io::Error),
    #[error("S3 error: {0}")]
    S3(String),
}
