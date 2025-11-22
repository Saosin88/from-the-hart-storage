use crate::error::StorageError;
use crate::repository::{DynamoDbRepositoryTrait, S3RepositoryTrait};
use crate::service::{models::ViewLink, File};
use async_trait::async_trait;
use aws_sdk_s3::operation::head_object::HeadObjectOutput;
use std::sync::{Arc, Mutex};

type FetchHeadBytesResponse = Arc<Mutex<Option<Result<Vec<u8>, StorageError>>>>;

#[derive(Clone)]
pub struct MockS3Repository {
    pub head_object_response: Arc<Mutex<Option<Result<HeadObjectOutput, StorageError>>>>,
    pub fetch_head_bytes_response: FetchHeadBytesResponse,
}

impl Default for MockS3Repository {
    fn default() -> Self {
        Self {
            head_object_response: Arc::new(Mutex::new(None)),
            fetch_head_bytes_response: Arc::new(Mutex::new(None)),
        }
    }
}

impl MockS3Repository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_head_object_response(self, response: Result<HeadObjectOutput, StorageError>) -> Self {
        *self.head_object_response.lock().unwrap() = Some(response);
        self
    }

    pub fn with_fetch_head_bytes_response(self, response: Result<Vec<u8>, StorageError>) -> Self {
        *self.fetch_head_bytes_response.lock().unwrap() = Some(response);
        self
    }
}

#[async_trait]
impl S3RepositoryTrait for MockS3Repository {
    async fn get_object_metadata(
        &self,
        _bucket: &str,
        _key: &str,
    ) -> Result<HeadObjectOutput, StorageError> {
        let mut lock = self.head_object_response.lock().unwrap();
        if let Some(response) = lock.take() {
             return response;
        }
        Err(StorageError::S3 {
            context: "Mock S3 metadata not configured".to_string(),
            source: anyhow::anyhow!("Mock not configured"),
        })
    }

    async fn fetch_head_bytes(
        &self,
        _bucket: &str,
        _key: &str,
        _num_bytes: u64,
    ) -> Result<Vec<u8>, StorageError> {
        let mut lock = self.fetch_head_bytes_response.lock().unwrap();
        if let Some(response) = lock.take() {
             return response;
        }
        Err(StorageError::S3 {
            context: "Mock S3 bytes not configured".to_string(),
            source: anyhow::anyhow!("Mock not configured"),
        })
    }
}

type PutFileCall = (File, Vec<ViewLink>);

#[derive(Clone)]
pub struct MockDynamoDbRepository {
    pub put_file_calls: Arc<Mutex<Vec<PutFileCall>>>,
    pub put_file_response: Arc<Mutex<Option<Result<(), StorageError>>>>,
}

impl Default for MockDynamoDbRepository {
    fn default() -> Self {
        Self {
            put_file_calls: Arc::new(Mutex::new(Vec::new())),
            put_file_response: Arc::new(Mutex::new(Some(Ok(())))),
        }
    }
}

impl MockDynamoDbRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_put_file_response(self, response: Result<(), StorageError>) -> Self {
        *self.put_file_response.lock().unwrap() = Some(response);
        self
    }
}

#[async_trait]
impl DynamoDbRepositoryTrait for MockDynamoDbRepository {
    async fn put_file_and_view_links(
        &self,
        file: &File,
        view_links: &[ViewLink],
    ) -> Result<(), StorageError> {
        self.put_file_calls
            .lock()
            .unwrap()
            .push((file.clone(), view_links.to_vec()));

        let mut lock = self.put_file_response.lock().unwrap();
        if let Some(response) = lock.take() {
            match &response {
                Ok(_) => {
                    *lock = Some(Ok(()));
                    Ok(())
                }
                Err(_) => response,
            }
        } else {
             Ok(())
        }
    }

    async fn find_view_links_by_folder(
        &self,
        _user_id: &str,
        _folder_path: &str,
        _limit: i32,
        _cursor: Option<String>,
    ) -> Result<(Vec<ViewLink>, Option<String>), StorageError> {
        Ok((vec![], None))
    }

    async fn get_file(&self, _user_id: &str, _file_path: &str) -> Result<Option<File>, StorageError> {
        Ok(None)
    }
}
