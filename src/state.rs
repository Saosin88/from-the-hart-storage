use crate::{
    repository::{DynamoDbRepositoryTrait, S3RepositoryTrait},
    service::metadata::MetadataServiceTrait,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub s3_repository: Arc<dyn S3RepositoryTrait>,
    pub dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
    pub metadata_service: Arc<dyn MetadataServiceTrait>,
}

impl AppState {
    pub fn new(
        s3_repository: Arc<dyn S3RepositoryTrait>,
        dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
        metadata_service: Arc<dyn MetadataServiceTrait>,
    ) -> Self {
        Self {
            s3_repository,
            dynamo_db_repository,
            metadata_service,
        }
    }
}
