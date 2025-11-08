use crate::services;
use actix_web::{HttpResponse, Responder};

pub async fn health() -> impl Responder {
    let health_status = services::get_health_status();
    HttpResponse::Ok().json(health_status)
}
