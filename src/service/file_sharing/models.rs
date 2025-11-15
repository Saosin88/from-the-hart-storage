use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a file metadata item in DynamoDB (canonical record)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    /// Partition Key: USER#<OwnerID>
    #[serde(rename = "PK")]
    pub pk: String,
    
    /// Sort Key: FILE#<Path>
    #[serde(rename = "SK")]
    pub sk: String,
    
    /// Item type identifier
    #[serde(rename = "ItemType")]
    pub item_type: String,
    
    /// Unique file identifier
    #[serde(rename = "FileID")]
    pub file_id: String,
    
    /// Owner user ID
    #[serde(rename = "OwnerID")]
    pub owner_id: String,
    
    /// File name only (no path)
    #[serde(rename = "FileName")]
    pub file_name: String,
    
    /// Folder prefix (S3-style, e.g., "media/photos/")
    #[serde(rename = "FolderPrefix")]
    pub folder_prefix: String,
    
    /// Creation timestamp (Unix milliseconds)
    #[serde(rename = "CreatedDate")]
    pub created_date: i64,
    
    /// MIME type (e.g., "image/jpeg")
    #[serde(rename = "MediaType")]
    pub media_type: String,
    
    /// S3 object key
    #[serde(rename = "S3Key")]
    pub s3_key: String,
    
    /// File size in bytes
    #[serde(rename = "Size")]
    pub size: i64,
    
    /// Content type
    #[serde(rename = "ContentType")]
    pub content_type: String,
    
    /// Optional metadata (exif, gps, etc.)
    #[serde(rename = "MediaMetadata", skip_serializing_if = "Option::is_none")]
    pub media_metadata: Option<HashMap<String, serde_json::Value>>,
}

impl FileItem {
    pub fn new(
        owner_id: String,
        file_path: String,
        file_id: String,
        file_name: String,
        folder_prefix: String,
        s3_key: String,
        media_type: String,
        content_type: String,
        size: i64,
    ) -> Self {
        Self {
            pk: format!("USER#{}", owner_id),
            sk: format!("FILE#{}", file_path),
            item_type: "FILE".to_string(),
            file_id,
            owner_id,
            file_name,
            folder_prefix,
            created_date: chrono::Utc::now().timestamp_millis(),
            media_type,
            s3_key,
            size,
            content_type,
            media_metadata: None,
        }
    }
}

/// Represents a sharing grant (prefix-level permission)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareGrantItem {
    /// Partition Key: USER#<OwnerID>
    #[serde(rename = "PK")]
    pub pk: String,
    
    /// Sort Key: GRANT#<RecipientID>#<GrantID>
    #[serde(rename = "SK")]
    pub sk: String,
    
    /// Item type identifier
    #[serde(rename = "ItemType")]
    pub item_type: String,
    
    /// Unique grant identifier (UUID)
    #[serde(rename = "GrantID")]
    pub grant_id: String,
    
    /// Owner user ID (who is sharing)
    #[serde(rename = "OwnerID")]
    pub owner_id: String,
    
    /// Recipient user ID (who receives access)
    #[serde(rename = "RecipientID")]
    pub recipient_id: String,
    
    /// Permission level (READ or READ/WRITE)
    #[serde(rename = "Permissions")]
    pub permissions: String,
    
    /// Folder prefix being shared (e.g., "media/photos/")
    #[serde(rename = "Prefix")]
    pub prefix: String,
    
    /// Creation timestamp (Unix milliseconds)
    #[serde(rename = "CreatedDate")]
    pub created_date: i64,
    
    /// GSI1 Partition Key: ACCESS#<RecipientID>
    #[serde(rename = "GSI1-PK")]
    pub gsi1_pk: String,
    
    /// GSI1 Sort Key: GRANT#<OwnerID>#<Prefix>
    #[serde(rename = "GSI1-SK")]
    pub gsi1_sk: String,
}

impl ShareGrantItem {
    pub fn new(
        owner_id: String,
        recipient_id: String,
        prefix: String,
        permissions: String,
    ) -> Self {
        let grant_id = format!("G-{}", uuid::Uuid::new_v4());
        
        Self {
            pk: format!("USER#{}", owner_id),
            sk: format!("GRANT#{}#{}", recipient_id, grant_id),
            item_type: "SHARE_GRANT".to_string(),
            grant_id,
            owner_id: owner_id.clone(),
            recipient_id: recipient_id.clone(),
            permissions,
            prefix: prefix.clone(),
            created_date: chrono::Utc::now().timestamp_millis(),
            gsi1_pk: format!("ACCESS#{}", recipient_id),
            gsi1_sk: format!("GRANT#{}#{}", owner_id, prefix),
        }
    }
}

/// Represents a denormalized view link for merged folder views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewLinkItem {
    /// Partition Key: USER#<ViewerID>
    #[serde(rename = "PK")]
    pub pk: String,
    
    /// Sort Key: VIEWLINK#<OwnerID>#<FileID>
    #[serde(rename = "SK")]
    pub sk: String,
    
    /// Item type identifier
    #[serde(rename = "ItemType")]
    pub item_type: String,
    
    /// File identifier
    #[serde(rename = "FileID")]
    pub file_id: String,
    
    /// Owner of the file
    #[serde(rename = "OwnerID")]
    pub owner_id: String,
    
    /// Grant ID that authorized this view (or "OWNER" for owner's own files)
    #[serde(rename = "GrantID")]
    pub grant_id: String,
    
    /// File creation timestamp
    #[serde(rename = "CreatedDate")]
    pub created_date: i64,
    
    /// Normalized folder name (e.g., "Project Docs/")
    #[serde(rename = "FolderName")]
    pub folder_name: String,
    
    /// Media type for filtering
    #[serde(rename = "MediaType")]
    pub media_type: String,
    
    /// GSI2 Partition Key: VIEWER#<ViewerID>#FOLDER#<FolderName>
    #[serde(rename = "GSI2-PK")]
    pub gsi2_pk: String,
    
    /// GSI2 Sort Key: <MediaType>#<CreatedDate>#<FileID>
    #[serde(rename = "GSI2-SK")]
    pub gsi2_sk: String,
}

impl ViewLinkItem {
    pub fn new(
        viewer_id: String,
        owner_id: String,
        file_id: String,
        grant_id: String,
        folder_name: String,
        media_type: String,
        created_date: i64,
    ) -> Self {
        Self {
            pk: format!("USER#{}", viewer_id),
            sk: format!("VIEWLINK#{}#{}", owner_id, file_id),
            item_type: "VIEW_LINK".to_string(),
            file_id: file_id.clone(),
            owner_id: owner_id.clone(),
            grant_id,
            created_date,
            folder_name: folder_name.clone(),
            media_type: media_type.clone(),
            gsi2_pk: format!("VIEWER#{}#FOLDER#{}", viewer_id, folder_name),
            gsi2_sk: format!("{}#{}#{}", media_type, created_date, file_id),
        }
    }
    
    /// Create a view link for owner's own file
    pub fn for_owner(
        owner_id: String,
        file_id: String,
        folder_name: String,
        media_type: String,
        created_date: i64,
    ) -> Self {
        Self::new(
            owner_id.clone(),
            owner_id,
            file_id,
            "OWNER".to_string(),
            folder_name,
            media_type,
            created_date,
        )
    }
}

/// Query result for folder contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderContentItem {
    pub file_id: String,
    pub owner_id: String,
    pub file_name: String,
    pub media_type: String,
    pub size: i64,
    pub created_date: i64,
}

/// Query result for shared folders list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFolder {
    pub grant_id: String,
    pub owner_id: String,
    pub prefix: String,
    pub permissions: String,
    pub created_date: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_item_creation() {
        let file = FileItem::new(
            "Sheldon".to_string(),
            "media/photos/image.jpg".to_string(),
            "R102".to_string(),
            "image.jpg".to_string(),
            "media/photos/".to_string(),
            "Sheldon/media/photos/image.jpg".to_string(),
            "image/jpeg".to_string(),
            "image/jpeg".to_string(),
            161713,
        );
        
        assert_eq!(file.pk, "USER#Sheldon");
        assert_eq!(file.sk, "FILE#media/photos/image.jpg");
        assert_eq!(file.item_type, "FILE");
        assert_eq!(file.owner_id, "Sheldon");
    }

    #[test]
    fn test_share_grant_creation() {
        let grant = ShareGrantItem::new(
            "Sheldon".to_string(),
            "Justin".to_string(),
            "media/photos/".to_string(),
            "READ".to_string(),
        );
        
        assert_eq!(grant.pk, "USER#Sheldon");
        assert!(grant.sk.starts_with("GRANT#Justin#G-"));
        assert_eq!(grant.item_type, "SHARE_GRANT");
        assert_eq!(grant.gsi1_pk, "ACCESS#Justin");
        assert_eq!(grant.gsi1_sk, "GRANT#Sheldon#media/photos/");
    }

    #[test]
    fn test_view_link_creation() {
        let view_link = ViewLinkItem::for_owner(
            "Sheldon".to_string(),
            "R102".to_string(),
            "photos/".to_string(),
            "image/jpeg".to_string(),
            1224685719000,
        );
        
        assert_eq!(view_link.pk, "USER#Sheldon");
        assert_eq!(view_link.sk, "VIEWLINK#Sheldon#R102");
        assert_eq!(view_link.item_type, "VIEW_LINK");
        assert_eq!(view_link.grant_id, "OWNER");
        assert_eq!(view_link.gsi2_pk, "VIEWER#Sheldon#FOLDER#photos/");
        assert!(view_link.gsi2_sk.starts_with("image/jpeg#"));
    }
}
