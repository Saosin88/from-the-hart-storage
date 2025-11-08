use crate::{models::HealthResponse, services};
use aide::{axum::IntoApiResponse, transform::TransformOperation};
use axum::Json;

pub async fn health() -> impl IntoApiResponse {
    let health_status = services::get_health_status();
    Json(health_status)
}

pub fn health_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Health check endpoint that returns the service status, uptime, and current timestamp",
    )
    .summary("Check service health")
    .tag("Health")
    .response::<200, Json<HealthResponse>>()
}
