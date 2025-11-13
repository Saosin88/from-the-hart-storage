use super::metadata::MetadataService;
use crate::{error::StorageError, repository, service::File};

use chrono::Utc;
use serde_json;
use std::sync::LazyLock;
use tracing::{debug, error, info};

static METADATA_SERVICE: LazyLock<MetadataService> = LazyLock::new(MetadataService::new);

pub async fn process_s3_event_message(mut file: File) -> Result<(), StorageError> {
    let bucket = &file.bucket;
    let key = &file.bucket_key;

    info!(
        bucket = %bucket,
        key = %key,
        "Processing file record"
    );

    match repository::s3::get_object_metadata(bucket, key).await {
        Ok(response) => {
            if let Some(ct) = response.content_type() {
                file.content_type = ct.to_string();
            }

            if let Some(len) = response.content_length() {
                file.content_length = len;
            }

            if let Some(lm) = response.last_modified() {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&lm.to_string()) {
                    file.created_date = parsed.with_timezone(&Utc);
                } else {
                    tracing::debug!(bucket = %bucket, key = %key, "Could not parse S3 last_modified");
                }
            }
        }
        Err(e) => {
            error!(
                bucket = %bucket,
                key = %key,
                error = %e,
                "Failed to fetch object metadata from S3; continuing without content-type"
            );
        }
    }

    let num_bytes = std::cmp::min(512 * 1024, file.content_length as u64);

    let head_bytes = repository::s3::fetch_head_bytes(bucket, key, num_bytes)
        .await
        .map_err(|e| {
            StorageError::S3(format!(
                "Failed to fetch first {} bytes from S3 for {}/{}: {}",
                num_bytes, bucket, key, e
            ))
        })?;

    info!(
        "Fetched {} bytes from S3 for metadata extraction",
        head_bytes.len()
    );

    METADATA_SERVICE
        .extract_metadata(&head_bytes, &mut file)
        .await;

    serde_json::to_string(&file)
        .map_err(|e| StorageError::Serialization(format!("Failed to serialize metadata: {}", e)))
        .map(|json| {
            info!(metadata = %json, "full_metadata");
        })?;

    debug!("File processed successfully");

    Ok(())
}
