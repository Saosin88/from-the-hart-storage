use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, NaiveDateTime, Utc};
use exif::{In, Reader, Tag};
use image::ImageReader;
use std::collections::HashMap;

use super::extractor::MetadataExtractor;
use super::types::{ExifData, GpsData, ImageMetadata, MediaMetadata, MediaType};
use crate::utils::s3;

pub struct ImageMetadataExtractor;

impl ImageMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Parse EXIF data from raw bytes
    fn parse_exif(&self, bytes: &[u8]) -> Option<ExifData> {
        let mut cursor = std::io::Cursor::new(bytes);
        let reader = Reader::new().read_from_container(&mut cursor).ok()?;

        let mut exif_data = ExifData {
            make: None,
            model: None,
            date_time_original: None,
            gps: None,
            orientation: None,
            iso: None,
            exposure_time: None,
            f_number: None,
            focal_length: None,
            flash: None,
            other_fields: HashMap::new(),
        };

        // Extract common EXIF fields
        if let Some(field) = reader.get_field(Tag::Make, In::PRIMARY) {
            exif_data.make = Some(field.display_value().to_string());
        }

        if let Some(field) = reader.get_field(Tag::Model, In::PRIMARY) {
            exif_data.model = Some(field.display_value().to_string());
        }

        if let Some(field) = reader.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            let date_str = field.display_value().to_string();
            // EXIF date format: "YYYY:MM:DD HH:MM:SS"
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&date_str, "%Y:%m:%d %H:%M:%S") {
                exif_data.date_time_original =
                    Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }

        if let Some(field) = reader.get_field(Tag::Orientation, In::PRIMARY) {
            if let Some(val) = field.value.get_uint(0) {
                exif_data.orientation = Some(val);
            }
        }

        if let Some(field) = reader.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
            if let Some(val) = field.value.get_uint(0) {
                exif_data.iso = Some(val);
            }
        }

        if let Some(field) = reader.get_field(Tag::ExposureTime, In::PRIMARY) {
            exif_data.exposure_time = Some(field.display_value().to_string());
        }

        if let Some(field) = reader.get_field(Tag::FNumber, In::PRIMARY) {
            exif_data.f_number = Some(field.display_value().to_string());
        }

        if let Some(field) = reader.get_field(Tag::FocalLength, In::PRIMARY) {
            exif_data.focal_length = Some(field.display_value().to_string());
        }

        if let Some(field) = reader.get_field(Tag::Flash, In::PRIMARY) {
            exif_data.flash = Some(field.display_value().to_string());
        }

        // Extract GPS data
        let gps_lat = reader.get_field(Tag::GPSLatitude, In::PRIMARY);
        let gps_lat_ref = reader.get_field(Tag::GPSLatitudeRef, In::PRIMARY);
        let gps_lon = reader.get_field(Tag::GPSLongitude, In::PRIMARY);
        let gps_lon_ref = reader.get_field(Tag::GPSLongitudeRef, In::PRIMARY);
        let gps_alt = reader.get_field(Tag::GPSAltitude, In::PRIMARY);

        if let (Some(lat), Some(lat_ref), Some(lon), Some(lon_ref)) =
            (gps_lat, gps_lat_ref, gps_lon, gps_lon_ref)
        {
            if let (Some(lat_vals), Some(lon_vals)) = (
                self.parse_gps_coordinate(&lat.value),
                self.parse_gps_coordinate(&lon.value),
            ) {
                let lat_sign = if lat_ref.display_value().to_string() == "S" {
                    -1.0
                } else {
                    1.0
                };
                let lon_sign = if lon_ref.display_value().to_string() == "W" {
                    -1.0
                } else {
                    1.0
                };

                let latitude = lat_sign * (lat_vals.0 + lat_vals.1 / 60.0 + lat_vals.2 / 3600.0);
                let longitude = lon_sign * (lon_vals.0 + lon_vals.1 / 60.0 + lon_vals.2 / 3600.0);

                let altitude = gps_alt.and_then(|alt| {
                    if let exif::Value::Rational(ref rationals) = alt.value {
                        rationals.first().map(|r| r.to_f64())
                    } else {
                        None
                    }
                });

                exif_data.gps = Some(GpsData {
                    latitude,
                    longitude,
                    altitude,
                });
            }
        }

        // Store other fields in a catch-all map
        for field in reader.fields() {
            let tag_name = format!("{:?}", field.tag);
            let value = field.display_value().to_string();
            exif_data.other_fields.entry(tag_name).or_insert(value);
        }

        Some(exif_data)
    }

    /// Parse GPS coordinate from EXIF rational values
    /// Returns (degrees, minutes, seconds)
    fn parse_gps_coordinate(&self, value: &exif::Value) -> Option<(f64, f64, f64)> {
        if let exif::Value::Rational(ref rationals) = value {
            if rationals.len() >= 3 {
                let degrees = rationals[0].to_f64();
                let minutes = rationals[1].to_f64();
                let seconds = rationals[2].to_f64();
                return Some((degrees, minutes, seconds));
            }
        }
        None
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
        // This is usually enough for headers and EXIF without downloading full file
        let num_bytes = std::cmp::min(512 * 1024, file_size as u64);
        let bytes = s3::fetch_head_bytes(s3_client, bucket, key, num_bytes)
            .await
            .with_context(|| format!("Failed to fetch first {} bytes from S3 for {}/{}", num_bytes, bucket, key))?;

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

        let dimensions = img_reader.into_dimensions()
            .with_context(|| format!("Failed to read image dimensions for {}/{}", bucket, key))?;

        let (width, height) = dimensions;
        tracing::debug!("Image dimensions: {}x{}", width, height);

        // Parse EXIF data
        let exif = self.parse_exif(&bytes);

        // If EXIF has DateTimeOriginal, use that as created_date
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
