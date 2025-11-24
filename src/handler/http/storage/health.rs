use crate::{
    handler::http::{
        dto::HealthResponse,
        error::HttpErrorResponse,
    },
    service::health,
};

use aide::{axum::IntoApiResponse, transform::TransformOperation};
use axum::{http::StatusCode, response::IntoResponse, Json};

pub async fn health() -> impl IntoApiResponse {
    match health::get_health_status() {
        Ok(status) => {
            let response = HealthResponse::from(status);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            let http_error = crate::handler::http::error::HttpError::from(err);
            http_error.into_response()
        }
    }
}

pub fn health_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Health check endpoint that verifies the service is operational and returns detailed status information.\n\n\
        This endpoint performs diagnostic checks including:\n\
        - Service availability and initialization status\n\
        - System time functionality\n\
        - Uptime calculation\n\
        A 200 OK response indicates the service is healthy and operational.\n\
        A 503 Service Unavailable response indicates the service has encountered an error and may not be functioning correctly.",
    )
    .summary("Check service health status")
    .tag("Health")
    .response::<200, Json<HealthResponse>>()
    .response_with::<503, Json<HttpErrorResponse>, _>(|res| {
        res.description("Service is unavailable or experiencing errors")
            .example(HttpErrorResponse {
                error: crate::handler::http::error::ErrorData {
                    code: "SERVICE_NOT_INITIALIZED".to_string(),
                    message: "Service is not properly initialized".to_string(),
                    details: Some("Service start time not initialized".to_string()),
                },
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::http::routes;
    use crate::repository::mock::{MockDynamoDbRepository, MockMetadataService};
    use crate::state::AppState;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use std::sync::Arc;

    fn create_test_state() -> AppState {
        let dynamodb_mock = MockDynamoDbRepository::new();
        let metadata_mock = MockMetadataService::new();
        AppState::new(
            None,
            Arc::new(dynamodb_mock),
            Some(Arc::new(metadata_mock)),
            None,
        )
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_ok_when_service_initialized() {
        crate::utils::time::init_start_time();
        let app = routes::configure_routes(create_test_state());
        let server = TestServer::new(app).unwrap();
        let response = server.get("/storage/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let body: HealthResponse = response.json();
        assert_eq!(body.data.status, "ok");
        assert!(body.data.timestamp > 0);
    }

    #[tokio::test]
    async fn test_health_endpoint_response_structure() {
        crate::utils::time::init_start_time();
        let app = routes::configure_routes(create_test_state());
        let server = TestServer::new(app).unwrap();
        let response = server.get("/storage/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let body: serde_json::Value = response.json();
        assert!(body.get("data").is_some());
        assert!(body["data"].get("status").is_some());
        assert!(body["data"].get("uptime").is_some());
        assert!(body["data"].get("timestamp").is_some());
    }

    #[tokio::test]
    async fn test_get_health_status_returns_ok() {
        crate::utils::time::init_start_time();
        let result = health::get_health_status();
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.timestamp > 0);
    }
}
