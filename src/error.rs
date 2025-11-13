use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Service not initialized: {0}")]
    NotInitialized(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("S3 error: {0}")]
    S3(String),

    #[error("Metadata extraction error: {0}")]
    Metadata(String),

    #[error("Time calculation error: {0}")]
    Time(String),

    #[error("URL decoding error: {0}")]
    UrlDecode(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
