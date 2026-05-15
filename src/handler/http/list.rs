use crate::{
    handler::http::{
        dto::{StorageListData, StorageListResponse, ViewLink},
        error::HttpError,
    },
    service::file::list,
    state::AppState,
};
use aide::{axum::IntoApiResponse, transform::TransformOperation};
use axum::extract::{Query, State};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema, Clone, Default)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    pub limit: i32,
    pub cursor: Option<String>,
}

fn default_limit() -> i32 {
    50
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PathParams {
    pub user_id: String,
    #[serde(default)]
    pub path: Option<String>,
}

use crate::handler::http::dto::FileResponse;

pub async fn handle_file_request(
    State(state): State<AppState>,
    Path(path_params): Path<PathParams>,
    Query(params): Query<ListParams>,
) -> impl IntoApiResponse {
    let repo = &state.dynamo_db_repository;
    let path = path_params.path.as_deref().unwrap_or("");

    // Strict Strategy:
    // - Ends with '/' or is empty -> Folder Listing
    // - Otherwise -> File Retrieval

    if path.is_empty() || path.ends_with('/') {
        // Folder Listing
        match list::list_folder_contents(
            &path_params.user_id,
            path,
            params.limit,
            params.cursor,
            repo.as_ref(),
        )
        .await
        {
            Ok((items, next_cursor)) => {
                let response = StorageListResponse {
                    data: StorageListData {
                        items: items.into_iter().map(ViewLink::from).collect(),
                        next_cursor,
                    },
                };
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(err) => {
                let http_error = HttpError::from(err);
                http_error.into_response()
            }
        }
    } else {
        // File Retrieval
        let cf_domain = match &state.cloudfront_domain {
            Some(d) => d.clone(),
            None => {
                return HttpError::from(crate::error::StorageError::NotInitialized {
                    context: "CloudFront domain not configured".into(),
                })
                .into_response();
            }
        };
        match crate::service::file::get::get_file(&path_params.user_id, path, repo.as_ref()).await {
            Ok(Some(file)) => {
                let response = FileResponse::from_file(file, &cf_domain);
                (StatusCode::OK, Json(response)).into_response()
            }
            Ok(None) => {
                // File not found. Since strict strategy, we return 404.
                // We don't fall back to folder listing.
                (
                    StatusCode::NOT_FOUND,
                    Json(crate::handler::http::error::HttpErrorResponse {
                        error: crate::handler::http::error::ErrorData {
                            code: "not_found".to_string(),
                            message: "File not found".to_string(),
                            details: None,
                        },
                    }),
                )
                    .into_response()
            }
            Err(err) => {
                let http_error = HttpError::from(err);
                http_error.into_response()
            }
        }
    }
}

pub fn handle_file_request_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Get file details or list folder contents.\n\n\
        - **Folder Listing**: Request path must end with a trailing slash (e.g., `/storage/{user_id}/foo/`). Returns a list of files and folders.\n\
        - **File Retrieval**: Request path must NOT have a trailing slash (e.g., `/storage/{user_id}/foo`). Returns file metadata.\n\n\
        If a folder is requested without a trailing slash, it will be treated as a file request and return 404 if no such file exists.",
    )
    .summary("Get file or list folder")
    .tag("Storage")
    .response::<200, Json<StorageListResponse>>()
    .response::<200, Json<FileResponse>>()
    .response::<400, Json<crate::handler::http::error::HttpErrorResponse>>()
    .response::<404, Json<crate::handler::http::error::HttpErrorResponse>>()
    .response::<500, Json<crate::handler::http::error::HttpErrorResponse>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::{MockDynamoDbRepository, MockMetadataService};
    use crate::state::AppState;
    use axum::http::StatusCode;
    use std::sync::Arc;

    fn create_test_state() -> (AppState, MockDynamoDbRepository) {
        let dynamodb_mock = MockDynamoDbRepository::new();
        let metadata_mock = MockMetadataService::new();
        let state = AppState::new(
            None,
            Arc::new(dynamodb_mock.clone()),
            Some(Arc::new(metadata_mock)),
            None,
        );
        (state, dynamodb_mock)
    }

    #[tokio::test]
    async fn test_handle_file_request_get_file_success() {
        let (mut state, _dynamodb_mock) = create_test_state();
        state.cloudfront_domain = Some("test.cloudfront.net".into());

        // MockDynamoDbRepository returns Ok(None) by default for get_file — testing NOT_FOUND path.

        let path_params = PathParams {
            user_id: "user1".into(),
            path: Some("test.jpg".into()),
        };
        let query_params = ListParams::default();

        let response = handle_file_request(State(state), Path(path_params), Query(query_params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_handle_file_request_folder_list_success() {
        let (state, _dynamodb_mock) = create_test_state();
        // MockDynamoDbRepository returns empty list by default

        let path_params = PathParams {
            user_id: "user1".into(),
            path: Some("folder/".into()),
        };
        let query_params = ListParams::default();

        let response = handle_file_request(State(state), Path(path_params), Query(query_params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
