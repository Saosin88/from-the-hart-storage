use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::DataResponse;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "CloudFront signed access data for secure file downloads")]
pub struct SignedAccessData {
    #[schemars(
        description = "Resource pattern that this signature grants access to (e.g., /{user_id}/*)"
    )]
    pub resource_pattern: String,
    #[schemars(
        description = "Expiration timestamp (UNIX epoch seconds) when this signature expires"
    )]
    pub expires_at: i64,
    #[schemars(
        description = "Query parameters to append to file URLs for signed access (Policy, Signature, Key-Pair-Id)"
    )]
    pub query_params: String,
}

pub type SignedAccessResponse = DataResponse<SignedAccessData>;
