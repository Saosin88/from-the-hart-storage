use crate::service::models::HealthStatus;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DataResponse<T> {
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Health check data containing service status and uptime information")]
pub struct HealthData {
    #[schemars(
        description = "Current status of the service (e.g., 'ok', 'degraded', 'unhealthy')"
    )]
    pub status: String,
    #[schemars(description = "Service uptime in seconds")]
    pub uptime: u64,
    #[schemars(description = "Current timestamp in milliseconds since UNIX epoch")]
    pub timestamp: u128,
}

pub type HealthResponse = DataResponse<HealthData>;

impl From<HealthStatus> for HealthResponse {
    fn from(status: HealthStatus) -> Self {
        Self {
            data: HealthData {
                status: "ok".to_string(),
                uptime: status.uptime,
                timestamp: status.timestamp,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "A link to a file or folder")]
pub struct ViewLink {
    pub viewer_id: String,
    pub resource_id: String,
    pub owner_id: String,
    pub grant_id: String,
    pub created_date: i64,
    pub folder_prefix: String,
    pub name: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub is_folder: bool,
}

impl From<crate::service::models::ViewLink> for ViewLink {
    fn from(model: crate::service::models::ViewLink) -> Self {
        Self {
            viewer_id: model.viewer_id.to_string(),
            resource_id: model.resource_id.to_string(),
            owner_id: model.owner_id.to_string(),
            grant_id: model.grant_id.to_string(),
            created_date: model.created_date,
            folder_prefix: model.folder_prefix.to_string(),
            name: model.name.to_string(),
            media_type: model.media_type.to_string(),
            size_bytes: model.size_bytes,
            is_folder: model.is_folder,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "List of files and folders")]
pub struct StorageListData {
    #[schemars(description = "List of files and folders")]
    pub items: Vec<ViewLink>,
    #[schemars(description = "Cursor for pagination")]
    pub next_cursor: Option<String>,
}

pub type StorageListResponse = DataResponse<StorageListData>;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Detailed file information")]
pub struct FileData {
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
    pub media_type: String,
    pub media_metadata: Option<serde_json::Value>,
}

pub type FileResponse = DataResponse<FileData>;

impl From<crate::service::models::File> for FileResponse {
    fn from(model: crate::service::models::File) -> Self {
        let media_metadata = model.media_metadata.as_ref().and_then(|m| {
            serde_json::to_value(m).ok()
        });

        Self {
            data: FileData {
                bucket_key: model.bucket_key.to_string(),
                bucket: model.bucket.to_string(),
                owner_id: model.owner_id.to_string(),
                file_id: model.file_id.to_string(),
                file_name: model.file_name.to_string(),
                file_path: model.file_path.to_string(),
                folder_prefix: model.folder_prefix.to_string(),
                created_date: model.created_date,
                size_bytes: model.size_bytes,
                content_type: model.content_type.to_string(),
                media_type: model.media_type.to_string(),
                media_metadata,
            },
        }
    }
}

