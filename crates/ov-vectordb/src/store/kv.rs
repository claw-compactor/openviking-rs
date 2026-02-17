use std::collections::HashMap;
use parking_lot::RwLock;

/// Simple KV store trait.
pub trait KvStore: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn put(&self, key: &str, value: Vec<u8>);
    fn delete(&self, key: &str) -> bool;
    fn contains(&self, key: &str) -> bool;
    fn keys(&self) -> Vec<String>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn clear(&self);
}

/// In-memory KV store.
pub struct MemoryKvStore {
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryKvStore {
    pub fn new() -> Self {
        Self { data: RwLock::new(HashMap::new()) }
    }
}

impl Default for MemoryKvStore {
    fn default() -> Self { Self::new() }
}

impl KvStore for MemoryKvStore {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.data.read().get(key).cloned()
    }

    fn put(&self, key: &str, value: Vec<u8>) {
        self.data.write().insert(key.to_string(), value);
    }

    fn delete(&self, key: &str) -> bool {
        self.data.write().remove(key).is_some()
    }

    fn contains(&self, key: &str) -> bool {
        self.data.read().contains_key(key)
    }

    fn keys(&self) -> Vec<String> {
        self.data.read().keys().cloned().collect()
    }

    fn len(&self) -> usize {
        self.data.read().len()
    }

    fn clear(&self) {
        self.data.write().clear();
    }
}
