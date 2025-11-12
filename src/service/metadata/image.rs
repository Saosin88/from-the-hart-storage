use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use exif::{In, Reader, Tag};
use image::ImageReader;
use std::collections::HashMap;

use super::extractor::MetadataExtractor;
use super::types::{ExifData, ImageMetadata, MediaMetadata, MediaType};
use crate::service::FileRecord;

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

    fn find_exif_marker(bytes: &[u8]) -> Option<usize> {
        if bytes.len() < 6 {
            return None;
        }

        // look for ASCII "Exif\0\0" using std library (no extra deps)
        if let Some(pos) = bytes
            .windows(6)
            .position(|w| w == b"Exif\0\0")
        {
            // position is at the 'E' of "Exif\0\0"; return approximate start of APP1 marker
            return Some(pos.saturating_sub(6));
        }

        // scan for JPEG APP1 marker (0xFF 0xE1)
        for i in 0..bytes.len().saturating_sub(1) {
            if bytes[i] == 0xFF && bytes[i + 1] == 0xE1 {
                return Some(i);
            }
        }

        None
    }

    fn parse_exif(&self, bytes: &[u8]) -> Option<ExifData> {
        tracing::debug!(buf_len = bytes.len(), "parse_exif: starting");

        if let Some(pos) = Self::find_exif_marker(bytes) {
            tracing::debug!(exif_marker_offset = pos, "EXIF marker detected in buffer");
        } else {
            tracing::debug!("No EXIF marker detected in head bytes");
        }

        // Use cursor so Reader can Seek/Read
        let mut cursor = std::io::Cursor::new(bytes);

        // Attempt parse and log parse error if any
        match Reader::new().read_from_container(&mut cursor) {
            Ok(reader) => {
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
                            Some(DateTime::<Utc>::from_utc(naive_dt, Utc));
                        tracing::debug!(date = %date_str, "Parsed DateTimeOriginal");
                    } else {
                        tracing::warn!(date = %date_str, "Could not parse DateTimeOriginal");
                    }
                }

                for field in reader.fields() {
                    let key = field.tag.to_string();
                    let raw_value = field.display_value().to_string();
                    let value = Self::clean_value(&raw_value);
                    exif_data.other_fields.insert(key, value);
                }

                Some(exif_data)
            }
            Err(e) => {
                tracing::debug!(error = %e, "exif::Reader failed to parse container");
                None
            }
        }
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
        head_bytes: &[u8],
        file_record: &FileRecord,
    ) -> Result<MediaMetadata> {
        // Create basic metadata
        let mut metadata = MediaMetadata::new_basic(
            file_record.bucket.clone(),
            file_record.file_name.clone(),
            file_record.file_size,
            file_record.last_modified.unwrap_or_else(Utc::now),
        );
        metadata.media_type = MediaType::Image;
        metadata.content_type = file_record.content_type.clone();

        tracing::debug!("Processing {} bytes for image metadata", head_bytes.len());

        // Parse image dimensions and format
        let cursor = std::io::Cursor::new(head_bytes);
        let img_reader = ImageReader::new(cursor)
            .with_guessed_format()
            .with_context(|| format!("Failed to guess image format for {}/{}", file_record.bucket, file_record.file_name))?;

        let format = img_reader
            .format()
            .map(|f| format!("{:?}", f))
            .unwrap_or_else(|| "Unknown".to_string());

        tracing::debug!("Detected image format: {}", format);

        let dimensions = img_reader
            .into_dimensions()
            .with_context(|| format!("Failed to read image dimensions for {}/{}", file_record.bucket, file_record.file_name))?;

        let (width, height) = dimensions;
        tracing::debug!("Image dimensions: {}x{}", width, height);

        let exif = self.parse_exif(head_bytes);

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