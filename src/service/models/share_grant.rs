use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareGrant {
    pub item_type: Option<String>,
    pub grant_id: String,
    pub owner_id: String,
    pub recipient_id: String,
    pub grant_type: Option<String>,
    pub prefix: Option<String>,
    pub resource_id: Option<String>,
    pub file_path: Option<String>,
    pub created_date: Option<i64>,
}
