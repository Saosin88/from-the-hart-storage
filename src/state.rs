use crate::repository::DynamoDbRepositoryTrait;
use std::sync::Arc;

#[cfg(feature = "sqs")]
use crate::repository::S3RepositoryTrait;

#[cfg(feature = "sqs")]
use crate::service::metadata::MetadataServiceTrait;

#[cfg(feature = "http")]
use crate::service::access::CloudFrontSigner;

#[derive(Clone)]
pub struct AppState {
    #[cfg(feature = "sqs")]
    pub s3_repository: Option<Arc<dyn S3RepositoryTrait>>,
    pub dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
    #[cfg(feature = "sqs")]
    pub metadata_service: Option<Arc<dyn MetadataServiceTrait>>,
    #[cfg(feature = "http")]
    pub cloudfront_signer: Option<Arc<CloudFrontSigner>>,
}

impl AppState {
    pub fn new(
        #[cfg(feature = "sqs")] s3_repository: Option<Arc<dyn S3RepositoryTrait>>,
        dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
        #[cfg(feature = "sqs")] metadata_service: Option<Arc<dyn MetadataServiceTrait>>,
        #[cfg(feature = "http")] cloudfront_signer: Option<Arc<CloudFrontSigner>>,
    ) -> Self {
        Self {
            #[cfg(feature = "sqs")]
            s3_repository,
            dynamo_db_repository,
            #[cfg(feature = "sqs")]
            metadata_service,
            #[cfg(feature = "http")]
            cloudfront_signer,
        }
    }
}
