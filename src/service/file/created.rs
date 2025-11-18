use crate::service::metadata::MetadataService;
use crate::{error::StorageError, repository, service::File, utils::time};

use std::sync::LazyLock;
use tracing::{error, info};
// use uuid::Uuid;

static METADATA_SERVICE: LazyLock<MetadataService> = LazyLock::new(MetadataService::new);

pub async fn handle_file_created(mut file: File) -> Result<(), StorageError> {
    let bucket = &file.bucket;
    let key = &file.bucket_key;

    info!(
        bucket = %bucket,
        key = %key,
        "Processing file"
    );

    let parts: Vec<&str> = key.splitn(2, '/').collect();

    if parts.len() != 2 {
        return Err(StorageError::InvalidFormat(format!(
            "Invalid S3 key format: {}",
            key
        )));
    }

    // let owner_id = parts[0];
    // let file_path = parts[1];
    // let file_id = Uuid::new_v4().to_string();

    match repository::s3::get_object_metadata(bucket, key).await {
        Ok(response) => {
            if let Some(ct) = response.content_type() {
                file.content_type = ct.to_string();
            }

            if let Some(len) = response.content_length() {
                file.size_bytes = len;
            }

            if let Some(lm) = response.last_modified() {
                if let Some(timestamp) =
                    time::parse_media_datetime_with_offset(&lm.to_string(), None)
                {
                    file.created_date = timestamp;
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

    let num_bytes = std::cmp::min(512 * 1024, file.size_bytes as u64);

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

    info!("File processed successfully");

    Ok(())
}
