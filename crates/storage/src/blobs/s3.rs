use crate::blobs::BlobStore;
use crate::StorageError;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;

pub struct S3Store {
    client: Client,
    bucket: String,
}

impl S3Store {
    pub async fn new(bucket: String, region: String, endpoint_url: Option<String>) -> Self {
        let mut config_builder = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_s3::config::Region::new(region));

        if let Some(url) = endpoint_url {
            config_builder = config_builder.endpoint_url(url);
        }

        let config = config_builder.load().await;
        let client = Client::new(&config);
        Self { client, bucket }
    }
}

#[async_trait::async_trait]
impl BlobStore for S3Store {
    async fn put(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data.to_vec()))
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?
            .into_bytes();
        Ok(bytes.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;
        Ok(())
    }
}
