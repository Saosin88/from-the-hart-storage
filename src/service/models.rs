use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub bucket_key: String,
    pub bucket_prefix: String,
    pub bucket: String,
    pub file_name: String,
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
            bucket_key,
            bucket_prefix,
            bucket,
            file_name,
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
