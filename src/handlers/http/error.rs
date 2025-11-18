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
            StorageError::NotInitialized(msg) => HttpError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_NOT_INITIALIZED".to_string(),
                "Service is not properly initialized".to_string(),
                Some(msg),
            ),
            StorageError::NotFound(msg) => HttpError::new(
                StatusCode::NOT_FOUND,
                "NOT_FOUND".to_string(),
                "Resource not found".to_string(),
                Some(msg),
            ),
            StorageError::Internal(msg) => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR".to_string(),
                "An internal error occurred".to_string(),
                Some(msg),
            ),
            StorageError::S3(msg) => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "S3_ERROR".to_string(),
                "Storage operation failed".to_string(),
                Some(msg),
            ),
            StorageError::Metadata(msg) => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "METADATA_ERROR".to_string(),
                "Metadata extraction failed".to_string(),
                Some(msg),
            ),
            StorageError::Time(msg) => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "TIME_ERROR".to_string(),
                "Time calculation failed".to_string(),
                Some(msg),
            ),
            StorageError::UrlDecode(msg) => HttpError::new(
                StatusCode::BAD_REQUEST,
                "URL_DECODE_ERROR".to_string(),
                "Failed to decode URL".to_string(),
                Some(msg),
            ),
            StorageError::Serialization(msg) => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERIALIZATION_ERROR".to_string(),
                "Failed to serialize data".to_string(),
                Some(msg),
            ),
            StorageError::InvalidFormat(msg) => HttpError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT_ERROR".to_string(),
                "Invalid format encountered".to_string(),
                Some(msg),
            ),
        }
    }
}
