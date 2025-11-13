use crate::error::StorageError;

use aws_sdk_s3::{operation::head_object::HeadObjectOutput, Client};
use std::sync::{Arc, OnceLock};

static S3_CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

async fn get_s3_client() -> Arc<Client> {
    S3_CLIENT
        .get_or_init(|| {
            let rt = tokio::runtime::Handle::current();
            let config = rt.block_on(async {
                aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
            });
            Arc::new(Client::new(&config))
        })
        .clone()
}

pub async fn get_object_metadata(
    bucket: &str,
    key: &str,
) -> Result<HeadObjectOutput, StorageError> {
    let s3_client = get_s3_client().await;

    let response = s3_client
        .head_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| {
            StorageError::S3(format!(
                "Failed to get S3 object metadata for s3://{}/{}: {}",
                bucket, key, e
            ))
        })?;

    Ok(response)
}

pub async fn fetch_head_bytes(
    bucket: &str,
    key: &str,
    num_bytes: u64,
) -> Result<Vec<u8>, StorageError> {
    let s3_client = get_s3_client().await;
    let range = format!("bytes={}-{}", 0, num_bytes - 1);

    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(&range)
        .send()
        .await
        .map_err(|e| {
            StorageError::S3(format!(
                "Failed to fetch byte range {} from S3 object s3://{}/{}: {}",
                range, bucket, key, e
            ))
        })?;

    let bytes = response
        .body
        .collect()
        .await
        .map_err(|e| {
            StorageError::S3(format!(
                "Failed to read S3 response body for s3://{}/{}: {}",
                bucket, key, e
            ))
        })?
        .into_bytes();

    Ok(bytes.to_vec())
}
