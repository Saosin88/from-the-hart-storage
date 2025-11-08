use crate::services;
use axum::{http::StatusCode, response::IntoResponse, Json};

pub async fn health() -> impl IntoResponse {
    let health_status = services::get_health_status();
    (StatusCode::OK, Json(health_status))
}
