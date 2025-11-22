use crate::{
    handler::http::{
        dto::{StorageListData, StorageListResponse, ViewLink},
        error::HttpError,
    },
    repository::dynamodb::DynamoDbRepository,
    service::file::list,
};
use aide::{axum::IntoApiResponse, transform::TransformOperation};
use axum::extract::Query;
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
    Path(path_params): Path<PathParams>,
    Query(params): Query<ListParams>,
) -> impl IntoApiResponse {
    let repo = DynamoDbRepository::new().await;
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
            &repo,
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
        match crate::service::file::get::get_file(&path_params.user_id, path, &repo).await {
            Ok(Some(file)) => {
                let response = FileResponse::from(file);
                (StatusCode::OK, Json(response)).into_response()
            }
            Ok(None) => {
                // File not found. Since strict strategy, we return 404.
                // We don't fall back to folder listing.
                (StatusCode::NOT_FOUND, Json(crate::handler::http::error::HttpErrorResponse {
                    error: crate::handler::http::error::ErrorData {
                        code: "not_found".to_string(),
                        message: "File not found".to_string(),
                        details: None,
                    }
                })).into_response()
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
    use crate::handler::http::routes;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_list_files_endpoint_structure() {
        crate::utils::time::init_start_time();

        std::env::set_var("APP_ENVIRONMENT", "test");
        std::env::set_var("APP_DYNAMODB_TABLE", "test-table");
        let _ = crate::config::init_config();

        let app = routes::configure_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/storage/sheldon/files/").await;
        assert_ne!(
            response.status_code(),
            StatusCode::NOT_FOUND,
            "Should match storage/sheldon/files/ (Folder listing)"
        );

        let response = server.get("/storage/sheldon/files").await;
        assert_ne!(
            response.status_code(),
            StatusCode::NOT_FOUND,
            "Should match /storage/sheldon/files (File retrieval)"
        );

        let response = server.get("/storage/sheldon/").await;
        assert_ne!(
            response.status_code(),
            StatusCode::NOT_FOUND,
            "Should match /storage/sheldon/ (Root folder listing)"
        );

        let response = server.get("/storage/sheldon").await;
        assert_ne!(
            response.status_code(),
            StatusCode::NOT_FOUND,
            "Should match /storage/sheldon"
        );
    }
}
