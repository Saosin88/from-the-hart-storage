use crate::error::StorageError;
use crate::repository::DynamoDbRepositoryTrait;
use crate::service::file::utils::get_parent_folder_path;
use crate::service::models::ViewLink;

fn validate_folder_path(folder_path: &str) -> Result<(), StorageError> {
    if !folder_path.ends_with('/') {
        return Err(StorageError::InvalidRequest {
            context: "Folder path must end with '/'".to_string(),
            source: anyhow::anyhow!("Invalid folder path: {}", folder_path),
        });
    }

    if folder_path.starts_with('/') {
        return Err(StorageError::InvalidRequest {
            context: "Folder path must not start with '/'".to_string(),
            source: anyhow::anyhow!("Invalid folder path: {}", folder_path),
        });
    }

    if folder_path.contains("//") {
        return Err(StorageError::InvalidRequest {
            context: "Folder path must not contain '//' sequences".to_string(),
            source: anyhow::anyhow!("Invalid folder path: {}", folder_path),
        });
    }

    let segments: Vec<&str> = folder_path
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    for segment in &segments {
        if *segment == "." || *segment == ".." {
            return Err(StorageError::InvalidRequest {
                context: "Folder path must not contain '.' or '..' segments".to_string(),
                source: anyhow::anyhow!("Invalid folder path: {}", folder_path),
            });
        }
    }

    Ok(())
}

pub async fn create_folder(
    repo: &dyn DynamoDbRepositoryTrait,
    user_id: &str,
    folder_path: &str,
) -> Result<ViewLink, StorageError> {
    validate_folder_path(folder_path)?;

    if repo.folder_exists(user_id, folder_path).await? {
        return repo.create_folder(user_id, folder_path).await;
    }

    let parent_path = get_parent_folder_path(folder_path);
    if !parent_path.is_empty() {
        let parent_exists = repo.folder_exists(user_id, &parent_path).await?;
        if !parent_exists {
            return Err(StorageError::NotFound {
                context: format!("Parent folder '{}' does not exist", parent_path),
            });
        }
    }

    repo.create_folder(user_id, folder_path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::MockDynamoDbRepository;
    use crate::service::file::utils::get_folder_name;
    use crate::utils::time;

    fn mock_view_link(user_id: &str, folder_path: &str) -> ViewLink {
        ViewLink {
            viewer_id: user_id.into(),
            resource_id: folder_path.into(),
            owner_id: user_id.into(),
            grant_id: "OWNER".into(),
            created_date: time::now_as_unix_millis(),
            folder_prefix: get_parent_folder_path(folder_path).into(),
            name: get_folder_name(folder_path).into(),
            media_type: "Folder".into(),
            size_bytes: 0,
            is_folder: true,
        }
    }

    #[test]
    fn test_validate_folder_path_valid_root() {
        assert!(validate_folder_path("media/").is_ok());
    }

    #[test]
    fn test_validate_folder_path_valid_nested() {
        assert!(validate_folder_path("media/photos/2024/").is_ok());
    }

    #[test]
    fn test_validate_folder_path_missing_trailing_slash() {
        let result = validate_folder_path("media");
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InvalidRequest { context, .. } => {
                assert!(context.contains("must end with '/'"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_folder_path_leading_slash() {
        let result = validate_folder_path("/media/");
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InvalidRequest { context, .. } => {
                assert!(context.contains("must not start with '/'"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_folder_path_double_slash() {
        let result = validate_folder_path("media//photos/");
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InvalidRequest { context, .. } => {
                assert!(context.contains("must not contain '//' sequences"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_folder_path_dot_segment() {
        let result = validate_folder_path("media/./photos/");
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InvalidRequest { context, .. } => {
                assert!(context.contains("must not contain '.' or '..'"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_folder_path_dotdot_segment() {
        let result = validate_folder_path("media/../photos/");
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InvalidRequest { context, .. } => {
                assert!(context.contains("must not contain '.' or '..'"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[tokio::test]
    async fn test_create_folder_root_level_success() {
        let mock_repo = MockDynamoDbRepository::new()
            .with_folder_exists_response(Ok(false))
            .with_create_folder_response(Ok(mock_view_link("user123", "media/")));

        let result = create_folder(&mock_repo, "user123", "media/").await;

        assert!(result.is_ok());
        let view_link = result.unwrap();
        assert_eq!(&*view_link.resource_id, "media/");
        assert_eq!(&*view_link.name, "media");
        assert!(view_link.is_folder);

        let folder_exists_calls = mock_repo.folder_exists_calls();
        assert_eq!(folder_exists_calls.len(), 1);
        assert_eq!(folder_exists_calls[0].0, "user123");
        assert_eq!(folder_exists_calls[0].1, "media/");

        let create_calls = mock_repo.create_folder_calls();
        assert_eq!(create_calls.len(), 1);
        assert_eq!(create_calls[0].0, "user123");
        assert_eq!(create_calls[0].1, "media/");
    }

    #[tokio::test]
    async fn test_create_folder_nested_parent_missing() {
        let mock_repo = MockDynamoDbRepository::new();

        let result = create_folder(&mock_repo, "user123", "media/photos/").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::NotFound { context } => {
                assert!(context.contains("Parent folder 'media/' does not exist"));
            }
            _ => panic!("Expected NotFound error"),
        }

        let folder_exists_calls = mock_repo.folder_exists_calls();
        assert_eq!(folder_exists_calls.len(), 2);
        assert_eq!(folder_exists_calls[0].1, "media/photos/");
        assert_eq!(folder_exists_calls[1].1, "media/");
    }

    #[tokio::test]
    async fn test_create_folder_idempotent() {
        let mock_repo = MockDynamoDbRepository::new()
            .with_folder_exists_response(Ok(true))
            .with_create_folder_response(Ok(mock_view_link("user123", "media/")));

        let result = create_folder(&mock_repo, "user123", "media/").await;

        assert!(result.is_ok());

        let folder_exists_calls = mock_repo.folder_exists_calls();
        assert_eq!(folder_exists_calls.len(), 1);

        let create_calls = mock_repo.create_folder_calls();
        assert_eq!(create_calls.len(), 1);
    }

    #[tokio::test]
    async fn test_create_folder_invalid_path_no_trailing_slash() {
        let mock_repo = MockDynamoDbRepository::new();

        let result = create_folder(&mock_repo, "user123", "media").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InvalidRequest { .. } => {}
            _ => panic!("Expected InvalidRequest error"),
        }

        let folder_exists_calls = mock_repo.folder_exists_calls();
        assert_eq!(folder_exists_calls.len(), 0);
    }

    #[tokio::test]
    async fn test_create_folder_invalid_path_leading_slash() {
        let mock_repo = MockDynamoDbRepository::new();

        let result = create_folder(&mock_repo, "user123", "/media/").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_folder_invalid_path_double_slash() {
        let mock_repo = MockDynamoDbRepository::new();

        let result = create_folder(&mock_repo, "user123", "media//photos/").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_folder_invalid_path_traversal() {
        let mock_repo = MockDynamoDbRepository::new();

        let result = create_folder(&mock_repo, "user123", "media/../photos/").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_folder_repository_error() {
        let mock_repo = MockDynamoDbRepository::new().with_folder_exists_response(Err(
            StorageError::DynamoDb {
                context: "Database error".to_string(),
                source: anyhow::anyhow!("Connection failed"),
            },
        ));

        let result = create_folder(&mock_repo, "user123", "media/").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::DynamoDb { .. } => {}
            _ => panic!("Expected DynamoDb error"),
        }
    }
}
