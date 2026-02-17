//! Collection management: CRUD for vectors with filtering and search.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::distance::DistanceMetric;
use crate::error::{Result, VectorDbError};
use crate::filter::Filter;
use crate::index::{FlatIndex, HnswIndex, VectorIndex};

/// Field type for collection schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Int64,
    Float32,
    String,
    Bool,
    Vector,
    #[serde(rename = "list<string>")]
    ListString,
    #[serde(rename = "list<int64>")]
    ListInt64,
    #[serde(rename = "list<float32>")]
    ListFloat32,
    Path,
    DateTime,
    GeoPoint,
    SparseVector,
}

impl FieldType {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "int64" | "int" | "integer" => Self::Int64,
            "float32" | "float" | "double" => Self::Float32,
            "string" | "str" | "text" => Self::String,
            "bool" | "boolean" => Self::Bool,
            "vector" => Self::Vector,
            "list<string>" => Self::ListString,
            "list<int64>" => Self::ListInt64,
            "list<float32>" => Self::ListFloat32,
            "path" => Self::Path,
            "date_time" | "datetime" => Self::DateTime,
            "geo_point" | "geopoint" => Self::GeoPoint,
            "sparse_vector" => Self::SparseVector,
            _ => Self::String,
        }
    }
}

/// Field definition in collection schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    #[serde(default)]
    pub is_primary_key: bool,
    #[serde(default)]
    pub dim: Option<usize>,
}

/// Collection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub name: String,
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub description: String,
}

impl CollectionConfig {
    pub fn primary_key(&self) -> Option<&str> {
        self.fields.iter()
            .find(|f| f.is_primary_key)
            .map(|f| f.name.as_str())
    }

    pub fn vector_field(&self) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.field_type == FieldType::Vector)
    }

    pub fn dimension(&self) -> usize {
        self.vector_field().and_then(|f| f.dim).unwrap_or(0)
    }
}

/// Search result item.
#[derive(Debug, Clone)]
pub struct SearchItem {
    pub id: Value,
    pub score: f32,
    pub fields: HashMap<String, Value>,
}

/// Collection search result.
#[derive(Debug, Clone, Default)]
pub struct CollectionSearchResult {
    pub data: Vec<SearchItem>,
}

/// Upsert result.
#[derive(Debug, Clone, Default)]
pub struct UpsertResult {
    pub ids: Vec<Value>,
}

/// Index configuration.
#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub index_type: String,  // "flat" or "hnsw"
    pub distance: DistanceMetric,
    pub scalar_index_fields: Vec<String>,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            index_type: "flat".to_string(),
            distance: DistanceMetric::Cosine,
            scalar_index_fields: Vec::new(),
        }
    }
}

/// Internal record stored in the collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    label: u64,
    vector: Vec<f32>,
    fields: HashMap<String, Value>,
}

struct CollectionIndex {
    config: IndexConfig,
    index: Box<dyn VectorIndex>,
}

/// A Collection manages vectors and their metadata with index-backed search.
pub struct Collection {
    config: CollectionConfig,
    /// All records keyed by label.
    records: RwLock<HashMap<u64, Record>>,
    /// Named indexes.
    indexes: RwLock<HashMap<String, CollectionIndex>>,
    /// Auto-increment ID counter.
    next_auto_id: RwLock<u64>,
    /// Optional persistence path.
    path: Option<PathBuf>,
}

impl Collection {
    /// Create a new in-memory collection.
    pub fn new(config: CollectionConfig) -> Self {
        Self {
            config,
            records: RwLock::new(HashMap::new()),
            indexes: RwLock::new(HashMap::new()),
            next_auto_id: RwLock::new(1),
            path: None,
        }
    }

    /// Create a persistent collection.
    pub fn with_path(config: CollectionConfig, path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&path)?;
        let mut coll = Self::new(config);
        coll.path = Some(path.clone());
        // Try to recover
        coll.try_recover()?;
        Ok(coll)
    }

    pub fn config(&self) -> &CollectionConfig {
        &self.config
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get the dimension of vectors in this collection.
    pub fn dimension(&self) -> usize {
        self.config.dimension()
    }

    /// Create a named index.
    pub fn create_index(&self, name: &str, cfg: IndexConfig) -> Result<()> {
        let mut indexes = self.indexes.write();
        if indexes.contains_key(name) {
            return Err(VectorDbError::IndexAlreadyExists(name.to_string()));
        }

        let dim = self.dimension();
        let index: Box<dyn VectorIndex> = match cfg.index_type.as_str() {
            "hnsw" => Box::new(HnswIndex::new(dim, cfg.distance)),
            _ => Box::new(FlatIndex::new(dim, cfg.distance)),
        };

        // Insert all existing records into the new index
        let records = self.records.read();
        for record in records.values() {
            if !record.vector.is_empty() {
                let _ = index.insert(record.label, &record.vector);
            }
        }

        indexes.insert(name.to_string(), CollectionIndex { config: cfg, index });
        Ok(())
    }

    pub fn has_index(&self, name: &str) -> bool {
        self.indexes.read().contains_key(name)
    }

    pub fn list_indexes(&self) -> Vec<String> {
        self.indexes.read().keys().cloned().collect()
    }

    pub fn drop_index(&self, name: &str) {
        self.indexes.write().remove(name);
    }

    /// Upsert data records.
    pub fn upsert_data(&self, data_list: &[HashMap<String, Value>]) -> Result<UpsertResult> {
        let pk_name = self.config.primary_key().map(|s| s.to_string());
        let vk_name = self.config.vector_field().map(|f| f.name.clone());
        let dim = self.dimension();

        let mut result = UpsertResult::default();
        let mut records = self.records.write();
        let indexes = self.indexes.read();

        for data in data_list {
            let label = if let Some(ref pk) = pk_name {
                if let Some(pk_val) = data.get(pk) {
                    value_to_u64(pk_val)
                } else {
                    let mut auto = self.next_auto_id.write();
                    let id = *auto;
                    *auto += 1;
                    id
                }
            } else {
                let mut auto = self.next_auto_id.write();
                let id = *auto;
                *auto += 1;
                id
            };

            let vector = if let Some(ref vk) = vk_name {
                if let Some(vec_val) = data.get(vk) {
                    value_to_f32_vec(vec_val)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            if !vector.is_empty() && vector.len() != dim {
                return Err(VectorDbError::DimensionMismatch {
                    expected: dim,
                    got: vector.len(),
                });
            }

            // Build fields (exclude vector field)
            let mut fields: HashMap<String, Value> = data.clone();
            if let Some(ref vk) = vk_name {
                fields.remove(vk);
            }

            // Update indexes
            if !vector.is_empty() {
                for ci in indexes.values() {
                    let _ = ci.index.insert(label, &vector);
                }
            }

            let id_val = if let Some(ref pk) = pk_name {
                data.get(pk).cloned().unwrap_or(Value::from(label))
            } else {
                Value::from(label)
            };

            records.insert(label, Record { label, vector, fields });
            result.ids.push(id_val);
        }

        Ok(result)
    }

    /// Fetch records by primary keys.
    pub fn fetch_data(&self, primary_keys: &[Value]) -> Vec<Option<HashMap<String, Value>>> {
        let records = self.records.read();
        primary_keys.iter().map(|pk| {
            let label = value_to_u64(pk);
            records.get(&label).map(|r| {
                let mut fields = r.fields.clone();
                // Add vector back
                if let Some(vf) = self.config.vector_field() {
                    fields.insert(vf.name.clone(), Value::from(r.vector.iter().map(|&f| Value::from(f as f64)).collect::<Vec<_>>()));
                }
                fields
            })
        }).collect()
    }

    /// Delete records by primary keys.
    pub fn delete_data(&self, primary_keys: &[Value]) {
        let mut records = self.records.write();
        let indexes = self.indexes.read();
        for pk in primary_keys {
            let label = value_to_u64(pk);
            if records.remove(&label).is_some() {
                for ci in indexes.values() {
                    let _ = ci.index.delete(label);
                }
            }
        }
    }

    /// Delete all data.
    pub fn delete_all_data(&self) {
        let mut records = self.records.write();
        records.clear();
        // Recreate indexes (empty)
        let mut indexes = self.indexes.write();
        let dim = self.dimension();
        for ci in indexes.values_mut() {
            let new_index: Box<dyn VectorIndex> = match ci.config.index_type.as_str() {
                "hnsw" => Box::new(HnswIndex::new(dim, ci.config.distance)),
                _ => Box::new(FlatIndex::new(dim, ci.config.distance)),
            };
            ci.index = new_index;
        }
    }

    /// Search by vector with optional filters.
    pub fn search_by_vector(
        &self,
        index_name: &str,
        dense_vector: &[f32],
        limit: usize,
        offset: usize,
        filters: Option<&Value>,
    ) -> Result<CollectionSearchResult> {
        let indexes = self.indexes.read();
        let ci = indexes.get(index_name).ok_or_else(|| VectorDbError::IndexNotFound(index_name.to_string()))?;

        let filter = filters.and_then(Filter::from_json);

        // If no filter, use index search directly
        if filter.is_none() {
            let actual_limit = limit + offset;
            let idx_result = ci.index.search(dense_vector, actual_limit)?;
            let records = self.records.read();

            let mut data: Vec<SearchItem> = Vec::new();
            for (i, (&id, &score)) in idx_result.ids.iter().zip(idx_result.scores.iter()).enumerate() {
                if i < offset { continue; }
                if data.len() >= limit { break; }
                let fields = records.get(&id).map(|r| r.fields.clone()).unwrap_or_default();
                let pk_val = self.label_to_pk(id);
                data.push(SearchItem { id: pk_val, score, fields });
            }

            return Ok(CollectionSearchResult { data });
        }

        // With filter: search more candidates, then filter
        let filter = filter.unwrap();
        let search_limit = (limit + offset) * 10; // over-fetch for filtering
        let idx_result = ci.index.search(dense_vector, search_limit)?;
        let records = self.records.read();

        let mut data: Vec<SearchItem> = Vec::new();
        let mut skipped = 0;
        for (&id, &score) in idx_result.ids.iter().zip(idx_result.scores.iter()) {
            if let Some(record) = records.get(&id) {
                if filter.matches(&record.fields) {
                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }
                    if data.len() >= limit { break; }
                    let pk_val = self.label_to_pk(id);
                    data.push(SearchItem { id: pk_val, score, fields: record.fields.clone() });
                }
            }
        }

        Ok(CollectionSearchResult { data })
    }

    /// Get record count.
    pub fn count(&self) -> usize {
        self.records.read().len()
    }

    /// Close the collection.
    pub fn close(&self) {
        if let Some(ref path) = self.path {
            let _ = self.persist(path);
        }
    }

    /// Drop the collection (remove all data and optionally files).
    pub fn drop_collection(&self) {
        self.close();
        if let Some(ref path) = self.path {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    fn label_to_pk(&self, label: u64) -> Value {
        if let Some(pk_name) = self.config.primary_key() {
            let records = self.records.read();
            if let Some(record) = records.get(&label) {
                if let Some(val) = record.fields.get(pk_name) {
                    return val.clone();
                }
            }
        }
        Value::from(label)
    }

    fn persist(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path)?;
        // Save config
        let config_path = path.join("collection_config.json");
        let config_bytes = serde_json::to_vec_pretty(&self.config)
            .map_err(|e| VectorDbError::Serialization(e.to_string()))?;
        std::fs::write(&config_path, &config_bytes)?;

        // Save records
        let records = self.records.read();
        let records_vec: Vec<&Record> = records.values().collect();
        let records_bytes = serde_json::to_vec(&records_vec)
            .map_err(|e| VectorDbError::Serialization(e.to_string()))?;
        std::fs::write(path.join("records.json"), &records_bytes)?;

        // Save indexes
        let indexes = self.indexes.read();
        for (name, ci) in indexes.iter() {
            let index_path = path.join("indexes").join(name);
            let _ = ci.index.save(&index_path);
        }

        Ok(())
    }

    fn try_recover(&mut self) -> Result<()> {
        if let Some(ref path) = self.path {
            let records_path = path.join("records.json");
            if records_path.exists() {
                let data = std::fs::read(&records_path)?;
                if let Ok(records_vec) = serde_json::from_slice::<Vec<Record>>(&data) {
                    let mut records = self.records.write();
                    let mut max_id = 0u64;
                    for r in records_vec {
                        if r.label > max_id { max_id = r.label; }
                        records.insert(r.label, r);
                    }
                    *self.next_auto_id.write() = max_id + 1;
                }
            }
        }
        Ok(())
    }
}

impl Drop for Collection {
    fn drop(&mut self) {
        if let Some(ref path) = self.path {
            let _ = self.persist(path);
        }
    }
}

// -- Helper functions --

pub fn value_to_u64(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i as u64)).unwrap_or(0),
        Value::String(s) => {
            // Hash string to u64 for primary key mapping
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            s.hash(&mut hasher);
            hasher.finish()
        }
        _ => 0,
    }
}

fn value_to_f32_vec(v: &Value) -> Vec<f32> {
    match v {
        Value::Array(arr) => arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect(),
        _ => Vec::new(),
    }
}
