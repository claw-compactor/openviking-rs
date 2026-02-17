//! Local filesystem import/export operations.
//!
//! Port of `openviking/storage/local_fs.py`.

use crate::viking_fs::VikingFS;
use ov_core::error::Result;

/// Ensure a path has `.ovpack` extension.
pub fn ensure_ovpack_extension(path: &str) -> String {
    if path.ends_with(".ovpack") {
        path.to_string()
    } else {
        format!("{path}.ovpack")
    }
}

/// Convert a relative path component starting with `.` to `_._` prefix
/// for ZIP-safe storage.
pub fn to_zip_safe_path(base_name: &str, rel_path: &str) -> String {
    let parts: Vec<String> = rel_path
        .split('/')
        .map(|p| {
            if let Some(rest) = p.strip_prefix('.') {
                format!("_._{rest}")
            } else {
                p.to_string()
            }
        })
        .collect();
    format!("{base_name}/{}", parts.join("/"))
}

/// Restore a Viking relative path from a ZIP-safe path.
pub fn from_zip_safe_path(zip_path: &str) -> String {
    let parts: Vec<&str> = zip_path.split('/').collect();
    if parts.len() <= 1 {
        return String::new();
    }
    let rel_parts: Vec<String> = parts[1..]
        .iter()
        .map(|&p| {
            if let Some(rest) = p.strip_prefix("_._") {
                format!(".{rest}")
            } else {
                p.to_string()
            }
        })
        .collect();
    rel_parts.join("/")
}

/// Simple file-based key-value store.
pub struct FileKvStore {
    vfs: std::sync::Arc<VikingFS>,
    base_uri: String,
}

impl FileKvStore {
    /// Create a new KV store backed by VikingFS at the given base URI.
    pub fn new(vfs: std::sync::Arc<VikingFS>, base_uri: impl Into<String>) -> Self {
        Self {
            vfs,
            base_uri: base_uri.into(),
        }
    }

    /// Get a value by key.
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let uri = format!("{}/{key}", self.base_uri);
        match self.vfs.read(&uri).await {
            Ok(data) => Ok(Some(data)),
            Err(_) => Ok(None),
        }
    }

    /// Set a value for a key.
    pub async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        let uri = format!("{}/{key}", self.base_uri);
        self.vfs.write(&uri, value).await
    }

    /// Delete a key.
    pub async fn delete(&self, key: &str) -> Result<()> {
        let uri = format!("{}/{key}", self.base_uri);
        self.vfs.rm(&uri, false).await
    }

    /// List all keys.
    pub async fn keys(&self) -> Result<Vec<String>> {
        let entries = self.vfs.ls(&self.base_uri).await?;
        Ok(entries.into_iter().map(|e| e.name).collect())
    }

    /// Check if a key exists.
    pub async fn contains(&self, key: &str) -> bool {
        let uri = format!("{}/{key}", self.base_uri);
        self.vfs.exists(&uri).await
    }
}

/// A simple bytes row for tabular data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BytesRow {
    /// Row key.
    pub key: String,
    /// Row data.
    pub data: Vec<u8>,
    /// Optional metadata.
    #[serde(default)]
    pub meta: std::collections::HashMap<String, String>,
}

impl BytesRow {
    /// Create a new bytes row.
    pub fn new(key: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            key: key.into(),
            data,
            meta: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_ovpack_extension() {
        assert_eq!(ensure_ovpack_extension("test"), "test.ovpack");
        assert_eq!(ensure_ovpack_extension("test.ovpack"), "test.ovpack");
    }

    #[test]
    fn test_to_zip_safe_path() {
        assert_eq!(
            to_zip_safe_path("root", ".abstract.md"),
            "root/_._abstract.md"
        );
        assert_eq!(to_zip_safe_path("root", "normal.txt"), "root/normal.txt");
        assert_eq!(
            to_zip_safe_path("root", "dir/.hidden"),
            "root/dir/_._hidden"
        );
    }

    #[test]
    fn test_from_zip_safe_path() {
        assert_eq!(from_zip_safe_path("root/_._abstract.md"), ".abstract.md");
        assert_eq!(from_zip_safe_path("root/normal.txt"), "normal.txt");
        assert_eq!(from_zip_safe_path("root"), "");
    }

    #[test]
    fn test_zip_path_roundtrip() {
        let rel = ".abstract.md";
        let zip = to_zip_safe_path("base", rel);
        let back = from_zip_safe_path(&zip);
        assert_eq!(back, rel);
    }

    #[test]
    fn test_bytes_row_new() {
        let row = BytesRow::new("k", vec![1, 2, 3]);
        assert_eq!(row.key, "k");
        assert_eq!(row.data, vec![1, 2, 3]);
        assert!(row.meta.is_empty());
    }

    #[test]
    fn test_bytes_row_serde() {
        let row = BytesRow::new("k", vec![1, 2]);
        let json = serde_json::to_string(&row).unwrap();
        let row2: BytesRow = serde_json::from_str(&json).unwrap();
        assert_eq!(row, row2);
    }

    #[tokio::test]
    async fn test_kv_store_crud() {
        let tmp = TempDir::new().unwrap();
        let vfs = std::sync::Arc::new(VikingFS::new(tmp.path()));
        vfs.mkdir("viking://kv").await.unwrap();
        let store = FileKvStore::new(vfs.clone(), "viking://kv");

        // Set
        store.set("key1", b"value1").await.unwrap();
        assert!(store.contains("key1").await);

        // Get
        let val = store.get("key1").await.unwrap().unwrap();
        assert_eq!(val, b"value1");

        // Keys
        let keys = store.keys().await.unwrap();
        assert_eq!(keys, vec!["key1"]);

        // Delete
        store.delete("key1").await.unwrap();
        assert!(!store.contains("key1").await);
    }

    #[tokio::test]
    async fn test_kv_store_get_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let vfs = std::sync::Arc::new(VikingFS::new(tmp.path()));
        vfs.mkdir("viking://kv").await.unwrap();
        let store = FileKvStore::new(vfs, "viking://kv");
        assert!(store.get("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_kv_store_overwrite() {
        let tmp = TempDir::new().unwrap();
        let vfs = std::sync::Arc::new(VikingFS::new(tmp.path()));
        vfs.mkdir("viking://kv").await.unwrap();
        let store = FileKvStore::new(vfs, "viking://kv");
        store.set("k", b"v1").await.unwrap();
        store.set("k", b"v2").await.unwrap();
        let val = store.get("k").await.unwrap().unwrap();
        assert_eq!(val, b"v2");
    }

    #[tokio::test]
    async fn test_kv_store_multiple_keys() {
        let tmp = TempDir::new().unwrap();
        let vfs = std::sync::Arc::new(VikingFS::new(tmp.path()));
        vfs.mkdir("viking://kv").await.unwrap();
        let store = FileKvStore::new(vfs, "viking://kv");
        for i in 0..5 {
            store.set(&format!("key_{i}"), format!("val_{i}").as_bytes()).await.unwrap();
        }
        let keys = store.keys().await.unwrap();
        assert_eq!(keys.len(), 5);
    }

    #[tokio::test]
    async fn test_kv_store_binary_data() {
        let tmp = TempDir::new().unwrap();
        let vfs = std::sync::Arc::new(VikingFS::new(tmp.path()));
        vfs.mkdir("viking://kv").await.unwrap();
        let store = FileKvStore::new(vfs, "viking://kv");
        let binary = vec![0u8, 1, 2, 255, 254, 253];
        store.set("bin", &binary).await.unwrap();
        let val = store.get("bin").await.unwrap().unwrap();
        assert_eq!(val, binary);
    }
}
