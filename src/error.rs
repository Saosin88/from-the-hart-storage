use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Service not initialized: {context}")]
    NotInitialized {
        context: String,
    },

    #[error("Resource not found: {context}")]
    NotFound {
        context: String,
    },

    #[error("Internal error: {context}")]
    Internal {
        context: String,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("S3 error: {context}")]
    S3 {
        context: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("DynamoDB error: {context}")]
    DynamoDb {
        context: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Metadata extraction error: {context}")]
    Metadata {
        context: String,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("Time calculation error: {context}")]
    Time {
        context: String,
    },

    #[error("URL decoding error: {context}")]
    UrlDecode {
        context: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Serialization error: {context}")]
    Serialization {
        context: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Invalid format: {context}")]
    InvalidFormat {
        context: String,
    },
}
