use thiserror::Error;

#[derive(Error, Debug)]
pub enum OvError {
    #[error("Context not found: {uri}")]
    ContextNotFound { uri: String },
    #[error("Collection not found: {name}")]
    CollectionNotFound { name: String },
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Embedding error: {0}")]
    Embedding(String),
    #[error("Transaction error: {0}")]
    Transaction(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OvError>;
