use crate::{
    error::StorageError,
    repository::DynamoDbRepositoryTrait,
    service::models::File,
};

pub async fn get_file(
    user_id: &str,
    path: &str,
    dynamo_db_repository: &(impl DynamoDbRepositoryTrait + ?Sized),
) -> Result<Option<File>, StorageError> {
    dynamo_db_repository.get_file(user_id, path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::MockDynamoDbRepository;
    use crate::service::models::File;

    #[tokio::test]
    async fn test_get_file_success() {
        let mock_repo = MockDynamoDbRepository::new();
        let file = File::new("user/file.txt".into(), "bucket".into());
        let mock_repo = mock_repo.with_get_file_response(Ok(Some(file.clone())));

        let result = get_file("user", "file.txt", &mock_repo).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(file));
    }

    #[tokio::test]
    async fn test_get_file_not_found() {
        let mock_repo = MockDynamoDbRepository::new();
        let mock_repo = mock_repo.with_get_file_response(Ok(None));

        let result = get_file("user", "file.txt", &mock_repo).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_get_file_error() {
        let mock_repo = MockDynamoDbRepository::new();
        let mock_repo = mock_repo.with_get_file_response(Err(StorageError::DynamoDb {
            context: "Error".into(),
            source: anyhow::anyhow!("Error"),
        }));

        let result = get_file("user", "file.txt", &mock_repo).await;

        assert!(result.is_err());
    }
}
