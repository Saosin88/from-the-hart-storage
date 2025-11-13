use async_trait::async_trait;
use exif::Reader;
use std::collections::HashMap;

use super::extractor::MetadataExtractor;
use crate::service::models::{GpsCoordinates, ImageMetadata, MediaMetadata, MediaType};
use crate::service::File;
use crate::utils::{gps, string, time};

pub struct ImageMetadataExtractor;

impl ImageMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    fn extract_creation_date(&self, exif_tags: &HashMap<String, String>) -> Option<i64> {
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

                if let Some(timestamp) = time::parse_media_datetime_with_offset(date_str, offset) {
                    return Some(timestamp);
                }
            }
        }

        tracing::debug!("No valid EXIF date tags found");
        None
    }

    fn extract_gps_coordinates(
        &self,
        exif_tags: &HashMap<String, String>,
    ) -> Option<GpsCoordinates> {
        let lat_str = exif_tags.get("GPSLatitude")?;
        let lat_ref = exif_tags.get("GPSLatitudeRef").map(|s| s.as_str());

        let lon_str = exif_tags.get("GPSLongitude")?;
        let lon_ref = exif_tags.get("GPSLongitudeRef").map(|s| s.as_str());

        let alt_str = exif_tags.get("GPSAltitude")?;
        let alt_ref = exif_tags.get("GPSAltitudeRef").map(|s| s.as_str());

        let latitude = gps::parse_coordinate_with_ref(lat_str, lat_ref)?;
        let longitude = gps::parse_coordinate_with_ref(lon_str, lon_ref)?;
        let altitude = gps::parse_altitude_with_ref(alt_str, alt_ref)?;

        tracing::debug!(
            latitude = latitude,
            longitude = longitude,
            altitude = altitude,
            "Extracted GPS coordinates from EXIF tags"
        );

        Some(GpsCoordinates {
            latitude,
            longitude,
            altitude: Some(altitude),
        })
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
        let mut gps_coords = None;

        if let Some(ref tags) = exif_tags {
            if let Some(created_date) = self.extract_creation_date(tags) {
                file.created_date = created_date;
            }
            gps_coords = self.extract_gps_coordinates(tags);
        }

        file.media_metadata = Some(MediaMetadata::Image(ImageMetadata {
            width: width.unwrap_or_default(),
            height: height.unwrap_or_default(),
            exif: exif_tags,
            gps: gps_coords,
        }));
    }
}
