use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
    pub created_date: DateTime<Utc>,
    pub media_type: MediaType,
    pub content_type: String,
    pub content_length: i64,
    pub image_metadata: Option<ImageMetadata>,
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
            created_date: Utc::now(),
            media_type: MediaType::Unknown,
            content_type: String::from("application/octet-stream"),
            content_length: 0,
            image_metadata: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub exif_tags: HashMap<String, String>,
}
