use anyhow::{Context, Result};
use aws_lambda_events::event::{s3::S3Event, sqs::SqsMessage};
use aws_sdk_s3::Client as S3Client;
use once_cell::sync::Lazy;
use serde_json;
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

use super::metadata::MetadataService;

// Use tokio's OnceCell for async initialization
static S3_CLIENT: OnceCell<Arc<S3Client>> = OnceCell::const_new();
static METADATA_SERVICE: Lazy<MetadataService> = Lazy::new(MetadataService::new);

async fn get_s3_client() -> Arc<S3Client> {
    S3_CLIENT
        .get_or_init(|| async {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Arc::new(S3Client::new(&config))
        })
        .await
        .clone()
}

/// URL-decode a string, handling AWS S3 event encoding
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

        // URL-decode the key - S3 events encode special characters like ~ as %7E
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

        // Extract metadata from the file
        let s3_client = get_s3_client().await;
        match METADATA_SERVICE
            .extract_metadata(&s3_client, bucket, &key, size)
            .await
        {
            Ok(metadata) => {
                // Log the metadata
                info!(
                    bucket = %bucket,
                    key = %key,
                    media_type = ?metadata.media_type,
                    file_size = %metadata.file_size,
                    content_type = ?metadata.content_type,
                    created_date = %metadata.created_date,
                    "Extracted metadata"
                );

                // Log image-specific metadata if available
                if let Some(ref img_meta) = metadata.image_metadata {
                    info!(
                        width = %img_meta.width,
                        height = %img_meta.height,
                        format = %img_meta.format,
                        has_exif = %img_meta.exif.is_some(),
                        "Image metadata"
                    );

                    if let Some(ref exif) = img_meta.exif {
                        info!(
                            make = ?exif.make,
                            model = ?exif.model,
                            date_time_original = ?exif.date_time_original,
                            has_gps = %exif.gps.is_some(),
                            iso = ?exif.iso,
                            exposure_time = ?exif.exposure_time,
                            f_number = ?exif.f_number,
                            "EXIF data"
                        );

                        if let Some(ref gps) = exif.gps {
                            info!(
                                latitude = %gps.latitude,
                                longitude = %gps.longitude,
                                altitude = ?gps.altitude,
                                "GPS coordinates"
                            );
                        }
                    }
                }

                // Log video-specific metadata if available
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

                // Pretty print the full metadata as JSON for easy copying
                match serde_json::to_string(&metadata) {
                    Ok(json) => {
                        info!(metadata = %json, "full_metadata");
                    }
                    Err(e) => {
                        error!("Failed to serialize metadata to JSON: {:?}", e);
                    }
                }

                // TODO: Future DynamoDB integration
                // store_metadata_in_dynamodb(metadata).await?;
            }
            Err(e) => {
                // Log the error but don't fail the message
                // This allows processing to continue even if metadata extraction fails
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
