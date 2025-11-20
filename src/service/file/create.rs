use crate::service::file::utils::calculate_folder_prefix;
use crate::service::metadata::MetadataServiceTrait;
use crate::service::models::ViewLink;
use crate::utils::string;
use crate::{
    error::StorageError,
    repository::{DynamoDbRepositoryTrait, S3RepositoryTrait},
    service::File,
    utils::time,
};

use std::path::Path;
use tracing::{error, info};

pub async fn handle_file_created(
    file: File,
    s3_repository: &impl S3RepositoryTrait,
    dynamo_db_repository: &impl DynamoDbRepositoryTrait,
    metadata_service: &impl MetadataServiceTrait,
) -> Result<(), StorageError> {
    let bucket = file.bucket.clone();
    let key = file.bucket_key.clone();

    info!(bucket = %bucket, key = %key, "Processing file");

    let mut file = parse_and_init_file(file)?;

    enrich_with_s3_metadata(&mut file, s3_repository).await;

    enrich_with_media_metadata(&mut file, s3_repository, metadata_service).await?;

    let view_link = ViewLink::for_owner(&file);

    dynamo_db_repository
        .put_file_and_view_link(&file, &view_link)
        .await
        .map_err(|e| StorageError::DynamoDb {
            context: "Failed to put file and view link in DynamoDB".to_string(),
            source: e.into(),
        })?;

    info!("File processed successfully");

    Ok(())
}

fn parse_and_init_file(mut file: File) -> Result<File, StorageError> {
    let key = &file.bucket_key;
    let bucket = &file.bucket;

    let (owner, path) = key
        .split_once('/')
        .ok_or_else(|| StorageError::InvalidFormat {
            context: format!("Invalid S3 key format: {}", key),
        })?;

    file.owner_id = owner.into();
    file.file_id = string::sha256_hash(&format!("{}/{}", bucket, key)).into();
    file.file_name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .into();
    file.file_path = path.into();
    file.folder_prefix = calculate_folder_prefix(path).into();

    Ok(file)
}

async fn enrich_with_s3_metadata(file: &mut File, s3_repository: &impl S3RepositoryTrait) {
    match s3_repository
        .get_object_metadata(&file.bucket, &file.bucket_key)
        .await
    {
        Ok(response) => {
            if let Some(ct) = response.content_type() {
                file.content_type = ct.into();
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
                    tracing::debug!(
                        bucket = %file.bucket,
                        key = %file.bucket_key,
                        "Could not parse S3 last_modified"
                    );
                }
            }
        }
        Err(e) => {
            error!(
                bucket = %file.bucket,
                key = %file.bucket_key,
                error = %e,
                "Failed to fetch object metadata from S3; continuing without the extra data"
            );
        }
    }
}

async fn enrich_with_media_metadata(
    file: &mut File,
    s3_repository: &impl S3RepositoryTrait,
    metadata_service: &impl MetadataServiceTrait,
) -> Result<(), StorageError> {
    let num_bytes = std::cmp::min(512 * 1024, file.size_bytes as u64);

    let head_bytes = s3_repository
        .fetch_head_bytes(&file.bucket, &file.bucket_key, num_bytes)
        .await
        .map_err(|e| StorageError::S3 {
            context: format!("Failed to fetch first {} bytes from S3 for {}/{}", num_bytes, file.bucket, file.bucket_key),
            source: e.into(),
        })?;

    info!(
        "Fetched {} bytes from S3 for metadata extraction",
        head_bytes.len()
    );

    metadata_service
        .extract_metadata(&head_bytes, file)
        .await;

    Ok(())
}
