use crate::error::StorageError;
use crate::repository::SsmRepositoryTrait;

use async_trait::async_trait;
use aws_sdk_ssm::Client;
use std::sync::Arc;
use tokio::sync::OnceCell;

static SSM_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

async fn get_ssm_client() -> Arc<Client> {
    SSM_CLIENT
        .get_or_init(|| async {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Arc::new(Client::new(&config))
        })
        .await
        .clone()
}

pub struct SsmRepository {
    client: Arc<Client>,
}

impl SsmRepository {
    pub async fn new() -> Self {
        Self {
            client: get_ssm_client().await,
        }
    }
}

#[async_trait]
impl SsmRepositoryTrait for SsmRepository {
    async fn get_parameter(
        &self,
        path: &str,
        with_decryption: bool,
    ) -> Result<String, StorageError> {
        let response = self
            .client
            .get_parameter()
            .name(path)
            .with_decryption(with_decryption)
            .send()
            .await
            .map_err(|e| StorageError::Ssm {
                context: format!("Failed to get SSM parameter: {}", path),
                source: e.into(),
            })?;

        let parameter = response.parameter.ok_or_else(|| StorageError::Ssm {
            context: format!("SSM parameter not found: {}", path),
            source: anyhow::anyhow!("Parameter is None"),
        })?;

        let value = parameter.value.ok_or_else(|| StorageError::Ssm {
            context: format!("SSM parameter value is empty: {}", path),
            source: anyhow::anyhow!("Parameter value is None"),
        })?;

        Ok(value)
    }
}
