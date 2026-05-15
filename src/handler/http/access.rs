use crate::{
    error::StorageError,
    handler::http::{
        dto::{SignedAccessData, SignedAccessResponse},
        error::HttpError,
    },
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
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};

const COOKIE_DOMAIN: &str = ".fromthehart.tech";

pub async fn get_signed_access(
    State(state): State<AppState>,
    headers: HeaderMap,
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

    let cloudfront_signer = match state.cloudfront_signer.as_ref() {
        Some(signer) => signer,
        None => {
            let error = StorageError::NotInitialized {
                context: "CloudFront signer not configured".to_string(),
            };
            return HttpError::from(error).into_response();
        }
    };

    let signed_access = match cloudfront_signer.sign_user_directory(&user_id) {
        Ok(access) => access,
        Err(e) => return HttpError::from(e).into_response(),
    };

    let response_data = SignedAccessData {
        resource_pattern: signed_access.resource_pattern,
        expires_at: signed_access.expires_at,
        query_params: signed_access.query_params,
    };

    let now = chrono::Utc::now().timestamp();
    let max_age = (signed_access.expires_at - now).max(0);

    let mut response = (
        StatusCode::OK,
        Json(SignedAccessResponse {
            data: response_data,
        }),
    )
        .into_response();

    let response_headers = response.headers_mut();

    response_headers.insert(
        header::SET_COOKIE,
        format!(
            "CloudFront-Policy={}; Domain={}; Path=/; Secure; HttpOnly; SameSite=None; Max-Age={}",
            signed_access.policy, COOKIE_DOMAIN, max_age
        )
        .parse()
        .unwrap(),
    );

    response_headers.append(
        header::SET_COOKIE,
        format!(
            "CloudFront-Signature={}; Domain={}; Path=/; Secure; HttpOnly; SameSite=None; Max-Age={}",
            signed_access.signature, COOKIE_DOMAIN, max_age
        )
        .parse()
        .unwrap(),
    );

    response_headers.append(
        header::SET_COOKIE,
        format!(
            "CloudFront-Key-Pair-Id={}; Domain={}; Path=/; Secure; HttpOnly; SameSite=None; Max-Age={}",
            signed_access.key_pair_id, COOKIE_DOMAIN, max_age
        )
        .parse()
        .unwrap(),
    );

    response
}

pub fn get_signed_access_docs(mut op: TransformOperation) -> TransformOperation {
    op = op.description(
        "Generate CloudFront signed access for user files.\n\n\
        This endpoint generates CloudFront signed URLs and sets signed cookies that grant access to all files \
        in the authenticated user's directory. The signature is valid for 1 hour.\n\n\
        **Authentication:** Requires a valid JWT token in the Authorization header (Bearer token).\n\n\
        **Usage:**\n\
        1. Call this endpoint to get signed access credentials\n\
        2. Use the returned `query_params` to append to any file URL for signed access\n\
        3. Alternatively, the browser will automatically send the signed cookies with subsequent requests\n\n\
        **Response includes:**\n\
        - Query parameters for signed URLs\n\
        - Three Set-Cookie headers (CloudFront-Policy, CloudFront-Signature, CloudFront-Key-Pair-Id)\n\n\
        **Security:**\n\
        - Cookies are set with Secure, HttpOnly, and SameSite=None attributes\n\
        - Access is limited to the authenticated user's directory pattern: `/{user_id}/*`\n\
        - Signatures expire after 1 hour",
    )
    .summary("Get CloudFront signed access")
    .tag("Storage")
    .response::<200, Json<SignedAccessResponse>>()
    .response::<401, Json<crate::handler::http::error::HttpErrorResponse>>()
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
    use axum::http::StatusCode;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_signed_access_missing_auth_header() {
        let dynamodb_mock = MockDynamoDbRepository::new();
        let metadata_mock = MockMetadataService::new();
        let state = AppState::new(
            None,
            Arc::new(dynamodb_mock),
            Some(Arc::new(metadata_mock)),
            None,
            None,
        );

        let headers = HeaderMap::new();

        let response = get_signed_access(State(state), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_signed_access_cloudfront_not_configured() {
        let dynamodb_mock = MockDynamoDbRepository::new();
        let metadata_mock = MockMetadataService::new();
        let state = AppState::new(
            None,
            Arc::new(dynamodb_mock),
            Some(Arc::new(metadata_mock)),
            None,
            None,
        );

        let mut headers = HeaderMap::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoidGVzdC11c2VyLTEyMyIsImV4cCI6OTk5OTk5OTk5OX0.fake_signature";
        headers.insert(
            "X-From-The-Hart-Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );

        let response = get_signed_access(State(state), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
