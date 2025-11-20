use crate::error::StorageError;

use aws_sdk_s3::{operation::head_object::HeadObjectOutput, Client};
use std::sync::Arc;
use tokio::sync::OnceCell;

static S3_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

async fn get_s3_client() -> Arc<Client> {
    S3_CLIENT
        .get_or_init(|| async {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Arc::new(Client::new(&config))
        })
        .await
        .clone()
}

pub struct S3Repository {
    client: Arc<Client>,
}

impl S3Repository {
    pub async fn new() -> Self {
        Self {
            client: get_s3_client().await,
        }
    }
}

#[async_trait::async_trait]
impl crate::repository::S3RepositoryTrait for S3Repository {
    async fn get_object_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<HeadObjectOutput, StorageError> {
        let response = self
            .client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::S3 {
                context: format!("Failed to get S3 object metadata for s3://{}/{}", bucket, key),
                source: e.into(),
            })?;

        Ok(response)
    }

    async fn fetch_head_bytes(
        &self,
        bucket: &str,
        key: &str,
        num_bytes: u64,
    ) -> Result<Vec<u8>, StorageError> {
        let range = format!("bytes={}-{}", 0, num_bytes - 1);

        let response = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .range(&range)
            .send()
            .await
            .map_err(|e| StorageError::S3 {
                context: format!("Failed to fetch byte range {} from S3 object s3://{}/{}", range, bucket, key),
                source: e.into(),
            })?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::S3 {
                context: format!("Failed to read S3 response body for s3://{}/{}", bucket, key),
                source: e.into(),
            })?
            .into_bytes();

        Ok(bytes.to_vec())
    }
}
