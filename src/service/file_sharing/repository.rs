use aws_sdk_dynamodb::{
    types::{AttributeValue, WriteRequest},
    Client as DynamoDbClient,
};
use std::collections::HashMap;

use super::models::*;
use crate::error::ServiceError;

/// Repository for file sharing operations in DynamoDB
pub struct FileShareRepository {
    client: DynamoDbClient,
    table_name: String,
}

impl FileShareRepository {
    pub fn new(client: DynamoDbClient, table_name: String) -> Self {
        Self { client, table_name }
    }

    /// Put a file item in DynamoDB
    pub async fn put_file(&self, file: &FileItem) -> Result<(), ServiceError> {
        let item = self.file_to_item(file)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| ServiceError::InternalServerError(format!("DynamoDB put_file error: {}", e)))?;

        Ok(())
    }

    /// Get a file item by owner and path
    pub async fn get_file(
        &self,
        owner_id: &str,
        file_path: &str,
    ) -> Result<Option<FileItem>, ServiceError> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("USER#{}", owner_id)))
            .key("SK", AttributeValue::S(format!("FILE#{}", file_path)))
            .send()
            .await
            .map_err(|e| ServiceError::InternalServerError(format!("DynamoDB get_file error: {}", e)))?;

        match result.item {
            Some(item) => Ok(Some(self.item_to_file(item)?)),
            None => Ok(None),
        }
    }

    /// Query files in a folder prefix (owner's own files)
    pub async fn query_files_by_prefix(
        &self,
        owner_id: &str,
        prefix: &str,
    ) -> Result<Vec<FileItem>, ServiceError> {
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
            .expression_attribute_values(
                ":sk_prefix",
                AttributeValue::S(format!("FILE#{}", prefix)),
            )
            .send()
            .await
            .map_err(|e| {
                ServiceError::InternalServerError(format!("DynamoDB query_files error: {}", e))
            })?;

        let items = result.items.unwrap_or_default();
        items
            .into_iter()
            .map(|item| self.item_to_file(item))
            .collect()
    }

    /// Create a share grant
    pub async fn create_share_grant(&self, grant: &ShareGrantItem) -> Result<(), ServiceError> {
        let item = self.share_grant_to_item(grant)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| {
                ServiceError::InternalServerError(format!("DynamoDB create_grant error: {}", e))
            })?;

        Ok(())
    }

    /// Delete a share grant
    pub async fn delete_share_grant(
        &self,
        owner_id: &str,
        recipient_id: &str,
        grant_id: &str,
    ) -> Result<(), ServiceError> {
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("USER#{}", owner_id)))
            .key(
                "SK",
                AttributeValue::S(format!("GRANT#{}#{}", recipient_id, grant_id)),
            )
            .send()
            .await
            .map_err(|e| {
                ServiceError::InternalServerError(format!("DynamoDB delete_grant error: {}", e))
            })?;

        Ok(())
    }

    /// Query grants for a recipient (GSI1: ShareAccessIndex)
    pub async fn query_grants_for_recipient(
        &self,
        recipient_id: &str,
    ) -> Result<Vec<ShareGrantItem>, ServiceError> {
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("ShareAccessIndex")
            .key_condition_expression("GSI1PK = :pk")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(format!("ACCESS#{}", recipient_id)),
            )
            .send()
            .await
            .map_err(|e| {
                ServiceError::InternalServerError(format!("DynamoDB query_grants error: {}", e))
            })?;

        let items = result.items.unwrap_or_default();
        items
            .into_iter()
            .map(|item| self.item_to_share_grant(item))
            .collect()
    }

    /// Find grants that match a specific prefix (for finding who has access)
    pub async fn query_grants_for_prefix(
        &self,
        owner_id: &str,
        prefix: &str,
    ) -> Result<Vec<ShareGrantItem>, ServiceError> {
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("GRANT#".to_string()))
            .filter_expression("Prefix = :prefix")
            .expression_attribute_values(":prefix", AttributeValue::S(prefix.to_string()))
            .send()
            .await
            .map_err(|e| {
                ServiceError::InternalServerError(format!(
                    "DynamoDB query_grants_for_prefix error: {}",
                    e
                ))
            })?;

        let items = result.items.unwrap_or_default();
        items
            .into_iter()
            .map(|item| self.item_to_share_grant(item))
            .collect()
    }

    /// Create view links (batch operation)
    pub async fn create_view_links(
        &self,
        view_links: Vec<ViewLinkItem>,
    ) -> Result<(), ServiceError> {
        // DynamoDB batch write supports up to 25 items per request
        for chunk in view_links.chunks(25) {
            let write_requests: Vec<WriteRequest> = chunk
                .iter()
                .map(|link| {
                    let item = self.view_link_to_item(link).unwrap();
                    WriteRequest::builder()
                        .put_request(
                            aws_sdk_dynamodb::types::PutRequest::builder()
                                .set_item(Some(item))
                                .build()
                                .unwrap(),
                        )
                        .build()
                })
                .collect();

            let mut request_items = HashMap::new();
            request_items.insert(self.table_name.clone(), write_requests);

            self.client
                .batch_write_item()
                .set_request_items(Some(request_items))
                .send()
                .await
                .map_err(|e| {
                    ServiceError::InternalServerError(format!(
                        "DynamoDB batch_write_view_links error: {}",
                        e
                    ))
                })?;
        }

        Ok(())
    }

    /// Delete view links for a specific grant (used during revocation)
    pub async fn delete_view_links_for_grant(
        &self,
        viewer_id: &str,
        grant_id: &str,
    ) -> Result<(), ServiceError> {
        // Query to find all view links with the grant ID
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(format!("USER#{}", viewer_id)),
            )
            .expression_attribute_values(":sk_prefix", AttributeValue::S("VIEWLINK#".to_string()))
            .filter_expression("GrantID = :grant_id")
            .expression_attribute_values(":grant_id", AttributeValue::S(grant_id.to_string()))
            .send()
            .await
            .map_err(|e| {
                ServiceError::InternalServerError(format!(
                    "DynamoDB query_view_links error: {}",
                    e
                ))
            })?;

        let items = result.items.unwrap_or_default();

        // Delete in batches of 25
        for chunk in items.chunks(25) {
            let write_requests: Vec<WriteRequest> = chunk
                .iter()
                .map(|item| {
                    let pk = item.get("PK").unwrap().as_s().unwrap().clone();
                    let sk = item.get("SK").unwrap().as_s().unwrap().clone();

                    WriteRequest::builder()
                        .delete_request(
                            aws_sdk_dynamodb::types::DeleteRequest::builder()
                                .key("PK", AttributeValue::S(pk))
                                .key("SK", AttributeValue::S(sk))
                                .build()
                                .unwrap(),
                        )
                        .build()
                })
                .collect();

            let mut request_items = HashMap::new();
            request_items.insert(self.table_name.clone(), write_requests);

            self.client
                .batch_write_item()
                .set_request_items(Some(request_items))
                .send()
                .await
                .map_err(|e| {
                    ServiceError::InternalServerError(format!(
                        "DynamoDB batch_delete_view_links error: {}",
                        e
                    ))
                })?;
        }

        Ok(())
    }

    /// Query merged folder view (GSI2: MergedFolderViewIndex)
    pub async fn query_merged_folder_view(
        &self,
        viewer_id: &str,
        folder_name: &str,
        media_type_prefix: Option<&str>,
        limit: Option<i32>,
        last_evaluated_key: Option<HashMap<String, AttributeValue>>,
    ) -> Result<(Vec<ViewLinkItem>, Option<HashMap<String, AttributeValue>>), ServiceError> {
        let mut query = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("MergedFolderViewIndex")
            .key_condition_expression("GSI2PK = :pk")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(format!("VIEWER#{}#FOLDER#{}", viewer_id, folder_name)),
            )
            .scan_index_forward(false); // Newest first

        // Add media type filter if specified
        if let Some(media_prefix) = media_type_prefix {
            query = query
                .key_condition_expression("GSI2PK = :pk AND begins_with(GSI2SK, :sk_prefix)")
                .expression_attribute_values(
                    ":sk_prefix",
                    AttributeValue::S(format!("{}#", media_prefix)),
                );
        }

        // Add pagination
        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        if let Some(key) = last_evaluated_key {
            query = query.set_exclusive_start_key(Some(key));
        }

        let result = query.send().await.map_err(|e| {
            ServiceError::InternalServerError(format!("DynamoDB query_merged_view error: {}", e))
        })?;

        let items = result.items.unwrap_or_default();
        let view_links: Result<Vec<ViewLinkItem>, ServiceError> = items
            .into_iter()
            .map(|item| self.item_to_view_link(item))
            .collect();

        Ok((view_links?, result.last_evaluated_key))
    }

    // Helper methods to convert between structs and DynamoDB items

    fn file_to_item(&self, file: &FileItem) -> Result<HashMap<String, AttributeValue>, ServiceError> {
        let json = serde_json::to_value(file).map_err(|e| {
            ServiceError::InternalServerError(format!("Failed to serialize file: {}", e))
        })?;

        self.json_to_dynamodb_item(json)
    }

    fn item_to_file(&self, item: HashMap<String, AttributeValue>) -> Result<FileItem, ServiceError> {
        let json = self.dynamodb_item_to_json(item)?;
        serde_json::from_value(json).map_err(|e| {
            ServiceError::InternalServerError(format!("Failed to deserialize file: {}", e))
        })
    }

    fn share_grant_to_item(
        &self,
        grant: &ShareGrantItem,
    ) -> Result<HashMap<String, AttributeValue>, ServiceError> {
        let json = serde_json::to_value(grant).map_err(|e| {
            ServiceError::InternalServerError(format!("Failed to serialize grant: {}", e))
        })?;

        self.json_to_dynamodb_item(json)
    }

    fn item_to_share_grant(
        &self,
        item: HashMap<String, AttributeValue>,
    ) -> Result<ShareGrantItem, ServiceError> {
        let json = self.dynamodb_item_to_json(item)?;
        serde_json::from_value(json).map_err(|e| {
            ServiceError::InternalServerError(format!("Failed to deserialize grant: {}", e))
        })
    }

    fn view_link_to_item(
        &self,
        link: &ViewLinkItem,
    ) -> Result<HashMap<String, AttributeValue>, ServiceError> {
        let json = serde_json::to_value(link).map_err(|e| {
            ServiceError::InternalServerError(format!("Failed to serialize view link: {}", e))
        })?;

        self.json_to_dynamodb_item(json)
    }

    fn item_to_view_link(
        &self,
        item: HashMap<String, AttributeValue>,
    ) -> Result<ViewLinkItem, ServiceError> {
        let json = self.dynamodb_item_to_json(item)?;
        serde_json::from_value(json).map_err(|e| {
            ServiceError::InternalServerError(format!("Failed to deserialize view link: {}", e))
        })
    }

    fn json_to_dynamodb_item(
        &self,
        json: serde_json::Value,
    ) -> Result<HashMap<String, AttributeValue>, ServiceError> {
        match json {
            serde_json::Value::Object(map) => {
                let mut item = HashMap::new();
                for (key, value) in map {
                    item.insert(key, self.json_value_to_attribute_value(value)?);
                }
                Ok(item)
            }
            _ => Err(ServiceError::InternalServerError(
                "Expected JSON object".to_string(),
            )),
        }
    }

    fn json_value_to_attribute_value(
        &self,
        value: serde_json::Value,
    ) -> Result<AttributeValue, ServiceError> {
        match value {
            serde_json::Value::String(s) => Ok(AttributeValue::S(s)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(AttributeValue::N(i.to_string()))
                } else if let Some(f) = n.as_f64() {
                    Ok(AttributeValue::N(f.to_string()))
                } else {
                    Err(ServiceError::InternalServerError(
                        "Invalid number".to_string(),
                    ))
                }
            }
            serde_json::Value::Bool(b) => Ok(AttributeValue::Bool(b)),
            serde_json::Value::Null => Ok(AttributeValue::Null(true)),
            serde_json::Value::Object(map) => {
                let mut m = HashMap::new();
                for (k, v) in map {
                    m.insert(k, self.json_value_to_attribute_value(v)?);
                }
                Ok(AttributeValue::M(m))
            }
            serde_json::Value::Array(arr) => {
                let list: Result<Vec<AttributeValue>, ServiceError> = arr
                    .into_iter()
                    .map(|v| self.json_value_to_attribute_value(v))
                    .collect();
                Ok(AttributeValue::L(list?))
            }
        }
    }

    fn dynamodb_item_to_json(
        &self,
        item: HashMap<String, AttributeValue>,
    ) -> Result<serde_json::Value, ServiceError> {
        let mut map = serde_json::Map::new();
        for (key, value) in item {
            map.insert(key, self.attribute_value_to_json(value)?);
        }
        Ok(serde_json::Value::Object(map))
    }

    fn attribute_value_to_json(
        &self,
        value: AttributeValue,
    ) -> Result<serde_json::Value, ServiceError> {
        match value {
            AttributeValue::S(s) => Ok(serde_json::Value::String(s)),
            AttributeValue::N(n) => {
                if let Ok(i) = n.parse::<i64>() {
                    Ok(serde_json::Value::Number(i.into()))
                } else if let Ok(f) = n.parse::<f64>() {
                    Ok(serde_json::Value::Number(
                        serde_json::Number::from_f64(f).unwrap(),
                    ))
                } else {
                    Err(ServiceError::InternalServerError(
                        "Invalid number".to_string(),
                    ))
                }
            }
            AttributeValue::Bool(b) => Ok(serde_json::Value::Bool(b)),
            AttributeValue::Null(_) => Ok(serde_json::Value::Null),
            AttributeValue::M(m) => {
                let mut map = serde_json::Map::new();
                for (k, v) in m {
                    map.insert(k, self.attribute_value_to_json(v)?);
                }
                Ok(serde_json::Value::Object(map))
            }
            AttributeValue::L(l) => {
                let arr: Result<Vec<serde_json::Value>, ServiceError> = l
                    .into_iter()
                    .map(|v| self.attribute_value_to_json(v))
                    .collect();
                Ok(serde_json::Value::Array(arr?))
            }
            _ => Err(ServiceError::InternalServerError(
                "Unsupported attribute type".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_dynamodb_conversion() {
        let config = aws_sdk_dynamodb::Config::builder()
            .behavior_version(aws_sdk_dynamodb::config::BehaviorVersion::latest())
            .build();
        let repo = FileShareRepository {
            client: DynamoDbClient::from_conf(config),
            table_name: "test".to_string(),
        };

        let json = serde_json::json!({
            "PK": "USER#test",
            "SK": "FILE#test.jpg",
            "Size": 1024
        });

        let item = repo.json_to_dynamodb_item(json).unwrap();
        assert!(matches!(item.get("PK"), Some(AttributeValue::S(_))));
        assert!(matches!(item.get("Size"), Some(AttributeValue::N(_))));
    }
}
