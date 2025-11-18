use crate::service::file::helpers::calculate_folder_prefix;
use crate::service::metadata::MetadataService;
use crate::{error::StorageError, repository, service::File, utils::time};

use std::path::Path;
use std::sync::LazyLock;
use tracing::{error, info};
use uuid::Uuid;

static METADATA_SERVICE: LazyLock<MetadataService> = LazyLock::new(MetadataService::new);

pub async fn handle_file_created(mut file: File) -> Result<(), StorageError> {
    let bucket = &file.bucket;
    let key = &file.bucket_key;

    info!(
        bucket = %bucket,
        key = %key,
        "Processing file"
    );

    let (owner, path) = key
        .split_once('/')
        .ok_or_else(|| StorageError::InvalidFormat(format!("Invalid S3 key format: {}", key)))?;

    file.owner_id = owner.to_string();
    file.file_id = Uuid::new_v4().to_string();
    file.file_name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string();
    file.file_path = path.to_string();
    file.folder_prefix = calculate_folder_prefix(path).to_string();

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
                "Failed to fetch object metadata from S3; continuing without the extra data"
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
