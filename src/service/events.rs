use anyhow::{Context, Result};
use aws_lambda_events::event::{s3::S3Event, sqs::SqsMessage};
use once_cell::sync::Lazy;
use serde_json;
use std::borrow::Cow;
use tracing::{debug, error, info};

use super::metadata::MetadataService;
use crate::repository;
use crate::service::FileRecord;

static METADATA_SERVICE: Lazy<MetadataService> = Lazy::new(MetadataService::new);

fn url_decode(s: &str) -> Result<String> {
    urlencoding::decode(s)
        .map(Cow::into_owned)
        .context("Failed to URL-decode string")
}

pub async fn process_s3_event_message(sqs_message: &SqsMessage) -> Result<()> {
    let body = sqs_message
        .body
        .as_ref()
        .context("SQS message has no body")?;

    let s3_event: S3Event =
        serde_json::from_str(body).context("Failed to parse S3 event from SQS body")?;

    info!(
        "Processing S3 event with {} records",
        s3_event.records.len()
    );

    for record in s3_event.records {
        let bucket = record
            .s3
            .bucket
            .name
            .as_ref()
            .context("S3 record missing bucket name")?;
        let key_raw = record
            .s3
            .object
            .key
            .as_ref()
            .context("S3 record missing object key")?;

        let key = url_decode(key_raw)?;

        let event_name = record
            .event_name
            .as_ref()
            .context("S3 record missing event name")?;
        let size = record.s3.object.size.unwrap_or(0);

        info!(
            bucket = %bucket,
            key = %key,
            key_raw = %key_raw,
            event = %event_name,
            size = %size,
            "Processing S3 object"
        );

        // Get object metadata from S3 repository
        let (content_type, last_modified) = repository::s3::get_object_metadata(bucket, &key)
            .await
            .context("Failed to get S3 object metadata")?;

        // Map AWS S3 event to domain FileRecord
        let file_record = FileRecord::with_metadata(
            bucket.to_string(),
            key.clone(),
            size,
            content_type,
            Some(last_modified),
        );

        // Determine how many bytes to fetch based on file type
        // For images: 512KB is usually enough for headers and EXIF
        // For videos: 1MB to capture MP4 metadata atoms
        let num_bytes = if let Some(ref ct) = file_record.content_type {
            if ct.starts_with("video/") {
                std::cmp::min(1024 * 1024, size as u64) // 1MB for video
            } else {
                std::cmp::min(512 * 1024, size as u64) // 512KB for images
            }
        } else {
            // Default to 512KB if content type unknown
            std::cmp::min(512 * 1024, size as u64)
        };

        // Fetch head bytes from S3 repository
        let head_bytes = repository::s3::fetch_head_bytes(bucket, &key, num_bytes)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch first {} bytes from S3 for {}/{}",
                    num_bytes, bucket, key
                )
            })?;

        info!(
            "Fetched {} bytes from S3 for metadata extraction",
            head_bytes.len()
        );

        // Extract metadata using pure service layer (no S3 dependencies)
        match METADATA_SERVICE
            .extract_metadata(&head_bytes, &file_record)
            .await
        {
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

        debug!("S3 object processed successfully");
    }

    Ok(())
}
