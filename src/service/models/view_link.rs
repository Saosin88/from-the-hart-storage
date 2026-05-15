use serde::{Deserialize, Serialize};

use super::file::File;
use crate::service::file::utils::{get_folder_name, get_parent_folder_path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ResourceId {
    File(String),
    Folder(String),
}

impl ResourceId {
    #[must_use]
    pub fn is_folder(&self) -> bool {
        matches!(self, ResourceId::Folder(_))
    }

    #[must_use]
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
    #[must_use]
    pub fn is_folder(&self) -> bool {
        self.resource_id.is_folder()
    }

    #[must_use]
    pub fn resource_id_str(&self) -> &str {
        self.resource_id.as_str()
    }

    #[must_use]
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

    #[must_use]
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
