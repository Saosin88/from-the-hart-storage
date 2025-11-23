use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, Utc};
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::{pkcs1v15::SigningKey, signature::{Signer, SignatureEncoding}, RsaPrivateKey};
use sha1::Sha1;
use std::sync::Arc;
use tracing::{error, info};

use crate::config::config;
use crate::error::StorageError;
use crate::repository::SsmRepositoryTrait;

#[derive(Clone)]
pub struct CloudFrontSigner {
    private_key: RsaPrivateKey,
    key_pair_id: String,
    domain: String,
}

pub struct SignedAccess {
    pub resource_pattern: String,
    pub expires_at: i64,
    pub query_params: String,
    pub policy: String,
    pub signature: String,
    pub key_pair_id: String,
}

impl CloudFrontSigner {
    pub fn new(
        private_key_pem: &str,
        key_pair_id: String,
        domain: String,
    ) -> Result<Self, StorageError> {
        let private_key = RsaPrivateKey::from_pkcs1_pem(private_key_pem).map_err(|e| {
            StorageError::CloudFrontSigning {
                context: "Failed to parse RSA private key".to_string(),
                source: anyhow::Error::new(e),
            }
        })?;

        Ok(Self {
            private_key,
            key_pair_id,
            domain,
        })
    }

    pub async fn from_ssm_config<T: SsmRepositoryTrait>(ssm_repo: &T) -> Option<Arc<Self>> {
        let cf_config = match config().cloudfront.as_ref() {
            Some(config) => config,
            None => {
                info!("CloudFront configuration not provided, signed URLs will not be available");
                return None;
            }
        };

        info!("Initializing CloudFront signer from SSM parameter store");

        let private_key_pem = match ssm_repo
            .get_parameter(&cf_config.private_key_ssm_path, true)
            .await
        {
            Ok(key) => key,
            Err(e) => {
                error!(
                    error = ?e,
                    path = %cf_config.private_key_ssm_path,
                    "Failed to fetch private key from SSM"
                );
                return None;
            }
        };

        match Self::new(
            &private_key_pem,
            cf_config.key_pair_id.clone(),
            cf_config.domain.clone(),
        ) {
            Ok(signer) => {
                info!("CloudFront signer initialized successfully");
                Some(Arc::new(signer))
            }
            Err(e) => {
                error!(error = ?e, "Failed to create CloudFront signer");
                None
            }
        }
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn sign_user_directory(&self, user_id: &str) -> Result<SignedAccess, StorageError> {
        let expiration = Utc::now() + Duration::hours(1);
        let expiration_epoch = expiration.timestamp();

        let resource_pattern = format!("https://{}{}/*", self.domain, Self::ensure_leading_slash(user_id));

        let custom_policy = format!(
            r#"{{"Statement":[{{"Resource":"{}","Condition":{{"DateLessThan":{{"AWS:EpochTime":{}}}}}}}]}}"#,
            resource_pattern, expiration_epoch
        );

        let signing_key = SigningKey::<Sha1>::new(self.private_key.clone());
        let signature = signing_key.sign(custom_policy.as_bytes());

        let cloudfront_safe_signature = Self::cloudfront_safe_base64(&signature.to_bytes());
        let cloudfront_safe_policy = Self::cloudfront_safe_base64(custom_policy.as_bytes());

        let query_params = format!(
            "Policy={}&Signature={}&Key-Pair-Id={}",
            cloudfront_safe_policy, cloudfront_safe_signature, self.key_pair_id
        );

        Ok(SignedAccess {
            resource_pattern,
            expires_at: expiration_epoch,
            query_params,
            policy: cloudfront_safe_policy,
            signature: cloudfront_safe_signature,
            key_pair_id: self.key_pair_id.clone(),
        })
    }

    fn ensure_leading_slash(user_id: &str) -> String {
        if user_id.starts_with('/') {
            user_id.to_string()
        } else {
            format!("/{}", user_id)
        }
    }

    fn cloudfront_safe_base64(data: &[u8]) -> String {
        STANDARD
            .encode(data)
            .replace('+', "-")
            .replace('=', "_")
            .replace('/', "~")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TEST_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEA18of7JuLeuvtlt/UHE+6sEZy5LKivMO9bNx0kPYzrzP+tQ8l
MyMMwnTeph5o9Czedfa9c8QLn2UkEYvt6pRni7AhAFVZdArSbJy8CTz5HNZtC5SW
Cb8ZdvQuWAXaQ4FctUCjdexCzfMRVWl6YxArgkuI9KKcH0CoJvqOxeRuVp6Vc1EM
4sZyovVkYcqE3+A54OCANoKdg5dRrs3dvOZxFiZsWJvAhV4P5nD3VF73sAiLvP+A
WMQkHju3Kvqu/V/WdijzxqTl68FvYLNxx9Kghm7XTVpXzbu795ncMKEmZ5drcjvo
wMRpt534P351PuGj9Y30Rwmy7X7f9/PWF6JI0QIDAQABAoIBAGHWYNcURw86fQSV
A0S62Xgm2NzcKXDQhsgexDMdjZ14Q5mv/jvLPnPELVbAHvHmjg6KCqe8UVC5uLrD
6OEc3D23Y58VE3PLnFBmV9MQdBohFlvTyJsuq8sFNyXtsWI9+tyrK/HBZyCdflRt
eHIF2NRAjx3rxEFfPV4+6BDNW0Gl28NDQev0dtyU+5OoIe/o8jLQa8ygrk3650pE
kYnTJcdeSN+1pyrCnP4B06abqgv6k3nCmd9lUPsztJ9zjbe/AG5FsxwVQcA1C8Oa
PS3r82aBsg03Qj5l9z1hJAILylBFoMs6ecipGVmQSXV41Ig9sCttblFP+BPfozZP
4M1IvKkCgYEA+HTcTQItyioeQ2nPPnwB+SACxD+zs4DAnvj+/cyyWSzw7+pmvEQL
Bye2myD25ayN08n2gyF7sLaiRAItZk4K4TdG9TeOZ9uwOcPwciXxZ0WbnTPcGQnw
CPSRG7APhyOzPxJ/vLtNbDfyDPWGOR5flRDbZaWOKr4pEhBkgGGeDAsCgYEA3ldb
4AUSJXk0GwS8JPHKseTN9m/VjqWgSgFDemkFWWslmNgn09DBrRadrHEHvmmD/lW3
TOi3iQ87tw54tFUjkpZqMWjHxAZk0G09H+v2i02RCcJLF1fpOA67EB274vKhdpSJ
c91Ti82lCP9FZRwyyta6BTUqmoCzvO639KxwrBMCgYEAk/Ny5GCxx7tA/j/Z64mI
20MWoqqUZgX7ri70GUp1weijKRraRq32KzKY6NO+cpJIep+/reKYd2iqQ/lP86Xx
kJ+MH6YPpQULcbqeSjsR/79RpVEmdbqXN537cxNqi7zUlnB7pHWc6x59gv4KCaVu
oaPCIktt10IZzun4DwMSTHUCgYAjUgdjWArg7xcq756f09VaWzmo2202gvMqrna0
vHhAEzhexn/VM0WBJKWZnj8XrZVtUtqSmimF2WioFOFx7FCBWem2val2Z3mebqwW
JRr+WC0hOr9JDwsaf6SR09dkHx0tRD1trYw3Gk0MV9kDTe53sJLOfvqsnqNu8RFC
Ch7ABwKBgHug7wZoH40PGsIR1TrDpU2N6biy7ONPd2vuecjQnFJnvqlhgCbqJciV
sXbddBS4f4fzTagTGqoyPQSfPKGmJQRPOdXMsLjSjxRNN+Qg1IvdPiQLtniGZJ9Q
r8su2oorFXPtUXJUxWtqNGVTVYxznN7rCe+SK5gdJKW7aSNjSEFT
-----END RSA PRIVATE KEY-----";

    #[test]
    fn test_cloudfront_safe_base64() {
        let input = b"test+data/with=special";
        let result = CloudFrontSigner::cloudfront_safe_base64(input);
        assert!(!result.contains('+'));
        assert!(!result.contains('='));
        assert!(!result.contains('/'));
    }

    #[test]
    fn test_ensure_leading_slash() {
        assert_eq!(CloudFrontSigner::ensure_leading_slash("user123"), "/user123");
        assert_eq!(CloudFrontSigner::ensure_leading_slash("/user123"), "/user123");
    }

    #[test]
    fn test_new_valid_key() {
        let result = CloudFrontSigner::new(
            VALID_TEST_KEY,
            "K1234567890ABC".to_string(),
            "example.com".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_invalid_key() {
        let result = CloudFrontSigner::new(
            "invalid-key",
            "K1234567890ABC".to_string(),
            "example.com".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_new_empty_key() {
        let result = CloudFrontSigner::new(
            "",
            "K1234567890ABC".to_string(),
            "example.com".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_user_directory() {
        let signer = CloudFrontSigner::new(
            VALID_TEST_KEY,
            "K1234567890ABC".to_string(),
            "example.com".to_string(),
        )
        .unwrap();

        let result = signer.sign_user_directory("user123");
        assert!(result.is_ok());

        let signed = result.unwrap();
        assert_eq!(signed.resource_pattern, "https://example.com/user123/*");
        assert!(signed.expires_at > 0);
        assert!(!signed.signature.is_empty());
        assert!(!signed.policy.is_empty());
        assert_eq!(signed.key_pair_id, "K1234567890ABC");
        assert!(signed.query_params.contains("Policy="));
        assert!(signed.query_params.contains("Signature="));
        assert!(signed.query_params.contains("Key-Pair-Id="));
    }

    #[test]
    fn test_sign_user_directory_with_leading_slash() {
        let signer = CloudFrontSigner::new(
            VALID_TEST_KEY,
            "K1234567890ABC".to_string(),
            "example.com".to_string(),
        )
        .unwrap();

        let result = signer.sign_user_directory("/user123");
        assert!(result.is_ok());

        let signed = result.unwrap();
        assert_eq!(signed.resource_pattern, "https://example.com/user123/*");
    }
}
