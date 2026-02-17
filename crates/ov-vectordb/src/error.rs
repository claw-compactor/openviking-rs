use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorDbError {
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),
    #[error("Collection already exists: {0}")]
    CollectionAlreadyExists(String),
    #[error("Index not found: {0}")]
    IndexNotFound(String),
    #[error("Index already exists: {0}")]
    IndexAlreadyExists(String),
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
    #[error("Project already exists: {0}")]
    ProjectAlreadyExists(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, VectorDbError>;
