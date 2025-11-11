mod extractor;
mod image;
mod types;
mod video;

use anyhow::{Context, Result};
use aws_sdk_s3::Client as S3Client;
use std::path::Path;

pub use types::{ExifData, GpsData, ImageMetadata, MediaMetadata, MediaType, VideoMetadata};

use extractor::MetadataExtractor;
use image::ImageMetadataExtractor;
use video::VideoMetadataExtractor;

/// Main orchestrator for metadata extraction
pub struct MetadataService {
    extractors: Vec<Box<dyn MetadataExtractor>>,
}

impl MetadataService {
    /// Create a new metadata service with all extractors
    pub fn new() -> Self {
        let extractors: Vec<Box<dyn MetadataExtractor>> = vec![
            Box::new(ImageMetadataExtractor::new()),
            Box::new(VideoMetadataExtractor::new()),
        ];

        Self { extractors }
    }

    /// Extract metadata from an S3 object
    /// Will automatically detect the file type and use the appropriate extractor
    pub async fn extract_metadata(
        &self,
        s3_client: &S3Client,
        bucket: &str,
        key: &str,
        file_size: i64,
    ) -> Result<MediaMetadata> {
        // Get file extension
        let extension = Path::new(key)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        // Get content type from S3
        let content_type = match crate::utils::s3::get_object_metadata(s3_client, bucket, key).await
        {
            Ok((ct, _)) => ct,
            Err(_) => None,
        };

        // Find appropriate extractor
        for extractor in &self.extractors {
            if extractor.can_handle(extension, content_type.as_deref()) {
                return extractor
                    .extract(s3_client, bucket, key, file_size)
                    .await
                    .context("Failed to extract metadata with matched extractor");
            }
        }

        // No extractor found - return basic metadata
        let (content_type, last_modified) =
            crate::utils::s3::get_object_metadata(s3_client, bucket, key)
                .await
                .context("Failed to get S3 object metadata")?;

        let mut metadata = MediaMetadata::new_basic(
            bucket.to_string(),
            key.to_string(),
            file_size,
            last_modified,
        );
        metadata.content_type = content_type;

        Ok(metadata)
    }
}

impl Default for MetadataService {
    fn default() -> Self {
        Self::new()
    }
}
