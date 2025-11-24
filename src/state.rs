use crate::repository::{DynamoDbRepositoryTrait, S3RepositoryTrait};
use crate::service::access::CloudFrontSigner;
use crate::service::metadata::MetadataServiceTrait;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub s3_repository: Option<Arc<dyn S3RepositoryTrait>>,
    pub dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
    pub metadata_service: Option<Arc<dyn MetadataServiceTrait>>,
    pub cloudfront_signer: Option<Arc<CloudFrontSigner>>,
}

impl AppState {
    pub fn new(
        s3_repository: Option<Arc<dyn S3RepositoryTrait>>,
        dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
        metadata_service: Option<Arc<dyn MetadataServiceTrait>>,
        cloudfront_signer: Option<Arc<CloudFrontSigner>>,
    ) -> Self {
        Self {
            s3_repository,
            dynamo_db_repository,
            metadata_service,
            cloudfront_signer,
        }
    }
}
