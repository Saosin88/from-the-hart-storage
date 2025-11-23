use crate::repository::{DynamoDbRepositoryTrait, S3RepositoryTrait};
use std::sync::Arc;

#[cfg(feature = "sqs")]
use crate::service::metadata::MetadataServiceTrait;

#[cfg(feature = "http")]
use crate::service::access::CloudFrontSigner;

#[derive(Clone)]
pub struct AppState {
    pub s3_repository: Arc<dyn S3RepositoryTrait>,
    pub dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
    #[cfg(feature = "sqs")]
    pub metadata_service: Arc<dyn MetadataServiceTrait>,
    #[cfg(feature = "http")]
    pub cloudfront_signer: Option<Arc<CloudFrontSigner>>,
}

impl AppState {
    #[cfg(all(feature = "sqs", feature = "http"))]
    pub fn new(
        s3_repository: Arc<dyn S3RepositoryTrait>,
        dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
        metadata_service: Arc<dyn MetadataServiceTrait>,
        cloudfront_signer: Option<Arc<CloudFrontSigner>>,
    ) -> Self {
        Self {
            s3_repository,
            dynamo_db_repository,
            metadata_service,
            cloudfront_signer,
        }
    }

    #[cfg(all(feature = "sqs", not(feature = "http")))]
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

    #[cfg(all(not(feature = "sqs"), feature = "http"))]
    pub fn new(
        s3_repository: Arc<dyn S3RepositoryTrait>,
        dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
        cloudfront_signer: Option<Arc<CloudFrontSigner>>,
    ) -> Self {
        Self {
            s3_repository,
            dynamo_db_repository,
            cloudfront_signer,
        }
    }

    #[cfg(all(not(feature = "sqs"), not(feature = "http")))]
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
