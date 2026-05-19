use aide::{
    openapi::{
        MediaType, Operation, Parameter, ParameterData, ParameterSchemaOrContent, PathItem,
        PathStyle, QueryStyle, ReferenceOr, Response, Responses, SchemaObject, StatusCode,
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
    let components = openapi.components.get_or_insert_with(Default::default);

    // Register schemas needed for the wildcard file route responses
    register_storage_schemas(components);

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
                responses: Some(Responses {
                    default: None,
                    responses: [
                        (
                            StatusCode::Code(200),
                            ReferenceOr::Item(json_response(
                                "Folder listing or file metadata",
                                "#/components/schemas/DataResponse_StorageList",
                                "#/components/schemas/DataResponse_File",
                            )),
                        ),
                        (
                            StatusCode::Code(400),
                            ReferenceOr::Item(error_response("Invalid request parameters")),
                        ),
                        (
                            StatusCode::Code(404),
                            ReferenceOr::Item(error_response("File not found")),
                        ),
                        (
                            StatusCode::Code(500),
                            ReferenceOr::Item(error_response("Internal server error")),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    extensions: Default::default(),
                }),
                tags: vec!["Storage".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        }),
    );

    transformed
}

/// Build a response with a single JSON schema reference.
fn json_response(desc: &str, primary_ref: &str, alt_ref: &str) -> Response {
    let content = [(
        "application/json".to_string(),
        MediaType {
            schema: Some(SchemaObject {
                json_schema: json!({"oneOf": [{"$ref": primary_ref}, {"$ref": alt_ref}]})
                    .try_into()
                    .unwrap(),
                external_docs: None,
                example: None,
            }),
            example: None,
            examples: Default::default(),
            encoding: Default::default(),
            extensions: Default::default(),
        },
    )]
    .into_iter()
    .collect();

    Response {
        description: desc.to_string(),
        headers: Default::default(),
        content,
        links: Default::default(),
        extensions: Default::default(),
    }
}

/// Build an error response.
fn error_response(desc: &str) -> Response {
    let content = [(
        "application/json".to_string(),
        MediaType {
            schema: Some(SchemaObject {
                json_schema: json!({"$ref": "#/components/schemas/HttpErrorResponse"})
                    .try_into()
                    .unwrap(),
                external_docs: None,
                example: None,
            }),
            example: None,
            examples: Default::default(),
            encoding: Default::default(),
            extensions: Default::default(),
        },
    )]
    .into_iter()
    .collect();

    Response {
        description: desc.to_string(),
        headers: Default::default(),
        content,
        links: Default::default(),
        extensions: Default::default(),
    }
}

/// Register schemas for the wildcard file/folder route responses.
fn register_storage_schemas(components: &mut aide::openapi::Components) {
    // DataResponse<StorageListData>
    components.schemas.insert(
        "DataResponse_StorageList".to_string(),
        SchemaObject {
            json_schema: json!({
                "type": "object",
                "properties": {
                    "data": {"$ref": "#/components/schemas/StorageListData"}
                },
                "required": ["data"]
            })
            .try_into()
            .unwrap(),
            external_docs: None,
            example: None,
        },
    );

    // StorageListData
    components.schemas.insert(
        "StorageListData".to_string(),
        SchemaObject {
            json_schema: json!({
                "type": "object",
                "description": "List of files and folders",
                "properties": {
                    "items": {
                        "type": "array",
                        "items": {"$ref": "#/components/schemas/ViewLinkData"},
                        "description": "List of files and folders"
                    },
                    "next_cursor": {
                        "type": "string",
                        "nullable": true,
                        "description": "Cursor for pagination"
                    }
                },
                "required": ["items"]
            })
            .try_into()
            .unwrap(),
            external_docs: None,
            example: None,
        },
    );

    // ViewLinkData
    components.schemas.insert(
        "ViewLinkData".to_string(),
        SchemaObject {
            json_schema: json!({
                "type": "object",
                "description": "A link to a file or folder",
                "properties": {
                    "viewer_id": {"type": "string"},
                    "resource_id": {"type": "string"},
                    "owner_id": {"type": "string"},
                    "grant_id": {"type": "string"},
                    "created_date": {"type": "integer", "format": "int64"},
                    "folder_prefix": {"type": "string"},
                    "name": {"type": "string"},
                    "media_type": {"type": "string"},
                    "size_bytes": {"type": "integer", "format": "int64"},
                    "is_folder": {"type": "boolean"}
                },
                "required": ["viewer_id", "resource_id", "owner_id", "grant_id",
                    "created_date", "folder_prefix", "name", "media_type", "size_bytes", "is_folder"]
            })
            .try_into()
            .unwrap(),
            external_docs: None,
            example: None,
        },
    );

    // DataResponse<FileData>
    components.schemas.insert(
        "DataResponse_File".to_string(),
        SchemaObject {
            json_schema: json!({
                "type": "object",
                "properties": {
                    "data": {"$ref": "#/components/schemas/FileData"}
                },
                "required": ["data"]
            })
            .try_into()
            .unwrap(),
            external_docs: None,
            example: None,
        },
    );

    // FileData
    components.schemas.insert(
        "FileData".to_string(),
        SchemaObject {
            json_schema: json!({
                "type": "object",
                "description": "Detailed file information",
                "properties": {
                    "file_url": {"type": "string", "description": "Signed URL to access the file"},
                    "owner_id": {"type": "string", "description": "Owner user ID"},
                    "file_id": {"type": "string", "description": "Unique file identifier (SHA-256 hash)"},
                    "file_name": {"type": "string", "description": "Original file name"},
                    "file_path": {"type": "string", "description": "Full path of the file in storage"},
                    "folder_prefix": {"type": "string", "description": "Parent folder prefix"},
                    "created_date": {"type": "integer", "format": "int64", "description": "File creation timestamp in milliseconds since UNIX epoch"},
                    "size_bytes": {"type": "integer", "format": "int64", "description": "File size in bytes"},
                    "content_type": {"type": "string", "description": "MIME content type of the file"},
                    "media_type": {"type": "string", "description": "Media type category (e.g., Image, Video, Document)"},
                    "media_metadata": {"oneOf": [{"type": "null"}, {"type": "object"}], "description": "Optional metadata extracted from the file (e.g., EXIF data for images)"}
                },
                "required": ["file_url", "owner_id", "file_id", "file_name", "file_path",
                    "folder_prefix", "created_date", "size_bytes", "content_type", "media_type"]
            })
            .try_into()
            .unwrap(),
            external_docs: None,
            example: None,
        },
    );
}
