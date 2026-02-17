use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Semantic extraction queue message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMsg {
    pub id: String,
    pub uri: String,
    pub context_type: String,
    pub status: SemanticStatus,
    pub timestamp: i64,
    pub recursive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SemanticStatus {
    Pending,
    Processing,
    Completed,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Init,
    Acquire,
    Exec,
    Commit,
    Fail,
    Releasing,
    Released,
}

/// Transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub id: String,
    pub locks: Vec<String>,
    pub status: TransactionStatus,
    pub init_info: HashMap<String, serde_json::Value>,
    pub rollback_info: HashMap<String, serde_json::Value>,
    pub created_at: f64,
    pub updated_at: f64,
}

/// Embedding result (dense + optional sparse)
#[derive(Debug, Clone, Default)]
pub struct EmbedResult {
    pub dense_vector: Option<Vec<f32>>,
    pub sparse_vector: Option<HashMap<String, f32>>,
}

/// Directory definition for preset structure
#[derive(Debug, Clone)]
pub struct DirectoryDefinition {
    pub path: String,
    pub abstract_text: String,
    pub overview: String,
    pub children: Vec<DirectoryDefinition>,
}
