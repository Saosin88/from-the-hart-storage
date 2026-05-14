use aide::{
    openapi::{
        Operation, Parameter, ParameterData, ParameterSchemaOrContent, PathItem, PathStyle,
        QueryStyle, ReferenceOr, SchemaObject,
    },
    transform::TransformOpenApi,
};
use serde_json::json;

pub fn create_api_docs(api: TransformOpenApi) -> TransformOpenApi {
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
