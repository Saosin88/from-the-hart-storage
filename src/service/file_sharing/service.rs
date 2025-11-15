use super::models::*;
use super::repository::FileShareRepository;
use crate::error::ServiceError;

/// Service layer for file sharing operations
pub struct FileShareService {
    repository: FileShareRepository,
}

impl FileShareService {
    pub fn new(repository: FileShareRepository) -> Self {
        Self { repository }
    }

    /// Create a new file record
    pub async fn create_file(
        &self,
        owner_id: String,
        file_path: String,
        file_id: String,
        file_name: String,
        folder_prefix: String,
        s3_key: String,
        media_type: String,
        content_type: String,
        size: i64,
    ) -> Result<(), ServiceError> {
        let file = FileItem::new(
            owner_id.clone(),
            file_path,
            file_id.clone(),
            file_name,
            folder_prefix.clone(),
            s3_key,
            media_type.clone(),
            content_type,
            size,
        );

        self.repository.put_file(&file).await?;

        // Create view link for owner's own file
        let folder_name = Self::extract_folder_name(&folder_prefix);
        let view_link = ViewLinkItem::for_owner(
            owner_id,
            file_id,
            folder_name,
            media_type,
            file.created_date,
        );

        self.repository.create_view_links(vec![view_link]).await?;

        Ok(())
    }

    /// Get files in a folder (owner's view)
    pub async fn list_folder_contents(
        &self,
        owner_id: &str,
        prefix: &str,
    ) -> Result<Vec<FileItem>, ServiceError> {
        self.repository.query_files_by_prefix(owner_id, prefix).await
    }

    /// Grant access to a folder prefix
    pub async fn create_share(
        &self,
        owner_id: String,
        recipient_id: String,
        prefix: String,
        permissions: String,
    ) -> Result<String, ServiceError> {
        let grant = ShareGrantItem::new(owner_id, recipient_id, prefix, permissions);
        let grant_id = grant.grant_id.clone();

        self.repository.create_share_grant(&grant).await?;

        // View links will be created lazily on first access
        Ok(grant_id)
    }

    /// Revoke access to a folder prefix
    pub async fn revoke_share(
        &self,
        owner_id: &str,
        recipient_id: &str,
        grant_id: &str,
    ) -> Result<(), ServiceError> {
        // Delete the grant (immediate revocation)
        self.repository
            .delete_share_grant(owner_id, recipient_id, grant_id)
            .await?;

        // Delete view links asynchronously (handled by SQS worker)
        // For now, we'll delete them here synchronously
        self.repository
            .delete_view_links_for_grant(recipient_id, grant_id)
            .await?;

        Ok(())
    }

    /// List all folders shared with a user
    pub async fn list_shared_folders(
        &self,
        recipient_id: &str,
    ) -> Result<Vec<SharedFolder>, ServiceError> {
        let grants = self
            .repository
            .query_grants_for_recipient(recipient_id)
            .await?;

        Ok(grants
            .into_iter()
            .map(|g| SharedFolder {
                grant_id: g.grant_id,
                owner_id: g.owner_id,
                prefix: g.prefix,
                permissions: g.permissions,
                created_date: g.created_date,
            })
            .collect())
    }

    /// Get merged view of a folder (combining files from multiple owners)
    pub async fn get_merged_folder_view(
        &self,
        viewer_id: &str,
        folder_name: &str,
        media_type_filter: Option<&str>,
        limit: Option<i32>,
        cursor: Option<String>,
    ) -> Result<(Vec<ViewLinkItem>, Option<String>), ServiceError> {
        // Parse cursor if provided (in a real implementation, this would be more robust)
        let last_key = cursor.map(|_c| {
            // Simplified cursor parsing - in production, use proper encoding
            let map = std::collections::HashMap::new();
            // This is a placeholder - proper implementation would decode the cursor
            map
        });

        let (view_links, last_evaluated_key) = self
            .repository
            .query_merged_folder_view(
                viewer_id,
                folder_name,
                media_type_filter,
                limit,
                last_key,
            )
            .await?;

        // Convert last_evaluated_key to cursor string
        let next_cursor = last_evaluated_key.map(|_| {
            // Simplified cursor encoding - in production, properly encode the key
            "next_page".to_string()
        });

        Ok((view_links, next_cursor))
    }

    /// Create view links for a file (called by DynamoDB Streams Lambda)
    pub async fn create_view_links_for_file(
        &self,
        file: &FileItem,
    ) -> Result<(), ServiceError> {
        // Find all grants that match this file's prefix
        let grants = self
            .repository
            .query_grants_for_prefix(&file.owner_id, &file.folder_prefix)
            .await?;

        let folder_name = Self::extract_folder_name(&file.folder_prefix);
        let mut view_links = Vec::new();

        // Create view link for owner
        view_links.push(ViewLinkItem::for_owner(
            file.owner_id.clone(),
            file.file_id.clone(),
            folder_name.clone(),
            file.media_type.clone(),
            file.created_date,
        ));

        // Create view links for all recipients
        for grant in grants {
            view_links.push(ViewLinkItem::new(
                grant.recipient_id,
                file.owner_id.clone(),
                file.file_id.clone(),
                grant.grant_id,
                folder_name.clone(),
                file.media_type.clone(),
                file.created_date,
            ));
        }

        self.repository.create_view_links(view_links).await?;

        Ok(())
    }

    /// Extract normalized folder name from prefix
    /// e.g., "media/Project Docs/" -> "Project Docs/"
    /// e.g., "media/photos/2024/" -> "2024/"
    fn extract_folder_name(prefix: &str) -> String {
        if prefix.is_empty() {
            return "/".to_string();
        }

        // Get the last folder in the path
        let parts: Vec<&str> = prefix.trim_end_matches('/').split('/').collect();
        if let Some(last) = parts.last() {
            format!("{}/", last)
        } else {
            "/".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_folder_name() {
        assert_eq!(
            FileShareService::extract_folder_name("media/Project Docs/"),
            "Project Docs/"
        );
        assert_eq!(
            FileShareService::extract_folder_name("media/photos/2024/"),
            "2024/"
        );
        assert_eq!(FileShareService::extract_folder_name(""), "/");
        assert_eq!(FileShareService::extract_folder_name("photos/"), "photos/");
    }
}
