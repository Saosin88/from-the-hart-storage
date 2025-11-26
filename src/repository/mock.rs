use crate::error::StorageError;
use crate::repository::{DynamoDbRepositoryTrait, S3RepositoryTrait, SsmRepositoryTrait};
use crate::service::{models::ViewLink, File};
use async_trait::async_trait;
use aws_sdk_s3::operation::head_object::HeadObjectOutput;
use std::sync::{Arc, Mutex};

type FetchHeadBytesResponse = Arc<Mutex<Option<Result<Vec<u8>, StorageError>>>>;
type HeadObjectResponse = Arc<Mutex<Option<Result<HeadObjectOutput, StorageError>>>>;

#[derive(Clone)]
pub struct MockS3Repository {
    pub head_object_response: HeadObjectResponse,
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

    pub fn with_head_object_response(
        self,
        response: Result<HeadObjectOutput, StorageError>,
    ) -> Self {
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
type PutFileResponse = Arc<Mutex<Option<Result<(), StorageError>>>>;
type FindViewLinksResponse =
    Arc<Mutex<Option<Result<(Vec<ViewLink>, Option<String>), StorageError>>>>;
type GetFileResponse = Arc<Mutex<Option<Result<Option<File>, StorageError>>>>;
type FolderExistsCall = (String, String);
type FolderExistsResponse = Arc<Mutex<Option<Result<bool, StorageError>>>>;
type CreateFolderCall = (String, String);
type CreateFolderResponse = Arc<Mutex<Option<Result<ViewLink, StorageError>>>>;

#[derive(Clone)]
pub struct MockDynamoDbRepository {
    pub put_file_calls: Arc<Mutex<Vec<PutFileCall>>>,
    pub put_file_response: PutFileResponse,
    pub find_view_links_response: FindViewLinksResponse,
    pub get_file_response: GetFileResponse,
    pub folder_exists_calls: Arc<Mutex<Vec<FolderExistsCall>>>,
    pub folder_exists_response: FolderExistsResponse,
    pub create_folder_calls: Arc<Mutex<Vec<CreateFolderCall>>>,
    pub create_folder_response: CreateFolderResponse,
}

impl Default for MockDynamoDbRepository {
    fn default() -> Self {
        Self {
            put_file_calls: Arc::new(Mutex::new(Vec::new())),
            put_file_response: Arc::new(Mutex::new(Some(Ok(())))),
            find_view_links_response: Arc::new(Mutex::new(None)),
            get_file_response: Arc::new(Mutex::new(None)),
            folder_exists_calls: Arc::new(Mutex::new(Vec::new())),
            folder_exists_response: Arc::new(Mutex::new(None)),
            create_folder_calls: Arc::new(Mutex::new(Vec::new())),
            create_folder_response: Arc::new(Mutex::new(None)),
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

    pub fn with_find_view_links_response(
        self,
        response: Result<(Vec<ViewLink>, Option<String>), StorageError>,
    ) -> Self {
        *self.find_view_links_response.lock().unwrap() = Some(response);
        self
    }

    pub fn with_get_file_response(self, response: Result<Option<File>, StorageError>) -> Self {
        *self.get_file_response.lock().unwrap() = Some(response);
        self
    }

    pub fn with_folder_exists_response(self, response: Result<bool, StorageError>) -> Self {
        *self.folder_exists_response.lock().unwrap() = Some(response);
        self
    }

    pub fn with_create_folder_response(self, response: Result<ViewLink, StorageError>) -> Self {
        *self.create_folder_response.lock().unwrap() = Some(response);
        self
    }

    pub fn folder_exists_calls(&self) -> Vec<FolderExistsCall> {
        self.folder_exists_calls.lock().unwrap().clone()
    }

    pub fn create_folder_calls(&self) -> Vec<CreateFolderCall> {
        self.create_folder_calls.lock().unwrap().clone()
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
        let mut lock = self.find_view_links_response.lock().unwrap();
        if let Some(response) = lock.take() {
            return response;
        }
        Ok((vec![], None))
    }

    async fn get_file(
        &self,
        _user_id: &str,
        _file_path: &str,
    ) -> Result<Option<File>, StorageError> {
        let mut lock = self.get_file_response.lock().unwrap();
        if let Some(response) = lock.take() {
            return response;
        }
        Ok(None)
    }

    async fn folder_exists(&self, user_id: &str, folder_path: &str) -> Result<bool, StorageError> {
        self.folder_exists_calls
            .lock()
            .unwrap()
            .push((user_id.to_string(), folder_path.to_string()));

        let mut lock = self.folder_exists_response.lock().unwrap();
        if let Some(response) = lock.take() {
            return response;
        }
        Ok(false)
    }

    async fn create_folder(
        &self,
        user_id: &str,
        folder_path: &str,
    ) -> Result<ViewLink, StorageError> {
        self.create_folder_calls
            .lock()
            .unwrap()
            .push((user_id.to_string(), folder_path.to_string()));

        let mut lock = self.create_folder_response.lock().unwrap();
        if let Some(response) = lock.take() {
            return response;
        }
        Err(StorageError::DynamoDb {
            context: "Mock not configured".to_string(),
            source: anyhow::anyhow!("Mock not configured"),
        })
    }
}

use crate::service::metadata::MetadataServiceTrait;

#[derive(Clone, Default)]
pub struct MockMetadataService;

impl MockMetadataService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MetadataServiceTrait for MockMetadataService {
    async fn extract_metadata(&self, _head_bytes: &[u8], _file: &mut File) {}
}

type GetParameterResponse = Arc<Mutex<Option<Result<String, StorageError>>>>;

#[derive(Clone)]
pub struct MockSsmRepository {
    pub get_parameter_response: GetParameterResponse,
    pub get_parameter_calls: Arc<Mutex<Vec<(String, bool)>>>,
}

impl Default for MockSsmRepository {
    fn default() -> Self {
        Self {
            get_parameter_response: Arc::new(Mutex::new(None)),
            get_parameter_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl MockSsmRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_get_parameter_response(self, response: Result<String, StorageError>) -> Self {
        *self.get_parameter_response.lock().unwrap() = Some(response);
        self
    }

    pub fn get_parameter_calls(&self) -> Vec<(String, bool)> {
        self.get_parameter_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl SsmRepositoryTrait for MockSsmRepository {
    async fn get_parameter(
        &self,
        path: &str,
        with_decryption: bool,
    ) -> Result<String, StorageError> {
        self.get_parameter_calls
            .lock()
            .unwrap()
            .push((path.to_string(), with_decryption));

        let mut lock = self.get_parameter_response.lock().unwrap();
        if let Some(response) = lock.take() {
            return response;
        }
        Err(StorageError::Ssm {
            context: "Mock SSM not configured".to_string(),
            source: anyhow::anyhow!("Mock not configured"),
        })
    }
}
