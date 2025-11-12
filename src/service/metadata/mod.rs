mod extractor;
mod image;
mod types;
mod video;

use anyhow::{Context, Result};
use std::path::Path;

pub use types::{ExifData, GpsData, ImageMetadata, MediaMetadata, MediaType, VideoMetadata};

use crate::service::FileRecord;
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

    /// Extract metadata from file bytes and file record
    /// Will automatically detect the file type and use the appropriate extractor
    pub async fn extract_metadata(
        &self,
        head_bytes: &[u8],
        file_record: &FileRecord,
    ) -> Result<MediaMetadata> {
        // Get file extension
        let extension = Path::new(&file_record.file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        // Find appropriate extractor
        for extractor in &self.extractors {
            if extractor.can_handle(extension, file_record.content_type.as_deref()) {
                return extractor
                    .extract(head_bytes, file_record)
                    .await
                    .context("Failed to extract metadata with matched extractor");
            }
        }

        // No extractor found - return basic metadata
        let mut metadata = MediaMetadata::new_basic(
            file_record.bucket.clone(),
            file_record.file_name.clone(),
            file_record.file_size,
            file_record.last_modified.unwrap_or_else(chrono::Utc::now),
        );
        metadata.content_type = file_record.content_type.clone();

        Ok(metadata)
    }
}

impl Default for MetadataService {
    fn default() -> Self {
        Self::new()
    }
}
