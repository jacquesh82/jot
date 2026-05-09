pub mod local;
pub mod s3;

#[async_trait::async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, key: &str, data: &[u8]) -> Result<(), crate::StorageError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, crate::StorageError>;
    async fn delete(&self, key: &str) -> Result<(), crate::StorageError>;
}
