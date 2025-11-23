use crate::repository::{DynamoDbRepositoryTrait, S3RepositoryTrait};
use std::sync::Arc;

#[cfg(feature = "sqs")]
use crate::service::metadata::MetadataServiceTrait;

#[derive(Clone)]
pub struct AppState {
    pub s3_repository: Arc<dyn S3RepositoryTrait>,
    pub dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
    #[cfg(feature = "sqs")]
    pub metadata_service: Arc<dyn MetadataServiceTrait>,
}

impl AppState {
    #[cfg(feature = "sqs")]
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

    #[cfg(not(feature = "sqs"))]
    pub fn new(
        s3_repository: Arc<dyn S3RepositoryTrait>,
        dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
    ) -> Self {
        Self {
            s3_repository,
            dynamo_db_repository,
        }
    }
}
