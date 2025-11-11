use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the type of media file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
    Unknown,
}

/// Container for all metadata extracted from a media file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    /// Type of media (image, video, unknown)
    pub media_type: MediaType,

    /// File size in bytes (from S3)
    pub file_size: i64,

    /// S3 bucket name
    pub bucket: String,

    /// S3 object key
    pub key: String,

    /// Content type/MIME type
    pub content_type: Option<String>,

    /// Date the file was created/captured
    /// Falls back to S3 LastModified if not available from metadata
    pub created_date: DateTime<Utc>,

    /// Image-specific metadata (dimensions, EXIF, etc.)
    pub image_metadata: Option<ImageMetadata>,

    /// Video-specific metadata (duration, codec, etc.)
    pub video_metadata: Option<VideoMetadata>,
}

/// Image-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    /// Image width in pixels
    pub width: u32,

    /// Image height in pixels
    pub height: u32,

    /// Image format (JPEG, PNG, etc.)
    pub format: String,

    /// EXIF data extracted from image
    pub exif: Option<ExifData>,
}

/// EXIF data from images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    /// Camera make
    pub make: Option<String>,

    /// Camera model
    pub model: Option<String>,

    /// Date/time original (when photo was taken)
    pub date_time_original: Option<DateTime<Utc>>,

    /// GPS coordinates
    pub gps: Option<GpsData>,

    /// Orientation (how the camera was held)
    pub orientation: Option<u32>,

    /// ISO speed
    pub iso: Option<u32>,

    /// Exposure time (shutter speed)
    pub exposure_time: Option<String>,

    /// F-number (aperture)
    pub f_number: Option<String>,

    /// Focal length
    pub focal_length: Option<String>,

    /// Flash setting
    pub flash: Option<String>,

    /// Additional EXIF fields not explicitly mapped
    pub other_fields: HashMap<String, String>,
}

/// GPS coordinates from EXIF data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsData {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

/// Video-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    /// Video width in pixels
    pub width: Option<u32>,

    /// Video height in pixels
    pub height: Option<u32>,

    /// Duration in seconds
    pub duration: Option<f64>,

    /// Video codec
    pub codec: Option<String>,

    /// Frame rate
    pub frame_rate: Option<f64>,

    /// Bitrate
    pub bitrate: Option<u64>,
}

impl MediaMetadata {
    /// Helper to create a basic metadata struct with minimal info
    pub fn new_basic(
        bucket: String,
        key: String,
        file_size: i64,
        last_modified: DateTime<Utc>,
    ) -> Self {
        Self {
            media_type: MediaType::Unknown,
            file_size,
            bucket,
            key,
            content_type: None,
            created_date: last_modified,
            image_metadata: None,
            video_metadata: None,
        }
    }
}
