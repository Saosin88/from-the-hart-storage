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

    let token_data = insecure_decode::<Claims>(token).map_err(|e| {
        StorageError::JwtParse {
            context: "Failed to decode JWT".to_string(),
            source: anyhow::Error::new(e),
        }
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
