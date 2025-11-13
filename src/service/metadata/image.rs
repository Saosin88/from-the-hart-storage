use async_trait::async_trait;
use exif::Reader;
use std::collections::HashMap;

use super::extractor::MetadataExtractor;
use crate::service::models::{ImageMetadata, MediaMetadata, MediaType};
use crate::service::File;
use crate::utils::{string, time};

pub struct ImageMetadataExtractor;

impl ImageMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    fn extract_creation_date(&self, exif_tags: &HashMap<String, String>) -> Option<i64> {
        // Extract GPS coordinates if available
        let gps_coords = self.extract_gps_coordinates(exif_tags);

        // Priority order for datetime + offset pairs
        let date_offset_pairs = [
            ("DateTimeOriginal", "OffsetTimeOriginal"),
            ("DateTimeDigitized", "OffsetTimeDigitized"),
            ("DateTime", "OffsetTime"),
            ("CreateDate", "OffsetTime"),
            ("ModifyDate", "OffsetTime"),
        ];

        for (date_tag, offset_tag) in &date_offset_pairs {
            if let Some(date_str) = exif_tags.get(*date_tag) {
                let offset = exif_tags.get(*offset_tag).map(|s| s.as_str());

                if let Some(timestamp) =
                    time::parse_media_datetime_with_context(date_str, offset, gps_coords)
                {
                    tracing::debug!(
                        datetime_tag = date_tag,
                        offset_tag = offset_tag,
                        offset = ?offset,
                        gps = ?gps_coords,
                        timestamp = timestamp,
                        "Extracted creation date"
                    );
                    return Some(timestamp);
                }
            }
        }

        tracing::debug!("No valid EXIF date tags found");
        None
    }

    fn extract_gps_coordinates(&self, exif_tags: &HashMap<String, String>) -> Option<(f64, f64)> {
        let lat = exif_tags.get("GPSLatitude")?;
        let lat_ref = exif_tags.get("GPSLatitudeRef")?;
        let lon = exif_tags.get("GPSLongitude")?;
        let lon_ref = exif_tags.get("GPSLongitudeRef")?;

        let latitude = self.parse_gps_coordinate(lat, lat_ref)?;
        let longitude = self.parse_gps_coordinate(lon, lon_ref)?;

        Some((latitude, longitude))
    }

    fn parse_gps_coordinate(&self, coord: &str, reference: &str) -> Option<f64> {
        // GPS coordinates are in format: "deg, min, sec" or "deg"
        // Example: "33, 55, 11.82" or "33.919950"

        // Try decimal format first
        if let Ok(value) = coord.parse::<f64>() {
            let multiplier = if reference == "S" || reference == "W" {
                -1.0
            } else {
                1.0
            };
            return Some(value * multiplier);
        }

        // Try DMS format: "degrees, minutes, seconds"
        let parts: Vec<&str> = coord.split(',').map(|s| s.trim()).collect();

        let value = match parts.len() {
            3 => {
                let deg: f64 = parts[0].parse().ok()?;
                let min: f64 = parts[1].parse().ok()?;
                let sec: f64 = parts[2].parse().ok()?;
                deg + min / 60.0 + sec / 3600.0
            }
            1 => parts[0].parse().ok()?,
            _ => return None,
        };

        let multiplier = if reference == "S" || reference == "W" {
            -1.0
        } else {
            1.0
        };
        Some(value * multiplier)
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

        let (width, height) = match imagesize::blob_size(head_bytes) {
            Ok(size) => {
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

        file.media_metadata = Some(MediaMetadata::Image(ImageMetadata {
            width: width.unwrap_or_default(),
            height: height.unwrap_or_default(),
            exif: exif_tags,
        }));
    }
}
