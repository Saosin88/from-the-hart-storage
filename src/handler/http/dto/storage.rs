use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::DataResponse;

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
        let (resource_id, is_folder) = match model.resource_id {
            crate::service::models::ResourceId::File(id) => (id, false),
            crate::service::models::ResourceId::Folder(path) => (format!("FOLDER#{}", path), true),
        };
        Self {
            viewer_id: model.viewer_id,
            resource_id,
            owner_id: model.owner_id,
            grant_id: model.grant_id,
            created_date: model.created_date,
            folder_prefix: model.folder_prefix,
            name: model.name,
            media_type: model.media_type,
            size_bytes: model.size_bytes,
            is_folder,
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
    pub file_url: String,
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

impl FileResponse {
    #[must_use] 
    pub fn from_file(model: crate::service::models::File, cloudfront_domain: &str) -> Self {
        let file_url = format!("https://{}/{}", cloudfront_domain, model.bucket_key);

        let media_metadata = model
            .media_metadata
            .as_ref()
            .and_then(|m| serde_json::to_value(m).ok());

        Self {
            data: FileData {
                file_url,
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

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request body for creating a new folder")]
pub struct CreateFolderRequest {
    #[schemars(
        description = "Folder path relative to user root, must end with '/' (e.g., 'media/')"
    )]
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Folder metadata")]
pub struct FolderData {
    #[schemars(description = "Full path of the folder")]
    pub folder_path: String,
    #[schemars(description = "Name of the folder")]
    pub folder_name: String,
    #[schemars(description = "Path of the parent folder")]
    pub parent_path: String,
    #[schemars(description = "Folder creation timestamp in milliseconds since UNIX epoch")]
    pub created_date: i64,
    #[schemars(description = "Owner user ID")]
    pub owner_id: String,
}

pub type CreateFolderResponse = DataResponse<FolderData>;

impl From<crate::service::models::ViewLink> for FolderData {
    fn from(model: crate::service::models::ViewLink) -> Self {
        let folder_path = match model.resource_id {
            crate::service::models::ResourceId::Folder(path) => {
                format!("FOLDER#{}", path)
            }
            crate::service::models::ResourceId::File(id) => id,
        };
        Self {
            folder_path,
            folder_name: model.name,
            parent_path: model.folder_prefix,
            created_date: model.created_date,
            owner_id: model.owner_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::models::{
        File, ImageMetadata, MediaMetadata, MediaType, ResourceId, ViewLink as DomainViewLink,
    };

    #[test]
    fn test_view_link_file_to_dto() {
        let domain = DomainViewLink {
            viewer_id: "viewer1".into(),
            resource_id: ResourceId::File("file-abc-123".to_string()),
            owner_id: "owner1".into(),
            grant_id: "OWNER".into(),
            created_date: 1700000000000,
            folder_prefix: "photos/".into(),
            name: "photo.jpg".into(),
            media_type: "Image".into(),
            size_bytes: 1048576,
        };

        let dto: ViewLink = ViewLink::from(domain);

        assert_eq!(dto.viewer_id, "viewer1");
        assert_eq!(dto.resource_id, "file-abc-123");
        assert_eq!(dto.owner_id, "owner1");
        assert_eq!(dto.grant_id, "OWNER");
        assert_eq!(dto.created_date, 1700000000000);
        assert_eq!(dto.folder_prefix, "photos/");
        assert_eq!(dto.name, "photo.jpg");
        assert_eq!(dto.media_type, "Image");
        assert_eq!(dto.size_bytes, 1048576);
        assert!(!dto.is_folder);
    }

    #[test]
    fn test_view_link_folder_to_dto() {
        let domain = DomainViewLink {
            viewer_id: "viewer1".into(),
            resource_id: ResourceId::Folder("media/photos/".to_string()),
            owner_id: "owner1".into(),
            grant_id: "OWNER".into(),
            created_date: 1700000000000,
            folder_prefix: "media/".into(),
            name: "photos".into(),
            media_type: "Folder".into(),
            size_bytes: 0,
        };

        let dto: ViewLink = ViewLink::from(domain);

        assert_eq!(dto.viewer_id, "viewer1");
        assert_eq!(dto.resource_id, "FOLDER#media/photos/");
        assert_eq!(dto.owner_id, "owner1");
        assert_eq!(dto.created_date, 1700000000000);
        assert_eq!(dto.folder_prefix, "media/");
        assert_eq!(dto.name, "photos");
        assert_eq!(dto.media_type, "Folder");
        assert!(dto.is_folder);
    }

    #[test]
    fn test_view_link_to_folder_data() {
        let domain = DomainViewLink {
            viewer_id: "viewer1".into(),
            resource_id: ResourceId::Folder("media/photos/".to_string()),
            owner_id: "owner1".into(),
            grant_id: "OWNER".into(),
            created_date: 1700000000000,
            folder_prefix: "media/".into(),
            name: "photos".into(),
            media_type: "Folder".into(),
            size_bytes: 0,
        };

        let folder_data: FolderData = FolderData::from(domain);

        assert_eq!(folder_data.folder_path, "FOLDER#media/photos/");
        assert_eq!(folder_data.folder_name, "photos");
        assert_eq!(folder_data.parent_path, "media/");
        assert_eq!(folder_data.created_date, 1700000000000);
        assert_eq!(folder_data.owner_id, "owner1");
    }

    #[test]
    fn test_file_to_file_response() {
        let file = File {
            bucket_key: "user123/photos/vacation.jpg".into(),
            bucket: "test-bucket".into(),
            owner_id: "user123".into(),
            file_id: "sha256-hash".into(),
            file_name: "vacation.jpg".into(),
            file_path: "photos/vacation.jpg".into(),
            folder_prefix: "photos/".into(),
            created_date: 1700000000000,
            size_bytes: 1048576,
            content_type: "image/jpeg".into(),
            media_type: MediaType::Image,
            media_metadata: Some(MediaMetadata::Image(ImageMetadata {
                width: 1920,
                height: 1080,
                exif: None,
                gps: None,
            })),
        };

        let response = FileResponse::from_file(file, "d123.cloudfront.net");

        assert!(response
            .data
            .file_url
            .starts_with("https://d123.cloudfront.net/"));
        assert_eq!(response.data.file_id, "sha256-hash");
        assert_eq!(response.data.file_name, "vacation.jpg");
        assert!(response.data.media_metadata.is_some());
    }
}
