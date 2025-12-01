use crate::{
    handler::http::storage::{access, folder, health, list},
    state::AppState,
};

use aide::{
    axum::{routing, ApiRouter},
    openapi::OpenApi,
    swagger::Swagger,
    transform::TransformOpenApi,
};
use axum::{response::IntoResponse, Extension, Json, Router};

fn create_api_docs(api: TransformOpenApi) -> TransformOpenApi {
    use aide::openapi::{Operation, Parameter, ParameterData, ParameterSchemaOrContent, PathItem, PathStyle, QueryStyle, ReferenceOr, SchemaObject};
    use serde_json::json;

    let mut transformed = api.title("From The Hart Storage API")
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
        );

    let openapi = transformed.inner_mut();
    let paths = openapi.paths.get_or_insert_with(Default::default);

    paths.paths.insert(
        "/storage/{user_id}/{path}".to_string(),
        ReferenceOr::Item(PathItem {
            get: Some(Operation {
                summary: Some("Get file metadata or list folder contents".to_string()),
                description: Some(
                    "Retrieves file metadata or lists folder contents based on path.\n\n\
                    **Path Behavior:**\n\
                    - Path WITHOUT trailing slash (e.g., `/user123/folder/file.jpg`) → Returns file metadata\n\
                    - Path WITH trailing slash (e.g., `/user123/folder/`) → Returns folder contents listing\n\
                    - Empty path (e.g., `/user123` or `/user123/`) → Returns root folder contents\n\n\
                    **Note:** The `path` parameter can be empty or contain multiple segments separated by slashes.\n\n\
                    **Pagination:** Use `limit` and `cursor` query parameters for folder listings.".to_string()
                ),
                operation_id: Some("get_file_or_list_folder".to_string()),
                parameters: vec![
                    ReferenceOr::Item(Parameter::Path {
                        parameter_data: ParameterData {
                            name: "user_id".to_string(),
                            description: Some("User identifier".to_string()),
                            required: true,
                            deprecated: None,
                            format: ParameterSchemaOrContent::Schema(SchemaObject {
                                json_schema: json!({"type": "string"}).try_into().unwrap(),
                                external_docs: None,
                                example: None,
                            }),
                            example: None,
                            examples: Default::default(),
                            explode: None,
                            extensions: Default::default(),
                        },
                        style: PathStyle::Simple,
                    }),
                    ReferenceOr::Item(Parameter::Path {
                        parameter_data: ParameterData {
                            name: "path".to_string(),
                            description: Some("File or folder path. Can be empty for root folder, or contain multiple segments (e.g., 'folder/subfolder/file.jpg'). Trailing slash indicates folder listing.".to_string()),
                            required: false,
                            deprecated: None,
                            format: ParameterSchemaOrContent::Schema(SchemaObject {
                                json_schema: json!({"type": "string"}).try_into().unwrap(),
                                external_docs: None,
                                example: Some(json!("folder/subfolder/")),
                            }),
                            example: None,
                            examples: Default::default(),
                            explode: None,
                            extensions: Default::default(),
                        },
                        style: PathStyle::Simple,
                    }),
                    ReferenceOr::Item(Parameter::Query {
                        parameter_data: ParameterData {
                            name: "limit".to_string(),
                            description: Some("Maximum number of items to return for folder listings".to_string()),
                            required: false,
                            deprecated: None,
                            format: ParameterSchemaOrContent::Schema(SchemaObject {
                                json_schema: json!({"type": "integer"}).try_into().unwrap(),
                                external_docs: None,
                                example: Some(json!(50)),
                            }),
                            example: None,
                            examples: Default::default(),
                            explode: None,
                            extensions: Default::default(),
                        },
                        allow_reserved: false,
                        style: QueryStyle::Form,
                        allow_empty_value: None,
                    }),
                    ReferenceOr::Item(Parameter::Query {
                        parameter_data: ParameterData {
                            name: "cursor".to_string(),
                            description: Some("Pagination cursor for retrieving next page of folder contents".to_string()),
                            required: false,
                            deprecated: None,
                            format: ParameterSchemaOrContent::Schema(SchemaObject {
                                json_schema: json!({"type": "string"}).try_into().unwrap(),
                                external_docs: None,
                                example: None,
                            }),
                            example: None,
                            examples: Default::default(),
                            explode: None,
                            extensions: Default::default(),
                        },
                        allow_reserved: false,
                        style: QueryStyle::Form,
                        allow_empty_value: None,
                    }),
                ],
                responses: None,
                tags: vec!["Storage".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        }),
    );

    transformed
}

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
        let server = TestServer::new(app).unwrap();
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_openapi_json_contains_wildcard_file_routes() {
        let app = configure_routes(create_test_state());
        let server = TestServer::new(app).unwrap();
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let openapi: serde_json::Value = response.json();

        assert!(openapi.get("paths").is_some(), "OpenAPI spec should have 'paths' section");
        let paths = openapi["paths"].as_object().expect("paths should be an object");

        let has_wildcard_route = paths.contains_key("/storage/{user_id}/{path}")
            || paths.contains_key("/storage/{user_id}")
            || paths.iter().any(|(k, _)| k.contains("{user_id}") && k.contains("{path}"));

        assert!(
            has_wildcard_route,
            "OpenAPI spec should document wildcard file/folder routes (/{{user_id}}/{{{{path}}}})"
        );
    }

    #[tokio::test]
    async fn test_openapi_json_wildcard_route_has_query_params() {
        let app = configure_routes(create_test_state());
        let server = TestServer::new(app).unwrap();
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let openapi: serde_json::Value = response.json();
        let paths = openapi["paths"].as_object().expect("paths should be an object");

        let wildcard_route = paths.iter()
            .find(|(k, _)| k.contains("{user_id}") && (k.contains("{path}") || k.ends_with("}")))
            .expect("Should find wildcard route in OpenAPI spec");

        let get_operation = wildcard_route.1.get("get")
            .expect("Wildcard route should have GET method documented");

        let parameters = get_operation.get("parameters")
            .expect("GET operation should have parameters");

        let params_array = parameters.as_array()
            .expect("parameters should be an array");

        let has_limit = params_array.iter().any(|p| {
            p.get("name").and_then(|n| n.as_str()) == Some("limit")
        });
        let has_cursor = params_array.iter().any(|p| {
            p.get("name").and_then(|n| n.as_str()) == Some("cursor")
        });

        assert!(has_limit, "Should document 'limit' query parameter");
        assert!(has_cursor, "Should document 'cursor' query parameter");
    }

    #[tokio::test]
    async fn test_openapi_json_has_required_endpoints() {
        let app = configure_routes(create_test_state());
        let server = TestServer::new(app).unwrap();
        let response = server.get("/storage/documentation/openapi.json").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let openapi: serde_json::Value = response.json();
        let paths = openapi["paths"].as_object().expect("paths should be an object");

        assert!(paths.contains_key("/storage/health"), "Should have /health endpoint");
        assert!(paths.contains_key("/storage/access"), "Should have /access endpoint");
        assert!(paths.contains_key("/storage/folders"), "Should have /folders endpoint");
    }
}
