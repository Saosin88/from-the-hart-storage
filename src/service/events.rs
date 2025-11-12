use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde_json;
use tracing::{debug, error, info};

use super::metadata::MetadataService;
use crate::repository;
use crate::service::FileRecord;

static METADATA_SERVICE: Lazy<MetadataService> = Lazy::new(MetadataService::new);

pub async fn process_s3_event_message(file_record: FileRecord) -> Result<()> {
    let bucket = &file_record.bucket;
    let key = &file_record.file_name;
    let size = file_record.file_size;

    info!(
        bucket = %bucket,
        key = %key,
        size = %size,
        "Processing file record"
    );

    // If content_type or last_modified missing, try to fill from S3
    let (content_type, last_modified) = if file_record.content_type.is_some() {
        (file_record.content_type.clone(), file_record.last_modified)
    } else {
        match repository::s3::get_object_metadata(bucket, key).await {
            Ok((ct, lm)) => (ct, Some(lm)),
            Err(e) => {
                error!(
                    bucket = %bucket,
                    key = %key,
                    error = %e,
                    "Failed to fetch object metadata from S3; continuing without content-type"
                );
                (None, file_record.last_modified)
            }
        }
    };

    // Determine how many bytes to fetch based on file type
    let num_bytes = if let Some(ref ct) = content_type {
        if ct.starts_with("video/") {
            std::cmp::min(1024 * 1024, size as u64)
        } else {
            std::cmp::min(512 * 1024, size as u64)
        }
    } else {
        std::cmp::min(512 * 1024, size as u64)
    };

    let head_bytes = repository::s3::fetch_head_bytes(bucket, key, num_bytes)
        .await
        .with_context(|| format!("Failed to fetch first {} bytes from S3 for {}/{}", num_bytes, bucket, key))?;

    info!("Fetched {} bytes from S3 for metadata extraction", head_bytes.len());

    match METADATA_SERVICE.extract_metadata(&head_bytes, &file_record).await {
        Ok(metadata) => {
            info!(
                bucket = %bucket,
                key = %key,
                media_type = ?metadata.media_type,
                file_size = %metadata.file_size,
                content_type = ?metadata.content_type,
                created_date = %metadata.created_date,
                "Extracted metadata"
            );

            if let Some(ref img_meta) = metadata.image_metadata {
                info!(
                    width = %img_meta.width,
                    height = %img_meta.height,
                    format = %img_meta.format,
                    has_exif = %img_meta.exif.is_some(),
                    "Image metadata"
                );
            }

            if let Some(ref video_meta) = metadata.video_metadata {
                info!(
                    width = ?video_meta.width,
                    height = ?video_meta.height,
                    duration = ?video_meta.duration,
                    codec = ?video_meta.codec,
                    frame_rate = ?video_meta.frame_rate,
                    bitrate = ?video_meta.bitrate,
                    "Video metadata"
                );
            }

            match serde_json::to_string(&metadata) {
                Ok(json) => {
                    info!(metadata = %json, "full_metadata");
                }
                Err(e) => {
                    error!("Failed to serialize metadata to JSON: {:?}", e);
                }
            }

            // TODO: Store metadata in DynamoDB repository
            // repository::dynamodb::store_metadata(&metadata).await?;
        }
        Err(e) => {
            error!(
                bucket = %bucket,
                key = %key,
                error = %e,
                error_chain = ?e,
                "Failed to extract metadata - skipping"
            );
        }
    }

    debug!("File processed successfully");

    Ok(())
}
