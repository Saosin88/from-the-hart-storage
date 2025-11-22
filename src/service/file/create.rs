use crate::service::file::utils::{calculate_folder_prefix, get_ancestor_folder_paths};
use crate::service::metadata::MetadataServiceTrait;
use crate::service::models::ViewLink;
use crate::utils::string;
use crate::{
    error::StorageError,
    repository::{DynamoDbRepositoryTrait, S3RepositoryTrait},
    service::File,
    utils::time,
};

use std::path::Path;
use tracing::{error, info};

pub async fn handle_file_created(
    file: File,
    s3_repository: &(impl S3RepositoryTrait + ?Sized),
    dynamo_db_repository: &(impl DynamoDbRepositoryTrait + ?Sized),
    metadata_service: &(impl MetadataServiceTrait + ?Sized),
) -> Result<(), StorageError> {
    let bucket = file.bucket.clone();
    let key = file.bucket_key.clone();

    info!(bucket = %bucket, key = %key, "Processing file");

    let mut file = parse_and_init_file(file)?;

    enrich_with_s3_metadata(&mut file, s3_repository).await;

    enrich_with_media_metadata(&mut file, s3_repository, metadata_service).await?;

    let file_view_link = ViewLink::for_owner(&file);

    let ancestor_paths = get_ancestor_folder_paths(&file.file_path);
    let mut folder_view_links: Vec<ViewLink> = ancestor_paths
        .iter()
        .map(|folder_path| ViewLink::for_owner_folder(&file, folder_path))
        .collect();

    let mut all_view_links = vec![file_view_link];
    all_view_links.append(&mut folder_view_links);

    info!(
        "Creating FILE item + {} VIEW_LINKs (1 file + {} folder markers)",
        all_view_links.len(),
        ancestor_paths.len()
    );

    dynamo_db_repository
        .put_file_and_view_links(&file, &all_view_links)
        .await
        .map_err(|e| StorageError::DynamoDb {
            context: "Failed to put file and view links in DynamoDB".to_string(),
            source: e.into(),
        })?;

    info!("File processed successfully");

    Ok(())
}

fn parse_and_init_file(mut file: File) -> Result<File, StorageError> {
    let key = &file.bucket_key;
    let bucket = &file.bucket;

    let (owner, path) = key
        .split_once('/')
        .ok_or_else(|| StorageError::InvalidFormat {
            context: format!("Invalid S3 key format: {}", key),
        })?;

    file.owner_id = owner.into();
    file.file_id = string::sha256_hash(&format!("{}/{}", bucket, key)).into();
    file.file_name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .into();
    file.file_path = path.into();
    file.folder_prefix = calculate_folder_prefix(path).into();

    Ok(file)
}

async fn enrich_with_s3_metadata(file: &mut File, s3_repository: &(impl S3RepositoryTrait + ?Sized)) {
    match s3_repository
        .get_object_metadata(&file.bucket, &file.bucket_key)
        .await
    {
        Ok(response) => {
            if let Some(ct) = response.content_type() {
                file.content_type = ct.into();
            }

            if let Some(len) = response.content_length() {
                file.size_bytes = len;
            }

            if let Some(lm) = response.last_modified() {
                if let Some(timestamp) =
                    time::parse_media_datetime_with_offset(&lm.to_string(), None)
                {
                    file.created_date = timestamp;
                } else {
                    tracing::debug!(
                        bucket = %file.bucket,
                        key = %file.bucket_key,
                        "Could not parse S3 last_modified"
                    );
                }
            }
        }
        Err(e) => {
            error!(
                bucket = %file.bucket,
                key = %file.bucket_key,
                error = %e,
                "Failed to fetch object metadata from S3; continuing without the extra data"
            );
        }
    }
}

async fn enrich_with_media_metadata(
    file: &mut File,
    s3_repository: &(impl S3RepositoryTrait + ?Sized),
    metadata_service: &(impl MetadataServiceTrait + ?Sized),
) -> Result<(), StorageError> {
    let num_bytes = std::cmp::min(512 * 1024, file.size_bytes as u64);

    let head_bytes = s3_repository
        .fetch_head_bytes(&file.bucket, &file.bucket_key, num_bytes)
        .await
        .map_err(|e| StorageError::S3 {
            context: format!(
                "Failed to fetch first {} bytes from S3 for {}/{}",
                num_bytes, file.bucket, file.bucket_key
            ),
            source: e.into(),
        })?;

    info!(
        "Fetched {} bytes from S3 for metadata extraction",
        head_bytes.len()
    );

    metadata_service.extract_metadata(&head_bytes, file).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::{MockDynamoDbRepository, MockS3Repository};
    use aws_sdk_s3::primitives::DateTime;
    use aws_sdk_s3::operation::head_object::HeadObjectOutput;
    use std::time::SystemTime;

    struct MockMetadataService;

    #[async_trait::async_trait]
    impl MetadataServiceTrait for MockMetadataService {
        async fn extract_metadata(&self, _head_bytes: &[u8], _file: &mut File) {}
    }

    fn create_test_file() -> File {
        File::new("user123/folder/test.jpg".to_string(), "test-bucket".to_string())
    }

    #[tokio::test]
    async fn test_handle_file_created_success() {
        let file = create_test_file();
        let s3_mock = MockS3Repository::new()
            .with_head_object_response(Ok(HeadObjectOutput::builder()
                .content_type("image/jpeg")
                .content_length(1024)
                .last_modified(DateTime::from(SystemTime::now()))
                .build()))
            .with_fetch_head_bytes_response(Ok(vec![0; 100]));
        
        let dynamodb_mock = MockDynamoDbRepository::new();
        let metadata_mock = MockMetadataService;

        let result = handle_file_created(file, &s3_mock, &dynamodb_mock, &metadata_mock).await;

        assert!(result.is_ok());

        let calls = dynamodb_mock.put_file_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (saved_file, view_links) = &calls[0];
        
        assert_eq!(&*saved_file.bucket, "test-bucket");
        assert_eq!(&*saved_file.bucket_key, "user123/folder/test.jpg");
        assert_eq!(&*saved_file.content_type, "image/jpeg");
        assert_eq!(saved_file.size_bytes, 1024);
        
        // 1 file link + 1 folder link (folder)
        assert_eq!(view_links.len(), 2); 
    }

    #[tokio::test]
    async fn test_handle_file_created_s3_metadata_failure() {
        let file = create_test_file();
        // S3 metadata fails, but process should continue
        let s3_mock = MockS3Repository::new()
            .with_head_object_response(Err(StorageError::S3 { 
                context: "fail".into(), 
                source: anyhow::anyhow!("fail") 
            }))
            .with_fetch_head_bytes_response(Ok(vec![]));
            
        let dynamodb_mock = MockDynamoDbRepository::new();
        let metadata_mock = MockMetadataService;

        let result = handle_file_created(file, &s3_mock, &dynamodb_mock, &metadata_mock).await;

        assert!(result.is_ok());
        
        let calls = dynamodb_mock.put_file_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_file_created_dynamodb_failure() {
        let file = create_test_file();
        let s3_mock = MockS3Repository::new()
            .with_head_object_response(Ok(HeadObjectOutput::builder().build()))
            .with_fetch_head_bytes_response(Ok(vec![]));
            
        let dynamodb_mock = MockDynamoDbRepository::new()
            .with_put_file_response(Err(StorageError::DynamoDb { 
                context: "fail".into(), 
                source: anyhow::anyhow!("fail") 
            }));
            
        let metadata_mock = MockMetadataService;

        let result = handle_file_created(file, &s3_mock, &dynamodb_mock, &metadata_mock).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::DynamoDb { .. } => {},
            _ => panic!("Expected DynamoDb error"),
        }
    }
}
