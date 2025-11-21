use crate::error::StorageError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Error response for failed requests")]
pub struct HttpErrorResponse {
    #[schemars(description = "Error code indicating the type of error")]
    pub code: String,
    #[schemars(description = "Human-readable error message")]
    pub message: String,
    #[schemars(description = "Optional additional details about the error")]
    pub details: Option<String>,
}

#[derive(Debug)]
pub struct HttpError {
    pub status: StatusCode,
    pub response: HttpErrorResponse,
}

impl HttpError {
    pub fn new(status: StatusCode, code: String, message: String, details: Option<String>) -> Self {
        Self {
            status,
            response: HttpErrorResponse {
                code,
                message,
                details,
            },
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.status, Json(self.response)).into_response()
    }
}

impl From<StorageError> for HttpError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::NotInitialized { context } => HttpError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_NOT_INITIALIZED".to_string(),
                "Service is not properly initialized".to_string(),
                Some(context),
            ),
            StorageError::NotFound { context } => HttpError::new(
                StatusCode::NOT_FOUND,
                "NOT_FOUND".to_string(),
                "Resource not found".to_string(),
                Some(context),
            ),
            StorageError::Internal { context, .. } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR".to_string(),
                "An internal error occurred".to_string(),
                Some(context),
            ),
            StorageError::S3 { context, .. } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "S3_ERROR".to_string(),
                "Failed to interact with S3".to_string(),
                Some(context),
            ),
            StorageError::DynamoDb { context, .. } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DYNAMODB_ERROR".to_string(),
                "Failed to interact with DynamoDB".to_string(),
                Some(context),
            ),
            StorageError::Metadata { context, .. } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "METADATA_ERROR".to_string(),
                "Failed to extract metadata".to_string(),
                Some(context),
            ),
            StorageError::Time { context } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "TIME_ERROR".to_string(),
                "Failed to calculate time".to_string(),
                Some(context),
            ),
            StorageError::UrlDecode { context, .. } => HttpError::new(
                StatusCode::BAD_REQUEST,
                "URL_DECODE_ERROR".to_string(),
                "Failed to decode URL".to_string(),
                Some(context),
            ),
            StorageError::Serialization { context, .. } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERIALIZATION_ERROR".to_string(),
                "Failed to serialize data".to_string(),
                Some(context),
            ),
            StorageError::InvalidFormat { context } => HttpError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT_ERROR".to_string(),
                "Invalid format encountered".to_string(),
                Some(context),
            ),
            StorageError::InvalidRequest { context, .. } => HttpError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_REQUEST".to_string(),
                "Invalid request parameters".to_string(),
                Some(context),
            ),
        }

    }
}
