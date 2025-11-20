use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::service::file::utils::{get_folder_name, get_parent_folder_path};
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
    pub owner_id: Arc<str>,
    pub file_id: Arc<str>,
    pub file_name: Arc<str>,
    pub file_path: Arc<str>,
    pub folder_prefix: Arc<str>,
    pub created_date: i64,
    pub size_bytes: i64,
    pub content_type: Arc<str>,
    pub media_type: MediaType,
    pub media_metadata: Option<MediaMetadata>,
}

impl File {
    pub fn new(bucket_key: String, bucket: String) -> Self {
        Self {
            bucket_key: bucket_key.into(),
            folder_prefix: "".into(),
            bucket: bucket.into(),
            owner_id: "".into(),
            file_id: "".into(),
            file_name: "".into(),
            file_path: "".into(),
            created_date: time::now_as_unix_millis(),
            size_bytes: 0,
            content_type: "application/octet-stream".into(),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewLink {
    pub viewer_id: Arc<str>,
    pub resource_id: Arc<str>,
    pub owner_id: Arc<str>,
    pub grant_id: Arc<str>,
    pub created_date: i64,
    pub folder_prefix: Arc<str>,
    pub name: Arc<str>,
    pub media_type: Arc<str>,
    pub size_bytes: i64,
    pub is_folder: bool,
}

impl ViewLink {
    pub fn for_owner(file: &File) -> Self {
        Self {
            viewer_id: file.owner_id.clone(),
            resource_id: file.file_id.clone(),
            owner_id: file.owner_id.clone(),
            grant_id: "OWNER".into(),
            created_date: file.created_date,
            folder_prefix: file.folder_prefix.clone(),
            name: file.file_name.clone(),
            media_type: file.media_type.to_string().into(),
            size_bytes: file.size_bytes,
            is_folder: false,
        }
    }

    pub fn for_owner_folder(file: &File, full_folder_path: &str) -> Self {
        Self {
            viewer_id: file.owner_id.clone(),
            resource_id: full_folder_path.into(),
            owner_id: file.owner_id.clone(),
            grant_id: "OWNER".into(),
            created_date: file.created_date,
            folder_prefix: get_parent_folder_path(full_folder_path).into(),
            name: get_folder_name(full_folder_path).into(),
            media_type: "Folder".into(),
            size_bytes: 0,
            is_folder: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareGrant {
    pub item_type: Option<String>,
    pub grant_id: String,
    pub owner_id: String,
    pub recipient_id: String,
    pub grant_type: Option<String>,
    pub prefix: Option<String>,
    pub resource_id: Option<String>,
    pub file_path: Option<String>,
    pub created_date: Option<i64>,
}
