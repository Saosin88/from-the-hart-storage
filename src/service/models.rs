use aws_sdk_dynamodb::types::AttributeValue;
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

    pub fn to_dynamo_item(&self) -> HashMap<String, AttributeValue> {
        let mut item = HashMap::new();
        item.insert(
            "PK".to_string(),
            AttributeValue::S(format!("USER#{}", self.owner_id)),
        );
        item.insert(
            "SK".to_string(),
            AttributeValue::S(format!("FILE#{}", self.file_path)),
        );
        item.insert(
            "item_type".to_string(),
            AttributeValue::S("FILE".to_string()),
        );
        item.insert(
            "owner_id".to_string(),
            AttributeValue::S(self.owner_id.to_string()),
        );
        item.insert(
            "resource_id".to_string(),
            AttributeValue::S(self.file_id.to_string()),
        );
        item.insert(
            "file_name".to_string(),
            AttributeValue::S(self.file_name.to_string()),
        );
        item.insert(
            "file_path".to_string(),
            AttributeValue::S(self.file_path.to_string()),
        );
        item.insert(
            "folder_prefix".to_string(),
            AttributeValue::S(self.folder_prefix.to_string()),
        );
        item.insert(
            "media_type".to_string(),
            AttributeValue::S(self.media_type.to_string()),
        );
        item.insert(
            "content_type".to_string(),
            AttributeValue::S(self.content_type.to_string()),
        );
        item.insert(
            "size_bytes".to_string(),
            AttributeValue::N(self.size_bytes.to_string()),
        );
        item.insert(
            "created_date".to_string(),
            AttributeValue::N(self.created_date.to_string()),
        );
        item.insert(
            "bucket_key".to_string(),
            AttributeValue::S(self.bucket_key.to_string()),
        );
        item.insert(
            "bucket".to_string(),
            AttributeValue::S(self.bucket.to_string()),
        );

        if let Some(metadata) = &self.media_metadata {
            if let Ok(meta_json) = serde_json::to_string(metadata) {
                item.insert("MediaMetadata".to_string(), AttributeValue::S(meta_json));
            }
        }

        item
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
    pub file_name: Arc<str>,
    pub media_type: Arc<str>,
    pub size_bytes: i64,
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
            file_name: file.file_name.clone(),
            media_type: file.media_type.to_string().into(),
            size_bytes: file.size_bytes,
        }
    }

    pub fn to_dynamo_item(&self) -> HashMap<String, AttributeValue> {
        let mut item = HashMap::new();
        item.insert(
            "PK".to_string(),
            AttributeValue::S(format!("USER#{}", self.viewer_id)),
        );
        item.insert(
            "SK".to_string(),
            AttributeValue::S(format!(
                "VIEWLINK#{}#{}",
                self.owner_id, self.resource_id
            )),
        );
        item.insert(
            "resource_id".to_string(),
            AttributeValue::S(self.resource_id.to_string()),
        );
        item.insert(
            "owner_id".to_string(),
            AttributeValue::S(self.owner_id.to_string()),
        );
        item.insert(
            "grant_id".to_string(),
            AttributeValue::S(self.grant_id.to_string()),
        );
        item.insert(
            "created_date".to_string(),
            AttributeValue::N(self.created_date.to_string()),
        );
        item.insert(
            "folder_prefix".to_string(),
            AttributeValue::S(self.folder_prefix.to_string()),
        );
        item.insert(
            "file_name".to_string(),
            AttributeValue::S(self.file_name.to_string()),
        );
        item.insert(
            "media_type".to_string(),
            AttributeValue::S(self.media_type.to_string()),
        );
        item.insert(
            "size_bytes".to_string(),
            AttributeValue::N(self.size_bytes.to_string()),
        );

        item.insert(
            "GSI2-PK".to_string(),
            AttributeValue::S(format!(
                "VIEWER#{}#FOLDER#{}",
                self.viewer_id, self.folder_prefix
            )),
        );
        item.insert(
            "GSI2-SK".to_string(),
            AttributeValue::S(format!(
                "TYPE#FILE#{}#{}#{}",
                self.created_date, self.media_type, self.resource_id
            )),
        );

        item
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
