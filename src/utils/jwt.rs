//! JWT utility for extracting user identity from pre-verified tokens.
//!
//! **Security Note:** This service uses `jsonwebtoken::dangerous::insecure_decode`
//! to extract the `user_id` claim WITHOUT verifying the JWT signature. This is
//! intentional and safe because:
//!
//! 1. The Cloudflare Worker API Gateway (the sole entry point) validates all JWT
//!    signatures BEFORE forwarding requests to this service.
//! 2. This service is not publicly accessible — all traffic flows through the gateway.
//! 3. Signature verification here would be redundant and add latency.
//!
//! If this service is ever exposed directly, signature verification MUST be added.

use jsonwebtoken::dangerous::insecure_decode;
use serde::{Deserialize, Serialize};

use crate::error::StorageError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
}

pub fn extract_user_id_from_jwt(auth_header: &str) -> Result<String, StorageError> {
    if !auth_header.starts_with("Bearer ") {
        return Err(StorageError::InvalidRequest {
            context: "JWT extraction".to_string(),
            source: anyhow::anyhow!("Authorization header must start with 'Bearer '"),
        });
    }

    let token = &auth_header[7..];

    // SAFETY: token signature verified by Cloudflare Worker API Gateway before reaching this service
    let token_data = insecure_decode::<Claims>(token).map_err(|e| StorageError::JwtParse {
        context: "Failed to decode JWT".to_string(),
        source: anyhow::Error::new(e),
    })?;

    Ok(token_data.claims.user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_user_id_missing_bearer() {
        let result = extract_user_id_from_jwt("token123");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_invalid_jwt() {
        let result = extract_user_id_from_jwt("Bearer invalid.jwt.token");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_success() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoidGVzdC11c2VyLTEyMyIsImV4cCI6OTk5OTk5OTk5OX0.fake_signature";
        let auth_header = format!("Bearer {}", token);

        let result = extract_user_id_from_jwt(&auth_header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-user-123");
    }
}
