use crate::{error::StorageError, repository::DynamoDbRepositoryTrait, service::models::ViewLink};

pub async fn list_folder_contents(
    user_id: &str,
    path: &str,
    limit: i32,
    cursor: Option<String>,
    dynamo_db_repository: &(impl DynamoDbRepositoryTrait + ?Sized),
) -> Result<(Vec<ViewLink>, Option<String>), StorageError> {
    let folder_path = if path.is_empty() {
        "".to_string()
    } else if !path.ends_with('/') {
        format!("{}/", path)
    } else {
        path.to_string()
    };

    dynamo_db_repository
        .find_view_links_by_folder(user_id, &folder_path, limit, cursor)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::MockDynamoDbRepository;
    use crate::service::models::ViewLink;

    #[tokio::test]
    async fn test_list_folder_contents_success() {
        let mock_repo = MockDynamoDbRepository::new();
        let file = crate::service::models::File::new("file.txt".into(), "bucket".into());
        let view_links = vec![ViewLink::for_owner(&file)];
        let mock_repo = mock_repo.with_find_view_links_response(Ok((view_links.clone(), None)));

        let result = list_folder_contents("user", "folder", 10, None, &mock_repo).await;

        assert!(result.is_ok());
        let (links, cursor) = result.unwrap();
        assert_eq!(links, view_links);
        assert_eq!(cursor, None);
    }

    #[tokio::test]
    async fn test_list_folder_contents_error() {
        let mock_repo = MockDynamoDbRepository::new();
        let mock_repo = mock_repo.with_find_view_links_response(Err(StorageError::DynamoDb {
            context: "Error".into(),
            source: anyhow::anyhow!("Error"),
        }));

        let result = list_folder_contents("user", "folder", 10, None, &mock_repo).await;

        assert!(result.is_err());
    }
}
