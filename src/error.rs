use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Not found")]
    NotFound,
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("S3 error: {0}")]
    S3Error(String),
    #[error("Metadata extraction error: {0}")]
    MetadataError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::S3Error(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::MetadataError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

// Convert from anyhow::Error for backward compatibility
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
