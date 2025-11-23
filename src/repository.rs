use crate::error::StorageError;
use crate::service::{models::ViewLink, File};
use async_trait::async_trait;
use aws_sdk_s3::operation::head_object::HeadObjectOutput;

pub mod dynamodb;
pub mod s3;
pub mod ssm;
pub mod utils;

#[cfg(test)]
pub mod mock;

#[async_trait]
pub trait DynamoDbRepositoryTrait: Send + Sync {
    async fn put_file_and_view_links(
        &self,
        file: &File,
        view_links: &[ViewLink],
    ) -> Result<(), StorageError>;

    async fn find_view_links_by_folder(
        &self,
        user_id: &str,
        folder_path: &str,
        limit: i32,
        cursor: Option<String>,
    ) -> Result<(Vec<ViewLink>, Option<String>), StorageError>;

    async fn get_file(&self, user_id: &str, file_path: &str) -> Result<Option<File>, StorageError>;
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

#[async_trait]
pub trait SsmRepositoryTrait: Send + Sync {
    async fn get_parameter(
        &self,
        path: &str,
        with_decryption: bool,
    ) -> Result<String, StorageError>;
}
