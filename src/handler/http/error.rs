use crate::error::StorageError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Error details")]
pub struct ErrorData {
    #[schemars(description = "Error code indicating the type of error")]
    pub code: String,
    #[schemars(description = "Human-readable error message")]
    pub message: String,
    #[schemars(description = "Optional additional details about the error")]
    pub details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Error response for failed requests")]
pub struct HttpErrorResponse {
    #[schemars(description = "Error details")]
    pub error: ErrorData,
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
                error: ErrorData {
                    code,
                    message,
                    details,
                },
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
            StorageError::JwtParse { .. } => HttpError::new(
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED".to_string(),
                "Invalid or missing authentication token".to_string(),
                None,
            ),
            StorageError::CloudFrontSigning { .. } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR".to_string(),
                "Failed to generate signed access".to_string(),
                None,
            ),
            StorageError::Ssm { .. } => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR".to_string(),
                "Configuration error".to_string(),
                None,
            ),
            // Catch-all for internal errors (500) - Include sanitized context for diagnostics
            _ => {
                let context = err.to_string(); // sanitized by thiserror Display impl
                HttpError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR".to_string(),
                    "An unexpected error occurred".to_string(),
                    Some(context),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StorageError;

    #[test]
    fn test_not_found_to_http_error() {
        let err = StorageError::NotFound {
            context: "x".into(),
        };
        let http: HttpError = HttpError::from(err);
        assert_eq!(http.status, StatusCode::NOT_FOUND);
        assert_eq!(http.response.error.code, "NOT_FOUND");
    }

    #[test]
    fn test_not_initialized_to_http_error() {
        let err = StorageError::NotInitialized {
            context: "x".into(),
        };
        let http: HttpError = HttpError::from(err);
        assert_eq!(http.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(http.response.error.code, "SERVICE_UNAVAILABLE");
    }

    #[test]
    fn test_invalid_request_to_http_error() {
        let err = StorageError::InvalidRequest {
            context: "x".into(),
            source: anyhow::anyhow!("bad param"),
        };
        let http: HttpError = HttpError::from(err);
        assert_eq!(http.status, StatusCode::BAD_REQUEST);
        assert_eq!(http.response.error.code, "BAD_REQUEST");
    }

    #[test]
    fn test_jwt_parse_to_http_error() {
        let err = StorageError::JwtParse {
            context: "x".into(),
            source: anyhow::anyhow!("bad token"),
        };
        let http: HttpError = HttpError::from(err);
        assert_eq!(http.status, StatusCode::UNAUTHORIZED);
        assert_eq!(http.response.error.code, "UNAUTHORIZED");
    }

    #[test]
    fn test_internal_errors_to_http_error() {
        // Test several internal errors all map to 500/INTERNAL_ERROR
        let cases: Vec<StorageError> = vec![
            StorageError::Internal {
                context: "oops".into(),
                source: None,
            },
            StorageError::S3 {
                context: "s3 failed".into(),
                source: anyhow::anyhow!("s3 error"),
            },
            StorageError::DynamoDb {
                context: "ddb failed".into(),
                source: anyhow::anyhow!("ddb error"),
            },
            StorageError::Metadata {
                context: "meta failed".into(),
                source: None,
            },
        ];

        for err in cases {
            let http: HttpError = HttpError::from(err);
            assert_eq!(
                http.status,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Expected 500 for {:?}",
                http.response.error.code
            );
            assert_eq!(http.response.error.code, "INTERNAL_ERROR");
            assert!(
                http.response.error.details.is_some(),
                "Expected details for Internal errors"
            );
            assert!(
                !http.response.error.details.as_ref().unwrap().is_empty(),
                "Expected non-empty details"
            );
        }
    }
}
