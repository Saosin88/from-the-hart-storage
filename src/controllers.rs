use crate::{models::{ErrorResponse, HealthResponse}, services};
use aide::{axum::IntoApiResponse, transform::TransformOperation};
use axum::{http::StatusCode, response::IntoResponse, Json};

pub async fn health() -> impl IntoApiResponse {
    match services::get_health_status() {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => {
            let error_response = ErrorResponse {
                code: "SERVICE_UNAVAILABLE".to_string(),
                message: "Health check failed".to_string(),
                details: Some(e.to_string()),
            };
            (StatusCode::SERVICE_UNAVAILABLE, Json(error_response)).into_response()
        }
    }
}

pub fn health_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Health check endpoint that verifies the service is operational and returns detailed status information.\n\n\
        This endpoint performs diagnostic checks including:\n\
        - Service availability and initialization status\n\
        - System time functionality\n\
        - Uptime calculation\n\n\
        A 200 OK response indicates the service is healthy and operational.\n\
        A 503 Service Unavailable response indicates the service has encountered an error and may not be functioning correctly.",
    )
    .summary("Check service health status")
    .tag("Health")
    .response::<200, Json<HealthResponse>>()
    .response_with::<503, Json<ErrorResponse>, _>(|res| {
        res.description("Service is unavailable or experiencing errors")
            .example(ErrorResponse {
                code: "SERVICE_UNAVAILABLE".to_string(),
                message: "Health check failed".to_string(),
                details: Some("Time error: Failed to calculate uptime".to_string()),
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use crate::routes;

    #[tokio::test]
    async fn test_health_endpoint_returns_ok_when_service_initialized() {
        // Initialize the service start time
        crate::services::init_start_time();

        // Create test server with routes
        let app = routes::configure_routes();
        let server = TestServer::new(app).unwrap();

        // Make request to health endpoint
        let response = server.get("/storage/health").await;

        // Assert response
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let body: HealthResponse = response.json();
        assert_eq!(body.data.status, "ok");
        assert!(body.data.timestamp > 0);
    }

    #[tokio::test]
    async fn test_health_endpoint_response_structure() {
        // Initialize the service
        crate::services::init_start_time();

        let app = routes::configure_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/storage/health").await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        // Verify JSON structure
        let body: serde_json::Value = response.json();
        assert!(body.get("data").is_some());
        assert!(body["data"].get("status").is_some());
        assert!(body["data"].get("uptime").is_some());
        assert!(body["data"].get("timestamp").is_some());
    }

    #[tokio::test]
    async fn test_get_health_status_returns_ok() {
        // Initialize the service
        crate::services::init_start_time();

        let result = services::get_health_status();
        
        assert!(result.is_ok());
        let health = result.unwrap();
        assert_eq!(health.data.status, "ok");
        assert!(health.data.timestamp > 0);
    }
}
