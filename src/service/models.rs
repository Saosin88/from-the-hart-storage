use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
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
    pub bucket: Arc<str>,
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
    pub fn new(bucket_key: String, bucket: String) -> Self {
        Self {
            bucket_key: bucket_key.into(),
            folder_prefix: String::new(),
            bucket: bucket.into(),
            owner_id: String::new(),
            file_id: String::new(),
            file_name: String::new(),
            file_path: String::new(),
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

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MediaType::Image => write!(f, "Image"),
            MediaType::Video => write!(f, "Video"),
            MediaType::Audio => write!(f, "Audio"),
            MediaType::Document => write!(f, "Document"),
            MediaType::Unknown => write!(f, "GreUnknownen"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MediaMetadata {
    Image(ImageMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub exif: Option<HashMap<String, String>>,
    pub gps: Option<GpsCoordinates>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

impl GpsCoordinates {
    pub fn new(latitude: f64, longitude: f64, altitude: Option<f64>) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
        }
    }
}
