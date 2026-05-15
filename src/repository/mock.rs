use crate::error::StorageError;
use crate::repository::{DynamoDbRepositoryTrait, S3RepositoryTrait, SsmRepositoryTrait};
use crate::service::metadata::MetadataServiceTrait;
use crate::service::models::{File, ViewLink};
use async_trait::async_trait;
use aws_sdk_s3::operation::head_object::HeadObjectOutput;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// MockS3Repository
// ---------------------------------------------------------------------------

type FetchHeadBytesResponses = Arc<Mutex<VecDeque<Result<Vec<u8>, StorageError>>>>;
type HeadObjectResponses = Arc<Mutex<VecDeque<Result<HeadObjectOutput, StorageError>>>>;

#[derive(Clone)]
pub struct MockS3Repository {
    pub strict: Arc<AtomicBool>,
    pub head_object_responses: HeadObjectResponses,
    pub fetch_head_bytes_responses: FetchHeadBytesResponses,
}

impl Default for MockS3Repository {
    fn default() -> Self {
        Self {
            strict: Arc::new(AtomicBool::new(false)),
            head_object_responses: Arc::new(Mutex::new(VecDeque::new())),
            fetch_head_bytes_responses: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl MockS3Repository {
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_strict_mode(&self, strict: bool) {
        self.strict.store(strict, Ordering::SeqCst);
    }

    #[must_use] 
    pub fn with_head_object_response(
        self,
        response: Result<HeadObjectOutput, StorageError>,
    ) -> Self {
        self.head_object_responses
            .lock()
            .unwrap()
            .push_back(response);
        self
    }

    #[must_use] 
    pub fn with_head_object_responses(
        self,
        responses: Vec<Result<HeadObjectOutput, StorageError>>,
    ) -> Self {
        {
            let mut lock = self.head_object_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
        self
    }

    #[must_use] 
    pub fn with_fetch_head_bytes_response(self, response: Result<Vec<u8>, StorageError>) -> Self {
        self.fetch_head_bytes_responses
            .lock()
            .unwrap()
            .push_back(response);
        self
    }

    #[must_use] 
    pub fn with_fetch_head_bytes_responses(
        self,
        responses: Vec<Result<Vec<u8>, StorageError>>,
    ) -> Self {
        {
            let mut lock = self.fetch_head_bytes_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
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
        let mut lock = self.head_object_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockS3Repository::get_object_metadata called but no responses configured");
        }
        tracing::warn!("MockS3Repository::get_object_metadata exhausted, returning default error");
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
        let mut lock = self.fetch_head_bytes_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockS3Repository::fetch_head_bytes called but no responses configured");
        }
        tracing::warn!("MockS3Repository::fetch_head_bytes exhausted, returning default error");
        Err(StorageError::S3 {
            context: "Mock S3 bytes not configured".to_string(),
            source: anyhow::anyhow!("Mock not configured"),
        })
    }
}

// ---------------------------------------------------------------------------
// MockDynamoDbRepository
// ---------------------------------------------------------------------------

type PutFileCall = (File, Vec<ViewLink>);
type PutFileResponses = Arc<Mutex<VecDeque<Result<(), StorageError>>>>;
type FindViewLinksResponses =
    Arc<Mutex<VecDeque<Result<(Vec<ViewLink>, Option<String>), StorageError>>>>;
type GetFileResponses = Arc<Mutex<VecDeque<Result<Option<File>, StorageError>>>>;
type FolderExistsCall = (String, String);
type FolderExistsResponses = Arc<Mutex<VecDeque<Result<bool, StorageError>>>>;
type CreateFolderCall = (String, String);
type CreateFolderResponses = Arc<Mutex<VecDeque<Result<ViewLink, StorageError>>>>;

#[derive(Clone)]
pub struct MockDynamoDbRepository {
    pub strict: Arc<AtomicBool>,
    pub put_file_calls: Arc<Mutex<Vec<PutFileCall>>>,
    pub put_file_responses: PutFileResponses,
    pub find_view_links_responses: FindViewLinksResponses,
    pub get_file_responses: GetFileResponses,
    pub folder_exists_calls: Arc<Mutex<Vec<FolderExistsCall>>>,
    pub folder_exists_responses: FolderExistsResponses,
    pub create_folder_calls: Arc<Mutex<Vec<CreateFolderCall>>>,
    pub create_folder_responses: CreateFolderResponses,
}

impl Default for MockDynamoDbRepository {
    fn default() -> Self {
        Self {
            strict: Arc::new(AtomicBool::new(false)),
            put_file_calls: Arc::new(Mutex::new(Vec::new())),
            put_file_responses: Arc::new(Mutex::new(VecDeque::new())),
            find_view_links_responses: Arc::new(Mutex::new(VecDeque::new())),
            get_file_responses: Arc::new(Mutex::new(VecDeque::new())),
            folder_exists_calls: Arc::new(Mutex::new(Vec::new())),
            folder_exists_responses: Arc::new(Mutex::new(VecDeque::new())),
            create_folder_calls: Arc::new(Mutex::new(Vec::new())),
            create_folder_responses: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl MockDynamoDbRepository {
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_strict_mode(&self, strict: bool) {
        self.strict.store(strict, Ordering::SeqCst);
    }

    #[must_use] 
    pub fn with_put_file_response(self, response: Result<(), StorageError>) -> Self {
        self.put_file_responses.lock().unwrap().push_back(response);
        self
    }

    #[must_use] 
    pub fn with_put_file_responses(self, responses: Vec<Result<(), StorageError>>) -> Self {
        {
            let mut lock = self.put_file_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
        self
    }

    #[must_use] 
    pub fn with_find_view_links_response(
        self,
        response: Result<(Vec<ViewLink>, Option<String>), StorageError>,
    ) -> Self {
        self.find_view_links_responses
            .lock()
            .unwrap()
            .push_back(response);
        self
    }

    #[allow(clippy::type_complexity)]
    #[must_use] 
    pub fn with_find_view_links_responses(
        self,
        responses: Vec<Result<(Vec<ViewLink>, Option<String>), StorageError>>,
    ) -> Self {
        {
            let mut lock = self.find_view_links_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
        self
    }

    #[must_use] 
    pub fn with_get_file_response(self, response: Result<Option<File>, StorageError>) -> Self {
        self.get_file_responses.lock().unwrap().push_back(response);
        self
    }

    #[must_use] 
    pub fn with_get_file_responses(
        self,
        responses: Vec<Result<Option<File>, StorageError>>,
    ) -> Self {
        {
            let mut lock = self.get_file_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
        self
    }

    #[must_use] 
    pub fn with_folder_exists_response(self, response: Result<bool, StorageError>) -> Self {
        self.folder_exists_responses
            .lock()
            .unwrap()
            .push_back(response);
        self
    }

    #[must_use] 
    pub fn with_folder_exists_responses(self, responses: Vec<Result<bool, StorageError>>) -> Self {
        {
            let mut lock = self.folder_exists_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
        self
    }

    #[must_use] 
    pub fn with_create_folder_response(self, response: Result<ViewLink, StorageError>) -> Self {
        self.create_folder_responses
            .lock()
            .unwrap()
            .push_back(response);
        self
    }

    #[must_use] 
    pub fn with_create_folder_responses(
        self,
        responses: Vec<Result<ViewLink, StorageError>>,
    ) -> Self {
        {
            let mut lock = self.create_folder_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
        self
    }

    #[must_use] 
    pub fn folder_exists_calls(&self) -> Vec<FolderExistsCall> {
        self.folder_exists_calls.lock().unwrap().clone()
    }

    #[must_use] 
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

        let mut lock = self.put_file_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockDynamoDbRepository::put_file_and_view_links called but no responses configured");
        }
        tracing::warn!(
            "MockDynamoDbRepository::put_file_and_view_links exhausted, returning default Ok(())"
        );
        Ok(())
    }

    async fn find_view_links_by_folder(
        &self,
        _user_id: &str,
        _folder_path: &str,
        _limit: i32,
        _cursor: Option<String>,
    ) -> Result<(Vec<ViewLink>, Option<String>), StorageError> {
        let mut lock = self.find_view_links_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockDynamoDbRepository::find_view_links_by_folder called but no responses configured");
        }
        tracing::warn!(
            "MockDynamoDbRepository::find_view_links_by_folder exhausted, returning default empty"
        );
        Ok((vec![], None))
    }

    async fn get_file(
        &self,
        _user_id: &str,
        _file_path: &str,
    ) -> Result<Option<File>, StorageError> {
        let mut lock = self.get_file_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockDynamoDbRepository::get_file called but no responses configured");
        }
        tracing::warn!("MockDynamoDbRepository::get_file exhausted, returning default None");
        Ok(None)
    }

    async fn folder_exists(&self, user_id: &str, folder_path: &str) -> Result<bool, StorageError> {
        self.folder_exists_calls
            .lock()
            .unwrap()
            .push((user_id.to_string(), folder_path.to_string()));

        let mut lock = self.folder_exists_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockDynamoDbRepository::folder_exists called but no responses configured");
        }
        tracing::warn!("MockDynamoDbRepository::folder_exists exhausted, returning default false");
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

        let mut lock = self.create_folder_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockDynamoDbRepository::create_folder called but no responses configured");
        }
        tracing::warn!("MockDynamoDbRepository::create_folder exhausted, returning default error");
        Err(StorageError::DynamoDb {
            context: "Mock not configured".to_string(),
            source: anyhow::anyhow!("Mock not configured"),
        })
    }
}

// ---------------------------------------------------------------------------
// MockMetadataService
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct MockMetadataService;

impl MockMetadataService {
    #[must_use] 
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MetadataServiceTrait for MockMetadataService {
    async fn extract_metadata(&self, _head_bytes: &[u8], _file: &mut File) {}
}

// ---------------------------------------------------------------------------
// MockSsmRepository
// ---------------------------------------------------------------------------

type GetParameterResponses = Arc<Mutex<VecDeque<Result<String, StorageError>>>>;

#[derive(Clone)]
pub struct MockSsmRepository {
    pub strict: Arc<AtomicBool>,
    pub get_parameter_responses: GetParameterResponses,
    pub get_parameter_calls: Arc<Mutex<Vec<(String, bool)>>>,
}

impl Default for MockSsmRepository {
    fn default() -> Self {
        Self {
            strict: Arc::new(AtomicBool::new(false)),
            get_parameter_responses: Arc::new(Mutex::new(VecDeque::new())),
            get_parameter_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl MockSsmRepository {
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_strict_mode(&self, strict: bool) {
        self.strict.store(strict, Ordering::SeqCst);
    }

    #[must_use] 
    pub fn with_get_parameter_response(self, response: Result<String, StorageError>) -> Self {
        self.get_parameter_responses
            .lock()
            .unwrap()
            .push_back(response);
        self
    }

    #[must_use] 
    pub fn with_get_parameter_responses(
        self,
        responses: Vec<Result<String, StorageError>>,
    ) -> Self {
        {
            let mut lock = self.get_parameter_responses.lock().unwrap();
            lock.clear();
            for r in responses {
                lock.push_back(r);
            }
        }
        self
    }

    #[must_use] 
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

        let mut lock = self.get_parameter_responses.lock().unwrap();
        if let Some(response) = lock.pop_front() {
            return response;
        }
        if self.strict.load(Ordering::SeqCst) {
            panic!("MockSsmRepository::get_parameter called but no responses configured");
        }
        tracing::warn!("MockSsmRepository::get_parameter exhausted, returning default error");
        Err(StorageError::Ssm {
            context: "Mock SSM not configured".to_string(),
            source: anyhow::anyhow!("Mock not configured"),
        })
    }
}
