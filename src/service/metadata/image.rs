use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, NaiveDateTime, Utc};
use exif::{In, Reader, Tag};
use image::ImageReader;
use std::collections::HashMap;

use super::extractor::MetadataExtractor;
use super::types::{ExifData, ImageMetadata, MediaMetadata, MediaType};
use crate::utils::s3;

pub struct ImageMetadataExtractor;

impl ImageMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    fn clean_value(raw: &str) -> String {
        let mut v = raw.trim().to_string();

        if v.starts_with("Some(") && v.ends_with(')') {
            v = v[5..v.len() - 1].to_string();
        }

        if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
            v = v[1..v.len() - 1].to_string();
        }

        v.trim().to_string()
    }

    fn parse_exif(&self, bytes: &[u8]) -> Option<ExifData> {
        let mut cursor = std::io::Cursor::new(bytes);
        let reader = Reader::new().read_from_container(&mut cursor).ok()?;

        let mut exif_data = ExifData {
            date_time_original: None,
            other_fields: HashMap::new(),
        };

        if let Some(field) = reader.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            let date_str = field.display_value().to_string();

            let parsed_date = NaiveDateTime::parse_from_str(&date_str, "%Y:%m:%d %H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S"))
                .ok();

            if let Some(naive_dt) = parsed_date {
                exif_data.date_time_original =
                    Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
                tracing::debug!("Parsed DateTimeOriginal: {} -> {:?}", date_str, naive_dt);
            } else {
                tracing::warn!("Failed to parse DateTimeOriginal: {}", date_str);
            }
        }

        for field in reader.fields() {
            let key = field.tag.to_string(); // human-friendly key from exif crate
            let raw_value = field.display_value().to_string();
            let value = Self::clean_value(&raw_value);
            exif_data.other_fields.insert(key, value);
        }

        Some(exif_data)
    }

}

#[async_trait]
impl MetadataExtractor for ImageMetadataExtractor {
    fn can_handle(&self, extension: &str, content_type: Option<&str>) -> bool {
        let ext = extension.to_lowercase();
        let is_image_ext = matches!(
            ext.as_str(),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "tif"
        );

        let is_image_mime = content_type
            .map(|ct| ct.to_lowercase().starts_with("image/"))
            .unwrap_or(false);

        is_image_ext || is_image_mime
    }

    async fn extract(
        &self,
        s3_client: &S3Client,
        bucket: &str,
        key: &str,
        file_size: i64,
    ) -> Result<MediaMetadata> {
        // Fetch S3 metadata first
        let (content_type, last_modified) = s3::get_object_metadata(s3_client, bucket, key)
            .await
            .context("Failed to get S3 object metadata")?;

        // Create basic metadata
        let mut metadata = MediaMetadata::new_basic(
            bucket.to_string(),
            key.to_string(),
            file_size,
            last_modified,
        );
        metadata.media_type = MediaType::Image;
        metadata.content_type = content_type;

        // Fetch first 512KB for image processing and EXIF data
        let num_bytes = std::cmp::min(512 * 1024, file_size as u64);
        let bytes = s3::fetch_head_bytes(s3_client, bucket, key, num_bytes)
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch first {} bytes from S3 for {}/{}",
                    num_bytes, bucket, key
                )
            })?;

        tracing::debug!("Fetched {} bytes from S3 for image processing", bytes.len());

        // Parse image dimensions and format
        let cursor = s3::bytes_to_cursor(bytes.clone());
        let img_reader = ImageReader::new(cursor)
            .with_guessed_format()
            .with_context(|| format!("Failed to guess image format for {}/{}", bucket, key))?;

        let format = img_reader
            .format()
            .map(|f| format!("{:?}", f))
            .unwrap_or_else(|| "Unknown".to_string());

        tracing::debug!("Detected image format: {}", format);

        let dimensions = img_reader
            .into_dimensions()
            .with_context(|| format!("Failed to read image dimensions for {}/{}", bucket, key))?;

        let (width, height) = dimensions;
        tracing::debug!("Image dimensions: {}x{}", width, height);

        let exif = self.parse_exif(&bytes);

        if let Some(ref exif_data) = exif {
            if let Some(date_time_original) = exif_data.date_time_original {
                metadata.created_date = date_time_original;
            }
        }

        metadata.image_metadata = Some(ImageMetadata {
            width,
            height,
            format,
            exif,
        });

        Ok(metadata)
    }
}