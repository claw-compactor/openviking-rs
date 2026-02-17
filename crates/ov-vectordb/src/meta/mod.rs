//! Metadata management for collections and indexes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::store::FileStore;

/// Volatile (in-memory) metadata dictionary.
#[derive(Debug, Clone, Default)]
pub struct VolatileDict {
    pub data: HashMap<String, serde_json::Value>,
}

impl VolatileDict {
    pub fn new(data: HashMap<String, serde_json::Value>) -> Self {
        Self { data }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.data.insert(key, value);
    }

    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
    }

    pub fn override_all(&mut self, data: HashMap<String, serde_json::Value>) {
        self.data = data;
    }
}

/// Persistent metadata dictionary backed by a JSON file.
#[derive(Debug, Clone)]
pub struct PersistentDict {
    path: PathBuf,
    data: HashMap<String, serde_json::Value>,
}

impl PersistentDict {
    pub fn new(path: PathBuf, initial: HashMap<String, serde_json::Value>) -> Self {
        let mut dict = Self {
            path: path.clone(),
            data: initial,
        };
        // Try to load existing data
        let store = FileStore::default();
        if let Some(bytes) = store.get(path.to_str().unwrap_or("")) {
            if let Ok(existing) = serde_json::from_slice::<HashMap<String, serde_json::Value>>(&bytes) {
                // Merge: existing data takes precedence for loaded keys
                for (k, v) in existing {
                    dict.data.insert(k, v);
                }
            }
        }
        dict
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.data.insert(key, value);
        self.persist();
    }

    pub fn override_all(&mut self, data: HashMap<String, serde_json::Value>) {
        self.data = data;
        self.persist();
    }

    pub fn data(&self) -> &HashMap<String, serde_json::Value> {
        &self.data
    }

    pub fn drop_file(&self) {
        let store = FileStore::default();
        store.delete(self.path.to_str().unwrap_or(""));
    }

    fn persist(&self) {
        let store = FileStore::default();
        if let Ok(bytes) = serde_json::to_vec(&self.data) {
            store.put(self.path.to_str().unwrap_or(""), &bytes);
        }
    }
}

/// Collection metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMeta {
    pub collection_name: String,
    pub primary_key: String,
    pub vector_key: String,
    pub dimension: usize,
    pub fields: Vec<FieldMeta>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMeta {
    pub name: String,
    pub field_type: String,
    pub is_primary_key: bool,
    #[serde(default)]
    pub dim: Option<usize>,
}

/// Index metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMeta {
    pub index_name: String,
    pub index_type: String,
    pub distance: String,
    #[serde(default)]
    pub scalar_index_fields: Vec<String>,
    #[serde(default)]
    pub description: String,
}
