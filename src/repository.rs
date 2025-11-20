use crate::error::StorageError;
use crate::service::{models::ViewLink, File};
use async_trait::async_trait;
use aws_sdk_s3::operation::head_object::HeadObjectOutput;

pub mod dynamodb;
pub mod s3;
pub mod utils;

#[async_trait]
pub trait DynamoDbRepositoryTrait: Send + Sync {
    async fn put_file_and_view_links(
        &self,
        file: &File,
        view_links: &[ViewLink],
    ) -> Result<(), StorageError>;
}

#[async_trait]
pub trait S3RepositoryTrait: Send + Sync {
    async fn get_object_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<HeadObjectOutput, StorageError>;

    async fn fetch_head_bytes(
        &self,
        bucket: &str,
        key: &str,
        num_bytes: u64,
    ) -> Result<Vec<u8>, StorageError>;
}
