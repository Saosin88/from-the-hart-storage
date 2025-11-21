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
        tracing::error!(error = ?err, "Request failed with storage error");

        match err {
            StorageError::NotInitialized { .. } => HttpError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE".to_string(),
                "Service is temporarily unavailable".to_string(),
                None,
            ),
            StorageError::NotFound { .. } => HttpError::new(
                StatusCode::NOT_FOUND,
                "NOT_FOUND".to_string(),
                "Resource not found".to_string(),
                None,
            ),
            StorageError::UrlDecode { .. } => HttpError::new(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST".to_string(),
                "Invalid URL encoding".to_string(),
                None,
            ),
            StorageError::InvalidFormat { .. } => HttpError::new(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST".to_string(),
                "Invalid format".to_string(),
                None,
            ),
            StorageError::InvalidRequest { .. } => HttpError::new(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST".to_string(),
                "Invalid request parameters".to_string(),
                None,
            ),
            // Catch-all for internal errors (500) - Sanitize output
            _ => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR".to_string(),
                "An unexpected error occurred".to_string(),
                None,
            ),
        }
    }
}
