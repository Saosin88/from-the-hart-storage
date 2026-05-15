use crate::{
    handler::http::{
        access, folder, health, list,
        openapi::create_api_docs,
    },
    state::AppState,
};

use aide::{
    axum::{routing, ApiRouter},
    openapi::OpenApi,
    swagger::Swagger,
};
use axum::{response::IntoResponse, Extension, Json, Router};

pub fn configure_routes(state: AppState) -> Router {
    let mut api = OpenApi::default();
    let storage_router = ApiRouter::new()
        .api_route(
            "/health",
            routing::get_with(health::health, health::health_docs),
        )
        .api_route(
            "/access",
            routing::get_with(access::get_signed_access, access::get_signed_access_docs),
        )
        .api_route(
            "/folders",
            routing::post_with(folder::create_folder, folder::create_folder_docs),
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
            "/documentation/openapi.json",
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
            Swagger::new("/storage/documentation/openapi.json")
                .with_title("From The Hart Storage API")
                .axum_route(),
        );
    ApiRouter::new()
        .nest("/storage", storage_router)
        .finish_api_with(&mut api, create_api_docs)
        .layer(Extension(api))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_openapi_json_endpoint_exists() {
        let app = configure_routes(create_test_state());
        let server = TestServer::new(app);
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_openapi_json_contains_wildcard_file_routes() {
        let app = configure_routes(create_test_state());
        let server = TestServer::new(app);
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let openapi: serde_json::Value = response.json();

        assert!(
            openapi.get("paths").is_some(),
            "OpenAPI spec should have 'paths' section"
        );
        let paths = openapi["paths"]
            .as_object()
            .expect("paths should be an object");

        let has_wildcard_route = paths.contains_key("/storage/{user_id}/{path}")
            || paths.contains_key("/storage/{user_id}")
            || paths
                .iter()
                .any(|(k, _)| k.contains("{user_id}") && k.contains("{path}"));

        assert!(
            has_wildcard_route,
            "OpenAPI spec should document wildcard file/folder routes (/{{user_id}}/{{{{path}}}})"
        );
    }

    #[tokio::test]
    async fn test_openapi_json_wildcard_route_has_query_params() {
        let app = configure_routes(create_test_state());
        let server = TestServer::new(app);
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let openapi: serde_json::Value = response.json();
        let paths = openapi["paths"]
            .as_object()
            .expect("paths should be an object");

        let wildcard_route = paths
            .iter()
            .find(|(k, _)| k.contains("{user_id}") && (k.contains("{path}") || k.ends_with("}")))
            .expect("Should find wildcard route in OpenAPI spec");

        let get_operation = wildcard_route
            .1
            .get("get")
            .expect("Wildcard route should have GET method documented");

        let parameters = get_operation
            .get("parameters")
            .expect("GET operation should have parameters");

        let params_array = parameters
            .as_array()
            .expect("parameters should be an array");

        let has_limit = params_array
            .iter()
            .any(|p| p.get("name").and_then(|n| n.as_str()) == Some("limit"));
        let has_cursor = params_array
            .iter()
            .any(|p| p.get("name").and_then(|n| n.as_str()) == Some("cursor"));

        assert!(has_limit, "Should document 'limit' query parameter");
        assert!(has_cursor, "Should document 'cursor' query parameter");
    }

    #[tokio::test]
    async fn test_openapi_json_has_required_endpoints() {
        let app = configure_routes(create_test_state());
        let server = TestServer::new(app);
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let openapi: serde_json::Value = response.json();
        let paths = openapi["paths"]
            .as_object()
            .expect("paths should be an object");

        assert!(
            paths.contains_key("/storage/health"),
            "Should have /health endpoint"
        );
        assert!(
            paths.contains_key("/storage/access"),
            "Should have /access endpoint"
        );
        assert!(
            paths.contains_key("/storage/folders"),
            "Should have /folders endpoint"
        );
    }
}
