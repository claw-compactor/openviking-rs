//! KV store implementations: in-memory and file-based persistent store.

mod kv;
mod file_store;
mod bytes_row;

pub use kv::{KvStore, MemoryKvStore};
pub use file_store::FileStore;
pub use bytes_row::{BytesRow, FieldSchema, SchemaFieldType};

use std::collections::BTreeMap;
use parking_lot::RwLock;

/// Multi-table store abstraction (like Python's IMutiTableStore).
/// Tables are namespaced key-value stores with ordered keys.
pub struct MultiTableStore {
    tables: RwLock<std::collections::HashMap<String, BTreeMap<String, Vec<u8>>>>,
}

impl MultiTableStore {
    pub fn new() -> Self {
        Self {
            tables: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn read(&self, keys: &[String], table: &str) -> Vec<Option<Vec<u8>>> {
        let tables = self.tables.read();
        let tbl = tables.get(table);
        keys.iter().map(|k| {
            tbl.and_then(|t| t.get(k).cloned())
        }).collect()
    }

    pub fn write(&self, keys: &[String], values: &[Vec<u8>], table: &str) {
        let mut tables = self.tables.write();
        let tbl = tables.entry(table.to_string()).or_default();
        for (k, v) in keys.iter().zip(values.iter()) {
            tbl.insert(k.clone(), v.clone());
        }
    }

    pub fn delete(&self, keys: &[String], table: &str) {
        let mut tables = self.tables.write();
        if let Some(tbl) = tables.get_mut(table) {
            for k in keys {
                tbl.remove(k);
            }
        }
    }

    pub fn clear(&self) {
        let mut tables = self.tables.write();
        tables.clear();
    }

    pub fn read_all(&self, table: &str) -> Vec<(String, Vec<u8>)> {
        let tables = self.tables.read();
        tables.get(table).map(|t| {
            t.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        }).unwrap_or_default()
    }

    /// All entries where key >= start_key.
    pub fn seek_to_end(&self, start_key: &str, table: &str) -> Vec<(String, Vec<u8>)> {
        let tables = self.tables.read();
        tables.get(table).map(|t| {
            t.range(start_key.to_string()..)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        }).unwrap_or_default()
    }

    /// All entries where key <= end_key.
    pub fn begin_to_seek(&self, end_key: &str, table: &str) -> Vec<(String, Vec<u8>)> {
        let tables = self.tables.read();
        tables.get(table).map(|t| {
            t.range(..=end_key.to_string())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        }).unwrap_or_default()
    }
}

impl Default for MultiTableStore {
    fn default() -> Self {
        Self::new()
    }
}
