use crate::{
    error::StorageError,
    repository::DynamoDbRepositoryTrait,
    service::models::ViewLink,
};

pub async fn list_folder_contents(
    user_id: &str,
    path: &str,
    limit: i32,
    cursor: Option<String>,
    dynamo_db_repository: &impl DynamoDbRepositoryTrait,
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
