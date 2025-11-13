use async_trait::async_trait;
use chrono::{DateTime, Utc};
use exif::Reader;
use std::collections::HashMap;

use super::extractor::MetadataExtractor;
use crate::service::models::{ImageMetadata, MediaType};
use crate::service::File;
use crate::utils::{string, time};

pub struct ImageMetadataExtractor;

impl ImageMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    fn extract_creation_date(&self, exif_tags: &HashMap<String, String>) -> Option<DateTime<Utc>> {
        let date_tag_priority = [
            "DateTimeOriginal",
            "DateTimeDigitized",
            "DateTime",
            "CreateDate",
            "ModifyDate",
        ];

        for tag_name in &date_tag_priority {
            if let Some(date_str) = exif_tags.get(*tag_name) {
                if let Some(parsed) = time::parse_media_datetime(date_str) {
                    return Some(parsed);
                }
            }
        }

        tracing::debug!("No valid EXIF date tags found");
        None
    }

    fn parse_exif(&self, bytes: &[u8]) -> Option<HashMap<String, String>> {
        tracing::debug!(buf_len = bytes.len(), "parse_exif: starting");

        let mut cursor = std::io::Cursor::new(bytes);

        match Reader::new().read_from_container(&mut cursor) {
            Ok(reader) => {
                let mut exif_tags: HashMap<String, String> = HashMap::new();

                for field in reader.fields() {
                    let key = field.tag.to_string();
                    let raw_value = field.display_value().to_string();
                    let value = string::clean_value(&raw_value);
                    exif_tags.insert(key, value);
                }

                Some(exif_tags)
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
        match content_type {
            Some(ct) => ct.to_lowercase().starts_with("image/"),
            None => {
                let ext = extension.to_lowercase();
                matches!(
                    ext.as_str(),
                    "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "tif"
                )
            }
        }
    }

    async fn extract_and_add_to_file(&self, head_bytes: &[u8], file: &mut File) {
        tracing::debug!("Processing {} bytes for image metadata", head_bytes.len());

        file.media_type = MediaType::Image;

        let format = match imagesize::image_type(head_bytes) {
            Ok(img_type) => format!("{:?}", img_type),
            Err(_) => "Unknown".to_string(),
        };

        let (width, height) = match imagesize::blob_size(head_bytes) {
            Ok(size) => {
                tracing::debug!("Detected image format: {}", format);
                tracing::debug!("Image dimensions: {}x{}", size.width, size.height);
                (Some(size.width as u32), Some(size.height as u32))
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to read image size for {}/{}: {}",
                    file.bucket,
                    file.file_name,
                    e
                );
                (None, None)
            }
        };

        let exif_tags = self.parse_exif(head_bytes);

        if let Some(ref tags) = exif_tags {
            if let Some(created_date) = self.extract_creation_date(tags) {
                file.created_date = created_date;
            }
        }

        file.image_metadata = Some(ImageMetadata {
            width: width.unwrap_or_default(),
            height: height.unwrap_or_default(),
            format,
            exif_tags: exif_tags.unwrap_or_default(),
        });
    }
}
