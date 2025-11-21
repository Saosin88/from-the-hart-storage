use crate::{
    error::StorageError,
    repository::DynamoDbRepositoryTrait,
    service::models::File,
};

pub async fn get_file(
    user_id: &str,
    path: &str,
    dynamo_db_repository: &impl DynamoDbRepositoryTrait,
) -> Result<Option<File>, StorageError> {
    dynamo_db_repository.get_file(user_id, path).await
}
