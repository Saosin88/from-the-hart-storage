use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::service::file::utils::{get_folder_name, get_parent_folder_path};
use crate::utils::time;

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub uptime: u64,
    pub timestamp: u128,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MediaMetadata {
    Image(ImageMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub exif: Option<HashMap<String, String>>,
    pub gps: Option<GpsCoordinates>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ResourceId {
    File(String),
    Folder(String),
}

impl ResourceId {
    pub fn is_folder(&self) -> bool {
        matches!(self, ResourceId::Folder(_))
    }

    pub fn as_str(&self) -> &str {
        match self {
            ResourceId::File(s) | ResourceId::Folder(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewLink {
    pub viewer_id: String,
    pub resource_id: ResourceId,
    pub owner_id: String,
    pub grant_id: String,
    pub created_date: i64,
    pub folder_prefix: String,
    pub name: String,
    pub media_type: String,
    pub size_bytes: i64,
}

impl ViewLink {
    pub fn is_folder(&self) -> bool {
        self.resource_id.is_folder()
    }

    pub fn resource_id_str(&self) -> &str {
        self.resource_id.as_str()
    }

    pub fn for_owner(file: &File) -> Self {
        Self {
            viewer_id: file.owner_id.clone(),
            resource_id: ResourceId::File(file.file_id.clone()),
            owner_id: file.owner_id.clone(),
            grant_id: "OWNER".to_string(),
            created_date: file.created_date,
            folder_prefix: file.folder_prefix.clone(),
            name: file.file_name.clone(),
            media_type: file.media_type.to_string(),
            size_bytes: file.size_bytes,
        }
    }

    pub fn for_owner_folder(file: &File, full_folder_path: &str) -> Self {
        Self {
            viewer_id: file.owner_id.clone(),
            resource_id: ResourceId::Folder(full_folder_path.to_string()),
            owner_id: file.owner_id.clone(),
            grant_id: "OWNER".to_string(),
            created_date: file.created_date,
            folder_prefix: get_parent_folder_path(full_folder_path),
            name: get_folder_name(full_folder_path),
            media_type: "Folder".to_string(),
            size_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_id_enum_serde_roundtrip() {
        // File variant
        let file_id = ResourceId::File("abc123".to_string());
        let json = serde_json::to_value(&file_id).unwrap();
        assert_eq!(json, serde_json::json!({"type": "File", "value": "abc123"}));
        let deserialized: ResourceId = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, file_id);

        // Folder variant
        let folder_id = ResourceId::Folder("media/photos/".to_string());
        let json = serde_json::to_value(&folder_id).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "Folder", "value": "media/photos/"})
        );
        let deserialized: ResourceId = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, folder_id);
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
