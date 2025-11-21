use crate::{
    handler::http::{
        dto::{StorageListResponse, ViewLink},
        error::HttpError,
    },
    repository::dynamodb::DynamoDbRepository,
    service::file::list,
};
use aide::{axum::IntoApiResponse, transform::TransformOperation};
use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum::extract::Query;
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
    pub path: String,
}

pub async fn list_files(
    Path(path_params): Path<PathParams>,
    Query(params): Query<ListParams>,
) -> impl IntoApiResponse {
    let repo = DynamoDbRepository::new().await;
    
    match list::list_folder_contents(&path_params.user_id, &path_params.path, params.limit, params.cursor, &repo).await {
        Ok((items, next_cursor)) => {
            let response = StorageListResponse {
                items: items.into_iter().map(ViewLink::from).collect(),
                next_cursor,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            let http_error = HttpError::from(err);
            http_error.into_response()
        }
    }
}

pub fn list_files_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "List files and folders in a specific path for a user.\n\n\
        This endpoint returns a paginated list of files and folders contained within the specified path.\n\
        Results are sorted with folders first, then files, both in descending order of creation.\n\
        Use the `cursor` parameter from the response to fetch the next page of results.",
    )
    .summary("List files and folders")
    .tag("Storage")
    .response::<200, Json<StorageListResponse>>()
    .response::<400, Json<crate::handler::http::error::HttpErrorResponse>>()
    .response::<500, Json<crate::handler::http::error::HttpErrorResponse>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::http::routes;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_list_files_endpoint_structure() {
        crate::utils::time::init_start_time();
        
        // Initialize config for DynamoDBRepository
        std::env::set_var("APP_ENVIRONMENT", "test");
        std::env::set_var("APP_DYNAMODB_TABLE", "test-table");
        let _ = crate::config::init_config();

        let app = routes::configure_routes();
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/storage/sheldon/files/").await;
        
        // If the route matches, it shouldn't be 404.
        assert_ne!(response.status_code(), StatusCode::NOT_FOUND);
    }
}
