use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::error::AppError;

/// Private S3 client singleton
static S3_CLIENT: OnceCell<Arc<S3Client>> = OnceCell::const_new();

/// Initialize and get the S3 client
async fn get_s3_client() -> Arc<S3Client> {
    S3_CLIENT
        .get_or_init(|| async {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Arc::new(S3Client::new(&config))
        })
        .await
        .clone()
}

/// Get the content type and last modified date from S3 object metadata
pub async fn get_object_metadata(
    bucket: &str,
    key: &str,
) -> Result<(Option<String>, DateTime<Utc>), AppError> {
    let s3_client = get_s3_client().await;
    
    let response = s3_client
        .head_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| {
            AppError::S3Error(format!(
                "Failed to get S3 object metadata for s3://{}/{}: {}",
                bucket, key, e
            ))
        })?;

    let content_type = response.content_type().map(|s| s.to_string());

    let last_modified = response
        .last_modified()
        .and_then(|dt| {
            chrono::DateTime::parse_from_rfc3339(&dt.to_string())
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        })
        .unwrap_or_else(Utc::now);

    Ok((content_type, last_modified))
}

/// Fetch the first N bytes of an S3 object
/// Useful for reading file headers to determine format and extract metadata
pub async fn fetch_head_bytes(
    bucket: &str,
    key: &str,
    num_bytes: u64,
) -> Result<Vec<u8>, AppError> {
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
            AppError::S3Error(format!(
                "Failed to fetch byte range {} from S3 object s3://{}/{}: {}",
                range, bucket, key, e
            ))
        })?;

    let bytes = response
        .body
        .collect()
        .await
        .map_err(|e| {
            AppError::S3Error(format!(
                "Failed to read S3 response body for s3://{}/{}: {}",
                bucket, key, e
            ))
        })?
        .into_bytes();

    Ok(bytes.to_vec())
}
