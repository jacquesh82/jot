use crate::blobs::BlobStore;
use crate::StorageError;
use std::path::PathBuf;

pub struct LocalStore {
    base_path: PathBuf,
}

impl LocalStore {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }
}

#[async_trait::async_trait]
impl BlobStore for LocalStore {
    async fn put(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        let path = self.base_path.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, data).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.base_path.join(key);
        if !path.exists() {
            return Err(StorageError::BlobNotFound(key.to_string()));
        }
        Ok(tokio::fs::read(&path).await?)
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.base_path.join(key);
        tokio::fs::remove_file(&path).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn put_and_get_round_trip() {
        let dir = tempdir().unwrap();
        let store = LocalStore::new(dir.path());
        store.put("my-blob", b"hello blob").await.unwrap();
        let data = store.get("my-blob").await.unwrap();
        assert_eq!(data, b"hello blob");
    }

    #[tokio::test]
    async fn get_nonexistent_returns_blob_not_found() {
        let dir = tempdir().unwrap();
        let store = LocalStore::new(dir.path());
        let err = store.get("missing-key").await.unwrap_err();
        assert!(matches!(err, StorageError::BlobNotFound(_)));
    }

    #[tokio::test]
    async fn delete_removes_blob() {
        let dir = tempdir().unwrap();
        let store = LocalStore::new(dir.path());
        store.put("to-delete", b"data").await.unwrap();
        store.delete("to-delete").await.unwrap();
        let err = store.get("to-delete").await.unwrap_err();
        assert!(matches!(err, StorageError::BlobNotFound(_)));
    }
}
