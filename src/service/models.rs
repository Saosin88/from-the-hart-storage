use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::utils::time;

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub uptime: u64,
    pub timestamp: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub bucket_key: Arc<str>,
    pub bucket_prefix: Arc<str>,
    pub bucket: Arc<str>,
    pub file_name: Arc<str>,
    pub created_date: i64, // Unix timestamp in milliseconds
    pub size_bytes: i64,
    pub content_type: String,  // MIME type from S3 (e.g., "image/jpeg")
    pub media_type: MediaType, // High-level category
    pub media_metadata: Option<MediaMetadata>, // Polymorphic metadata
}

impl File {
    pub fn new(
        bucket_key: String,
        bucket_prefix: String,
        bucket: String,
        file_name: String,
    ) -> Self {
        Self {
            bucket_key: bucket_key.into(),
            bucket_prefix: bucket_prefix.into(),
            bucket: bucket.into(),
            file_name: file_name.into(),
            created_date: time::now_as_unix_millis(),
            size_bytes: 0,
            content_type: String::from("application/octet-stream"),
            media_type: MediaType::Unknown,
            media_metadata: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Document,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MediaMetadata {
    Image(ImageMetadata),
    // Future: Video(VideoMetadata), Audio(AudioMetadata), etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub exif: Option<HashMap<String, String>>,
}
