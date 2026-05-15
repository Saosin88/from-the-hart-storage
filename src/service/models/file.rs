use serde::{Deserialize, Serialize};
use std::fmt;

use super::metadata::MediaMetadata;
use crate::utils::time;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct File {
    pub bucket_key: String,
    pub bucket: String,
    pub owner_id: String,
    pub file_id: String,
    pub file_name: String,
    pub file_path: String,
    pub folder_prefix: String,
    pub created_date: i64,
    pub size_bytes: i64,
    pub content_type: String,
    pub media_type: MediaType,
    pub media_metadata: Option<MediaMetadata>,
}

impl File {
    #[must_use]
    pub fn new(bucket_key: String, bucket: String) -> Self {
        Self {
            bucket_key,
            folder_prefix: String::new(),
            bucket,
            owner_id: String::new(),
            file_id: String::new(),
            file_name: String::new(),
            file_path: String::new(),
            created_date: time::now_as_unix_millis(),
            size_bytes: 0,
            content_type: "application/octet-stream".to_string(),
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

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MediaType::Image => write!(f, "Image"),
            MediaType::Video => write!(f, "Video"),
            MediaType::Audio => write!(f, "Audio"),
            MediaType::Document => write!(f, "Document"),
            MediaType::Unknown => write!(f, "Unknown"),
        }
    }
}
