use crate::{
    error::StorageError,
    handler::http::{
        dto::{CreateFolderRequest, CreateFolderResponse, FolderData},
        error::HttpError,
    },
    service::folder::create,
    state::AppState,
    utils::jwt::extract_user_id_from_jwt,
};
use aide::{axum::IntoApiResponse, transform::TransformOperation};
use aide::openapi::{
    HeaderStyle, Parameter, ParameterData, ParameterSchemaOrContent, ReferenceOr, SchemaObject,
};
use serde_json::json;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};

pub async fn create_folder(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateFolderRequest>,
) -> impl IntoApiResponse {
    let auth_header = match headers
        .get("X-From-The-Hart-Authorization")
        .and_then(|h| h.to_str().ok())
    {
        Some(h) => h,
        None => {
            let error = StorageError::InvalidRequest {
                context: "Missing Authorization header".to_string(),
                source: anyhow::anyhow!("Authorization header is required"),
            };
            return HttpError::from(error).into_response();
        }
    };

    let user_id = match extract_user_id_from_jwt(auth_header) {
        Ok(id) => id,
        Err(e) => return HttpError::from(e).into_response(),
    };

    let view_link = match create::create_folder(
        state.dynamo_db_repository.as_ref(),
        &user_id,
        &body.path,
    )
    .await
    {
        Ok(vl) => vl,
        Err(e) => return HttpError::from(e).into_response(),
    };

    let folder_data = FolderData::from(view_link);

    (
        StatusCode::OK,
        Json(CreateFolderResponse { data: folder_data }),
    )
        .into_response()
}

pub fn create_folder_docs(mut op: TransformOperation) -> TransformOperation {
    op = op.description(
        "Create a new folder in the user's storage.\n\n\
        This endpoint creates a new folder at the specified path. The folder path must:\n\
        - End with a trailing slash (e.g., 'media/')\n\
        - Not start with a leading slash\n\
        - Not contain '//' sequences\n\
        - Not contain '.' or '..' path segments\n\n\
        **Authentication:** Requires a valid JWT token in the X-From-The-Hart-Authorization header.\n\n\
        **Parent Folder Requirements:**\n\
        - Parent folder must exist before creating nested folders\n\
        - Root-level folders (e.g., 'media/') can be created without a parent\n\n\
        **Idempotent Operation:**\n\
        - Creating an existing folder returns the folder metadata without error\n\n\
        **Examples:**\n\
        - Root folder: {\"path\": \"media/\"}\n\
        - Nested folder: {\"path\": \"media/photos/\"} (requires 'media/' to exist)",
    )
    .summary("Create a new folder")
    .tag("Storage")
    .response::<200, Json<CreateFolderResponse>>()
    .response::<400, Json<crate::handler::http::error::HttpErrorResponse>>()
    .response::<401, Json<crate::handler::http::error::HttpErrorResponse>>()
    .response::<404, Json<crate::handler::http::error::HttpErrorResponse>>()
    .response::<500, Json<crate::handler::http::error::HttpErrorResponse>>();

    // Add the auth header parameter manually (not auto-detected since handler uses HeaderMap)
    {
        let operation = op.inner_mut();
        operation.parameters.push(ReferenceOr::Item(Parameter::Header {
            parameter_data: ParameterData {
                name: "X-From-The-Hart-Authorization".to_string(),
                description: Some(
                    "JWT Bearer token. Validated by the API Gateway before reaching this service."
                        .to_string(),
                ),
                required: true,
                deprecated: None,
                format: ParameterSchemaOrContent::Schema(SchemaObject {
                    json_schema: json!({"type": "string", "pattern": "^Bearer .+$"})
                        .try_into()
                        .unwrap(),
                    external_docs: None,
                    example: Some(json!("Bearer eyJhbGciOiJIUzI1NiIs...")),
                }),
                example: None,
                examples: Default::default(),
                explode: None,
                extensions: Default::default(),
            },
            style: HeaderStyle::Simple,
        }));
    }

    op
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::{MockDynamoDbRepository, MockMetadataService};
    use crate::service::models::ViewLink;
    use crate::utils::time;
    use axum::http::StatusCode;
    use std::sync::Arc;

    fn create_test_state(mock_repo: MockDynamoDbRepository) -> AppState {
        let metadata_mock = MockMetadataService::new();
        AppState::new(
            None,
            Arc::new(mock_repo),
            Some(Arc::new(metadata_mock)),
            None,
        )
    }

    fn mock_view_link(
        user_id: &str,
        folder_path: &str,
        folder_name: &str,
        parent_path: &str,
    ) -> ViewLink {
        use crate::service::models::ResourceId;
        ViewLink {
            viewer_id: user_id.into(),
            resource_id: ResourceId::Folder(folder_path.into()),
            owner_id: user_id.into(),
            grant_id: "OWNER".into(),
            created_date: time::now_as_unix_millis(),
            folder_prefix: parent_path.into(),
            name: folder_name.into(),
            media_type: "Folder".into(),
            size_bytes: 0,
        }
    }

    #[tokio::test]
    async fn test_create_folder_missing_auth_header() {
        let mock_repo = MockDynamoDbRepository::new();
        let state = create_test_state(mock_repo);

        let headers = HeaderMap::new();
        let body = CreateFolderRequest {
            path: "media/".to_string(),
        };

        let response = create_folder(State(state), headers, Json(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_folder_invalid_jwt() {
        let mock_repo = MockDynamoDbRepository::new();
        let state = create_test_state(mock_repo);

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-From-The-Hart-Authorization",
            "Bearer invalid_token".parse().unwrap(),
        );

        let body = CreateFolderRequest {
            path: "media/".to_string(),
        };

        let response = create_folder(State(state), headers, Json(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_create_folder_invalid_path_no_trailing_slash() {
        let mock_repo = MockDynamoDbRepository::new();
        let state = create_test_state(mock_repo);

        let mut headers = HeaderMap::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoidGVzdC11c2VyLTEyMyIsImV4cCI6OTk5OTk5OTk5OX0.fake_signature";
        headers.insert(
            "X-From-The-Hart-Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );

        let body = CreateFolderRequest {
            path: "media".to_string(),
        };

        let response = create_folder(State(state), headers, Json(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_folder_missing_parent() {
        let mock_repo = MockDynamoDbRepository::new().with_folder_exists_response(Ok(false));

        let state = create_test_state(mock_repo);

        let mut headers = HeaderMap::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoidGVzdC11c2VyLTEyMyIsImV4cCI6OTk5OTk5OTk5OX0.fake_signature";
        headers.insert(
            "X-From-The-Hart-Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );

        let body = CreateFolderRequest {
            path: "media/photos/".to_string(),
        };

        let response = create_folder(State(state), headers, Json(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_folder_success() {
        let mock_repo = MockDynamoDbRepository::new()
            .with_folder_exists_response(Ok(false))
            .with_create_folder_response(Ok(mock_view_link(
                "test-user-123",
                "media/",
                "media",
                "",
            )));

        let state = create_test_state(mock_repo);

        let mut headers = HeaderMap::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoidGVzdC11c2VyLTEyMyIsImV4cCI6OTk5OTk5OTk5OX0.fake_signature";
        headers.insert(
            "X-From-The-Hart-Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );

        let body = CreateFolderRequest {
            path: "media/".to_string(),
        };

        let response = create_folder(State(state), headers, Json(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_folder_idempotent() {
        let mock_repo = MockDynamoDbRepository::new()
            .with_folder_exists_response(Ok(true))
            .with_create_folder_response(Ok(mock_view_link(
                "test-user-123",
                "media/",
                "media",
                "",
            )));

        let state = create_test_state(mock_repo);

        let mut headers = HeaderMap::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoidGVzdC11c2VyLTEyMyIsImV4cCI6OTk5OTk5OTk5OX0.fake_signature";
        headers.insert(
            "X-From-The-Hart-Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );

        let body = CreateFolderRequest {
            path: "media/".to_string(),
        };

        let response = create_folder(State(state), headers, Json(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
