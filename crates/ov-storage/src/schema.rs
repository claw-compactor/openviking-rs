//! Collection schema definitions for OpenViking.
//!
//! Port of `openviking/storage/collection_schemas.py`.

use serde::{Deserialize, Serialize};

/// Field type in a collection schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// String field.
    String,
    /// Dense vector field.
    Vector,
    /// Sparse vector field.
    SparseVector,
    /// Path / URI field.
    Path,
    /// Boolean field.
    Bool,
    /// 64-bit integer.
    Int64,
    /// Date-time field.
    DateTime,
}

impl FieldType {
    /// Return the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Vector => "vector",
            Self::SparseVector => "sparse_vector",
            Self::Path => "path",
            Self::Bool => "bool",
            Self::Int64 => "int64",
            Self::DateTime => "date_time",
        }
    }
}

/// A field definition in a collection schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: FieldType,
    /// Whether this is the primary key.
    #[serde(default)]
    pub is_primary_key: bool,
    /// Vector dimension (only for Vector type).
    #[serde(default)]
    pub dimension: Option<usize>,
}

/// Collection schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSchema {
    /// Collection name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Field definitions.
    pub fields: Vec<FieldDef>,
    /// Fields to create scalar indexes on.
    pub scalar_index: Vec<String>,
}

/// Build the default context collection schema.
pub fn context_collection_schema(name: &str, vector_dim: usize) -> CollectionSchema {
    CollectionSchema {
        name: name.to_string(),
        description: "Unified context collection".to_string(),
        fields: vec![
            FieldDef { name: "id".into(), field_type: FieldType::String, is_primary_key: true, dimension: None },
            FieldDef { name: "uri".into(), field_type: FieldType::Path, is_primary_key: false, dimension: None },
            FieldDef { name: "type".into(), field_type: FieldType::String, is_primary_key: false, dimension: None },
            FieldDef { name: "context_type".into(), field_type: FieldType::String, is_primary_key: false, dimension: None },
            FieldDef { name: "vector".into(), field_type: FieldType::Vector, is_primary_key: false, dimension: Some(vector_dim) },
            FieldDef { name: "sparse_vector".into(), field_type: FieldType::SparseVector, is_primary_key: false, dimension: None },
            FieldDef { name: "created_at".into(), field_type: FieldType::DateTime, is_primary_key: false, dimension: None },
            FieldDef { name: "updated_at".into(), field_type: FieldType::DateTime, is_primary_key: false, dimension: None },
            FieldDef { name: "active_count".into(), field_type: FieldType::Int64, is_primary_key: false, dimension: None },
            FieldDef { name: "parent_uri".into(), field_type: FieldType::Path, is_primary_key: false, dimension: None },
            FieldDef { name: "is_leaf".into(), field_type: FieldType::Bool, is_primary_key: false, dimension: None },
            FieldDef { name: "name".into(), field_type: FieldType::String, is_primary_key: false, dimension: None },
            FieldDef { name: "description".into(), field_type: FieldType::String, is_primary_key: false, dimension: None },
            FieldDef { name: "tags".into(), field_type: FieldType::String, is_primary_key: false, dimension: None },
            FieldDef { name: "abstract".into(), field_type: FieldType::String, is_primary_key: false, dimension: None },
        ],
        scalar_index: vec![
            "uri", "type", "context_type", "created_at", "updated_at",
            "active_count", "parent_uri", "is_leaf", "name", "tags",
        ].into_iter().map(String::from).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_collection_schema() {
        let schema = context_collection_schema("test", 1024);
        assert_eq!(schema.name, "test");
        assert_eq!(schema.fields.len(), 15);
    }

    #[test]
    fn test_schema_has_primary_key() {
        let schema = context_collection_schema("ctx", 512);
        let pk = schema.fields.iter().find(|f| f.is_primary_key).unwrap();
        assert_eq!(pk.name, "id");
    }

    #[test]
    fn test_schema_vector_dim() {
        let schema = context_collection_schema("ctx", 768);
        let vec_field = schema.fields.iter().find(|f| f.name == "vector").unwrap();
        assert_eq!(vec_field.dimension, Some(768));
    }

    #[test]
    fn test_schema_scalar_index() {
        let schema = context_collection_schema("ctx", 1024);
        assert!(schema.scalar_index.contains(&"uri".to_string()));
        assert!(schema.scalar_index.contains(&"context_type".to_string()));
        assert_eq!(schema.scalar_index.len(), 10);
    }

    #[test]
    fn test_field_type_as_str() {
        assert_eq!(FieldType::String.as_str(), "string");
        assert_eq!(FieldType::Vector.as_str(), "vector");
        assert_eq!(FieldType::Bool.as_str(), "bool");
        assert_eq!(FieldType::Int64.as_str(), "int64");
    }

    #[test]
    fn test_schema_serde_roundtrip() {
        let schema = context_collection_schema("rt", 256);
        let json = serde_json::to_string(&schema).unwrap();
        let schema2: CollectionSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema2.name, "rt");
        assert_eq!(schema2.fields.len(), 15);
    }

    #[test]
    fn test_field_def_serde() {
        let f = FieldDef {
            name: "test".into(),
            field_type: FieldType::Vector,
            is_primary_key: false,
            dimension: Some(1024),
        };
        let json = serde_json::to_string(&f).unwrap();
        let f2: FieldDef = serde_json::from_str(&json).unwrap();
        assert_eq!(f2.dimension, Some(1024));
    }
}
