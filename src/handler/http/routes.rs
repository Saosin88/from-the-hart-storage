use crate::handler::http::storage::{health, list};

use aide::{
    axum::{routing, ApiRouter},
    openapi::OpenApi,
    swagger::Swagger,
    transform::TransformOpenApi,
};
use axum::{response::IntoResponse, Extension, Json, Router};

fn create_api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("From The Hart Storage API")
        .version("1.0.0")
        .summary("Secure cloud storage API for From The Hart platform")
        .description(
            "Provides endpoints for file storage, retrieval, and management with secure access controls.\n\n\
            ## Features\n\
            - Health monitoring and status checks\n\
            - RESTful API design\n\
            - Comprehensive error handling\n\
            - OpenAPI 3.0 specification\n\n\
            ## Response Codes\n\
            The API uses standard HTTP response codes:\n\
            - 2xx: Success\n\
            - 4xx: Client errors\n\
            - 5xx: Server errors\n\n\
            ## Support\n\
            For issues or questions, please refer to the project documentation.",
        )
}

pub fn configure_routes() -> Router {
    let mut api = OpenApi::default();
    let storage_router = ApiRouter::new()
        .api_route(
            "/health",
            routing::get_with(health::health, health::health_docs),
        )
        .route(
            "/{user_id}",
            axum::routing::get(list::handle_file_request),
        )
        .route(
            "/{user_id}/",
            axum::routing::get(list::handle_file_request),
        )
        .route(
            "/{user_id}/{*path}",
            axum::routing::get(list::handle_file_request),
        )
        .route(
            "/openapi.json",
            routing::get_with(
                |Extension(api): Extension<OpenApi>| async move { Json(api).into_response() },
                |op| {
                    op.description(
                        "Returns the complete OpenAPI 3.0 specification for this API in JSON format.\n\n\
                        This endpoint provides machine-readable API documentation that can be used by:\n\
                        - API clients and SDKs\n\
                        - Documentation generators\n\
                        - Testing tools\n\
                        - Development tools and IDEs",
                    )
                    .summary("Get OpenAPI specification")
                    .tag("Documentation")
                },
            ),
        )
        .route(
            "/documentation",
            Swagger::new("/storage/openapi.json")
                .with_title("From The Hart Storage API")
                .axum_route(),
        );
    ApiRouter::new()
        .nest("/storage", storage_router)
        .finish_api_with(&mut api, create_api_docs)
        .layer(Extension(api))
}
