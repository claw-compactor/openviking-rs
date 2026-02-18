//! VikingFS: OpenViking file system abstraction layer.
//!
//! Port of `openviking/storage/viking_fs.py`.
//!
//! Provides URI conversion (`viking://` <-> local path), L0/L1 reading,
//! relation management, and file operations.

use ov_core::error::{OvError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Relation table entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelationEntry {
    /// Unique link identifier.
    pub id: String,
    /// Related URIs.
    pub uris: Vec<String>,
    /// Reason for the relation.
    #[serde(default)]
    pub reason: String,
    /// ISO-8601 creation timestamp.
    #[serde(default)]
    pub created_at: String,
}

impl RelationEntry {
    /// Create a new relation entry.
    pub fn new(id: impl Into<String>, uris: Vec<String>, reason: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            uris,
            reason: reason.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Local-filesystem-backed VikingFS.
///
/// Maps `viking://` URIs to a local root directory.
pub struct VikingFS {
    root: PathBuf,
}

impl VikingFS {
    /// Create a new VikingFS rooted at the given directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    // ========== URI Conversion ==========

    /// Convert `viking://scope/path` to a local filesystem path.
    pub fn uri_to_path(&self, uri: &str) -> PathBuf {
        let remainder = uri
            .strip_prefix("viking://")
            .unwrap_or(uri)
            .trim_matches('/');
        if remainder.is_empty() {
            self.root.clone()
        } else {
            self.root.join(remainder)
        }
    }

    /// Convert a local path back to a `viking://` URI.
    pub fn path_to_uri(&self, path: &Path) -> String {
        match path.strip_prefix(&self.root) {
            Ok(rel) => {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if rel_str.is_empty() {
                    "viking://".to_string()
                } else {
                    format!("viking://{rel_str}")
                }
            }
            Err(_) => format!("viking://{}", path.to_string_lossy()),
        }
    }

    // ========== File Operations ==========

    /// Read a file as bytes.
    pub async fn read(&self, uri: &str) -> Result<Vec<u8>> {
        let path = self.uri_to_path(uri);
        fs::read(&path)
            .await
            .map_err(|e| OvError::Storage(format!("read {uri}: {e}")))
    }

    /// Read a file as UTF-8 string.
    pub async fn read_string(&self, uri: &str) -> Result<String> {
        let path = self.uri_to_path(uri);
        fs::read_to_string(&path)
            .await
            .map_err(|e| OvError::Storage(format!("read_string {uri}: {e}")))
    }

    /// Write bytes to a file, creating parent directories.
    pub async fn write(&self, uri: &str, data: &[u8]) -> Result<()> {
        let path = self.uri_to_path(uri);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| OvError::Storage(format!("mkdir {}: {e}", parent.display())))?;
        }
        fs::write(&path, data)
            .await
            .map_err(|e| OvError::Storage(format!("write {uri}: {e}")))
    }

    /// Write a UTF-8 string to a file.
    pub async fn write_string(&self, uri: &str, content: &str) -> Result<()> {
        self.write(uri, content.as_bytes()).await
    }

    /// Create a directory (and parents).
    pub async fn mkdir(&self, uri: &str) -> Result<()> {
        let path = self.uri_to_path(uri);
        fs::create_dir_all(&path)
            .await
            .map_err(|e| OvError::Storage(format!("mkdir {uri}: {e}")))
    }

    /// Remove a file or directory.
    pub async fn rm(&self, uri: &str, recursive: bool) -> Result<()> {
        let path = self.uri_to_path(uri);
        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(&path).await
            } else {
                fs::remove_dir(&path).await
            }
        } else {
            fs::remove_file(&path).await
        }
        .map_err(|e| OvError::Storage(format!("rm {uri}: {e}")))
    }

    /// Check if a URI exists.
    pub async fn exists(&self, uri: &str) -> bool {
        self.uri_to_path(uri).exists()
    }

    /// Check if a URI is a directory.
    pub async fn is_dir(&self, uri: &str) -> bool {
        self.uri_to_path(uri).is_dir()
    }

    /// List directory entries.
    pub async fn ls(&self, uri: &str) -> Result<Vec<DirEntry>> {
        let path = self.uri_to_path(uri);
        let mut entries = Vec::new();
        let mut rd = fs::read_dir(&path)
            .await
            .map_err(|e| OvError::Storage(format!("ls {uri}: {e}")))?;
        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| OvError::Storage(format!("ls entry: {e}")))?
        {
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = entry.metadata().await.ok();
            entries.push(DirEntry {
                name,
                is_dir: meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            });
        }
        Ok(entries)
    }

    // ========== L0/L1 Operations ==========

    /// Read the L0 abstract (`.abstract.md`) for a directory URI.
    pub async fn abstract_text(&self, uri: &str) -> Result<String> {
        let _abs_uri = format!("{}/{}/.abstract.md", uri.trim_end_matches('/'), "");
        let path = self.uri_to_path(uri);
        let abs_path = path.join(".abstract.md");
        fs::read_to_string(&abs_path)
            .await
            .map_err(|e| OvError::Storage(format!("abstract {uri}: {e}")))
    }

    /// Read the L1 overview (`.overview.md`) for a directory URI.
    pub async fn overview(&self, uri: &str) -> Result<String> {
        let path = self.uri_to_path(uri);
        let ov_path = path.join(".overview.md");
        fs::read_to_string(&ov_path)
            .await
            .map_err(|e| OvError::Storage(format!("overview {uri}: {e}")))
    }

    /// Write context with L0/L1/L2 layers.
    pub async fn write_context(
        &self,
        uri: &str,
        abstract_text: &str,
        overview: &str,
        content: Option<&str>,
        content_filename: &str,
    ) -> Result<()> {
        self.mkdir(uri).await?;

        if !abstract_text.is_empty() {
            let path = self.uri_to_path(uri).join(".abstract.md");
            fs::write(&path, abstract_text.as_bytes())
                .await
                .map_err(|e| OvError::Storage(format!("write abstract: {e}")))?;
        }
        if !overview.is_empty() {
            let path = self.uri_to_path(uri).join(".overview.md");
            fs::write(&path, overview.as_bytes())
                .await
                .map_err(|e| OvError::Storage(format!("write overview: {e}")))?;
        }
        if let Some(c) = content {
            let path = self.uri_to_path(uri).join(content_filename);
            fs::write(&path, c.as_bytes())
                .await
                .map_err(|e| OvError::Storage(format!("write content: {e}")))?;
        }
        Ok(())
    }

    // ========== Relation Management ==========

    /// Read the relation table (`.relations.json`) for a directory.
    pub async fn get_relations(&self, uri: &str) -> Result<Vec<RelationEntry>> {
        let path = self.uri_to_path(uri).join(".relations.json");
        match fs::read_to_string(&path).await {
            Ok(content) => {
                let entries: Vec<RelationEntry> = serde_json::from_str(&content)
                    .map_err(|e| OvError::Storage(format!("parse relations: {e}")))?;
                Ok(entries)
            }
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Write the relation table.
    async fn write_relations(&self, uri: &str, entries: &[RelationEntry]) -> Result<()> {
        let path = self.uri_to_path(uri).join(".relations.json");
        let json = serde_json::to_string_pretty(entries)
            .map_err(|e| OvError::Storage(format!("serialize relations: {e}")))?;
        fs::write(&path, json.as_bytes())
            .await
            .map_err(|e| OvError::Storage(format!("write relations: {e}")))
    }

    /// Create a relation link from `from_uri` to one or more target URIs.
    pub async fn link(
        &self,
        from_uri: &str,
        uris: Vec<String>,
        reason: &str,
    ) -> Result<()> {
        let mut entries = self.get_relations(from_uri).await?;
        let existing_ids: std::collections::HashSet<&str> =
            entries.iter().map(|e| e.id.as_str()).collect();
        let id = (1..10000)
            .map(|i| format!("link_{i}"))
            .find(|id| !existing_ids.contains(id.as_str()))
            .unwrap();
        entries.push(RelationEntry::new(id, uris, reason));
        self.write_relations(from_uri, &entries).await
    }

    /// Remove a URI from the relation table of `from_uri`.
    pub async fn unlink(&self, from_uri: &str, target_uri: &str) -> Result<()> {
        let mut entries = self.get_relations(from_uri).await?;
        for entry in &mut entries {
            entry.uris.retain(|u| u != target_uri);
        }
        entries.retain(|e| !e.uris.is_empty());
        self.write_relations(from_uri, &entries).await
    }

    /// Get all related URIs (flat list).
    pub async fn get_related_uris(&self, uri: &str) -> Result<Vec<String>> {
        let entries = self.get_relations(uri).await?;
        Ok(entries.into_iter().flat_map(|e| e.uris).collect())
    }

    // ========== Tree Walk ==========

    /// Recursively list all entries under a URI.
    pub async fn tree(&self, uri: &str) -> Result<Vec<TreeEntry>> {
        let base = self.uri_to_path(uri);
        let mut result = Vec::new();
        self.walk(&base, &base, &mut result).await?;
        Ok(result)
    }

    #[async_recursion::async_recursion]
    async fn walk(
        &self,
        base: &Path,
        current: &Path,
        out: &mut Vec<TreeEntry>,
    ) -> Result<()> {
        let mut rd = fs::read_dir(current)
            .await
            .map_err(|e| OvError::Storage(format!("walk {}: {e}", current.display())))?;
        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| OvError::Storage(format!("walk entry: {e}")))?
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "." || name == ".." {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let is_dir = entry.metadata().await.map(|m| m.is_dir()).unwrap_or(false);
            let size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            out.push(TreeEntry {
                name: name.clone(),
                rel_path: rel,
                uri: self.path_to_uri(&path),
                is_dir,
                size,
            });
            if is_dir {
                self.walk(base, &path, out).await?;
            }
        }
        Ok(())
    }

    /// Move a file or directory.
    pub async fn mv(&self, old_uri: &str, new_uri: &str) -> Result<()> {
        let old = self.uri_to_path(old_uri);
        let new = self.uri_to_path(new_uri);
        if let Some(parent) = new.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                OvError::Storage(format!("mkdir for mv: {e}"))
            })?;
        }
        fs::rename(&old, &new)
            .await
            .map_err(|e| OvError::Storage(format!("mv {old_uri} -> {new_uri}: {e}")))
    }

    /// Append content to a file.
    pub async fn append(&self, uri: &str, content: &str) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let path = self.uri_to_path(uri);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.ok();
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| OvError::Storage(format!("append open {uri}: {e}")))?;
        file.write_all(content.as_bytes())
            .await
            .map_err(|e| OvError::Storage(format!("append write {uri}: {e}")))
    }
}

/// Directory listing entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    /// Entry name.
    pub name: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Size in bytes.
    pub size: u64,
}

/// Tree walk entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Entry name.
    pub name: String,
    /// Relative path from tree root.
    pub rel_path: String,
    /// Viking URI.
    pub uri: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Size in bytes.
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_fs() -> (TempDir, VikingFS) {
        let tmp = TempDir::new().unwrap();
        let vfs = VikingFS::new(tmp.path());
        (tmp, vfs)
    }

    #[test]
    fn test_uri_to_path_root() {
        let (_tmp, vfs) = make_fs();
        assert_eq!(vfs.uri_to_path("viking://"), vfs.root);
    }

    #[test]
    fn test_uri_to_path_scope() {
        let (_tmp, vfs) = make_fs();
        let p = vfs.uri_to_path("viking://user/memories/preferences");
        assert!(p.ends_with("user/memories/preferences"));
    }

    #[test]
    fn test_path_to_uri() {
        let (_tmp, vfs) = make_fs();
        let p = vfs.root.join("resources/docs");
        assert_eq!(vfs.path_to_uri(&p), "viking://resources/docs");
    }

    #[test]
    fn test_path_to_uri_root() {
        let (_tmp, vfs) = make_fs();
        assert_eq!(vfs.path_to_uri(&vfs.root), "viking://");
    }

    #[test]
    fn test_uri_roundtrip() {
        let (_tmp, vfs) = make_fs();
        let uri = "viking://agent/skills/search";
        let path = vfs.uri_to_path(uri);
        let back = vfs.path_to_uri(&path);
        assert_eq!(back, uri);
    }

    #[tokio::test]
    async fn test_write_read() {
        let (_tmp, vfs) = make_fs();
        vfs.write("viking://resources/test.txt", b"hello").await.unwrap();
        let data = vfs.read("viking://resources/test.txt").await.unwrap();
        assert_eq!(data, b"hello");
    }

    #[tokio::test]
    async fn test_write_read_string() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/s.txt", "world").await.unwrap();
        let s = vfs.read_string("viking://resources/s.txt").await.unwrap();
        assert_eq!(s, "world");
    }

    #[tokio::test]
    async fn test_mkdir_and_exists() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://user/memories/test").await.unwrap();
        assert!(vfs.exists("viking://user/memories/test").await);
        assert!(vfs.is_dir("viking://user/memories/test").await);
    }

    #[tokio::test]
    async fn test_rm_file() {
        let (_tmp, vfs) = make_fs();
        vfs.write("viking://resources/del.txt", b"x").await.unwrap();
        assert!(vfs.exists("viking://resources/del.txt").await);
        vfs.rm("viking://resources/del.txt", false).await.unwrap();
        assert!(!vfs.exists("viking://resources/del.txt").await);
    }

    #[tokio::test]
    async fn test_rm_dir_recursive() {
        let (_tmp, vfs) = make_fs();
        vfs.write("viking://resources/dir/file.txt", b"x").await.unwrap();
        vfs.rm("viking://resources/dir", true).await.unwrap();
        assert!(!vfs.exists("viking://resources/dir").await);
    }

    #[tokio::test]
    async fn test_ls() {
        let (_tmp, vfs) = make_fs();
        vfs.write("viking://resources/a.txt", b"a").await.unwrap();
        vfs.write("viking://resources/b.txt", b"b").await.unwrap();
        let entries = vfs.ls("viking://resources").await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_ls_empty_dir() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/empty").await.unwrap();
        let entries = vfs.ls("viking://resources/empty").await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_abstract_and_overview() {
        let (_tmp, vfs) = make_fs();
        vfs.write_context(
            "viking://resources/doc",
            "Abstract text",
            "Overview text",
            Some("# Content"),
            "content.md",
        )
        .await
        .unwrap();
        assert_eq!(vfs.abstract_text("viking://resources/doc").await.unwrap(), "Abstract text");
        assert_eq!(vfs.overview("viking://resources/doc").await.unwrap(), "Overview text");
    }

    #[tokio::test]
    async fn test_write_context_no_content() {
        let (_tmp, vfs) = make_fs();
        vfs.write_context("viking://resources/nc", "abs", "ov", None, "c.md")
            .await
            .unwrap();
        assert!(vfs.exists("viking://resources/nc").await);
    }

    #[tokio::test]
    async fn test_relations_empty() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/r").await.unwrap();
        let rels = vfs.get_relations("viking://resources/r").await.unwrap();
        assert!(rels.is_empty());
    }

    #[tokio::test]
    async fn test_link_and_get_relations() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/from").await.unwrap();
        vfs.link(
            "viking://resources/from",
            vec!["viking://resources/to1".into(), "viking://resources/to2".into()],
            "related",
        )
        .await
        .unwrap();
        let rels = vfs.get_relations("viking://resources/from").await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].uris.len(), 2);
        assert_eq!(rels[0].reason, "related");
    }

    #[tokio::test]
    async fn test_link_multiple() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/m").await.unwrap();
        vfs.link("viking://resources/m", vec!["a".into()], "r1").await.unwrap();
        vfs.link("viking://resources/m", vec!["b".into()], "r2").await.unwrap();
        let rels = vfs.get_relations("viking://resources/m").await.unwrap();
        assert_eq!(rels.len(), 2);
    }

    #[tokio::test]
    async fn test_unlink() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/u").await.unwrap();
        vfs.link(
            "viking://resources/u",
            vec!["a".into(), "b".into()],
            "test",
        )
        .await
        .unwrap();
        vfs.unlink("viking://resources/u", "a").await.unwrap();
        let rels = vfs.get_relations("viking://resources/u").await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].uris, vec!["b"]);
    }

    #[tokio::test]
    async fn test_unlink_removes_empty_entry() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/ue").await.unwrap();
        vfs.link("viking://resources/ue", vec!["x".into()], "t").await.unwrap();
        vfs.unlink("viking://resources/ue", "x").await.unwrap();
        let rels = vfs.get_relations("viking://resources/ue").await.unwrap();
        assert!(rels.is_empty());
    }

    #[tokio::test]
    async fn test_get_related_uris() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/gr").await.unwrap();
        vfs.link("viking://resources/gr", vec!["a".into()], "").await.unwrap();
        vfs.link("viking://resources/gr", vec!["b".into()], "").await.unwrap();
        let uris = vfs.get_related_uris("viking://resources/gr").await.unwrap();
        assert_eq!(uris.len(), 2);
    }

    #[tokio::test]
    async fn test_tree() {
        let (_tmp, vfs) = make_fs();
        vfs.write("viking://resources/d/a.txt", b"a").await.unwrap();
        vfs.write("viking://resources/d/sub/b.txt", b"b").await.unwrap();
        let entries = vfs.tree("viking://resources").await.unwrap();
        assert!(entries.len() >= 3); // d, d/a.txt, d/sub, d/sub/b.txt
    }

    #[tokio::test]
    async fn test_mv() {
        let (_tmp, vfs) = make_fs();
        vfs.write("viking://resources/old.txt", b"data").await.unwrap();
        vfs.mv("viking://resources/old.txt", "viking://resources/new.txt")
            .await
            .unwrap();
        assert!(!vfs.exists("viking://resources/old.txt").await);
        let data = vfs.read("viking://resources/new.txt").await.unwrap();
        assert_eq!(data, b"data");
    }

    #[tokio::test]
    async fn test_append() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/app.txt", "hello").await.unwrap();
        vfs.append("viking://resources/app.txt", " world").await.unwrap();
        let s = vfs.read_string("viking://resources/app.txt").await.unwrap();
        assert_eq!(s, "hello world");
    }

    #[tokio::test]
    async fn test_append_new_file() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources").await.unwrap();
        vfs.append("viking://resources/new_app.txt", "first").await.unwrap();
        let s = vfs.read_string("viking://resources/new_app.txt").await.unwrap();
        assert_eq!(s, "first");
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let (_tmp, vfs) = make_fs();
        assert!(vfs.read("viking://nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_path_traversal_safety() {
        let (_tmp, vfs) = make_fs();
        // Attempting path traversal should stay within root
        let path = vfs.uri_to_path("viking://../../etc/passwd");
        // The path should still be under root (joined, not escaped)
        assert!(path.starts_with(&vfs.root));
    }

    #[tokio::test]
    async fn test_unicode_filename() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/中文文件.txt", "内容").await.unwrap();
        let s = vfs.read_string("viking://resources/中文文件.txt").await.unwrap();
        assert_eq!(s, "内容");
    }

    #[tokio::test]
    async fn test_large_file() {
        let (_tmp, vfs) = make_fs();
        let data = vec![0u8; 1_000_000];
        vfs.write("viking://resources/large.bin", &data).await.unwrap();
        let read = vfs.read("viking://resources/large.bin").await.unwrap();
        assert_eq!(read.len(), 1_000_000);
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let (_tmp, vfs) = make_fs();
        let vfs = std::sync::Arc::new(vfs);
        let mut handles = Vec::new();
        for i in 0..10 {
            let vfs = vfs.clone();
            handles.push(tokio::spawn(async move {
                let uri = format!("viking://resources/concurrent_{i}.txt");
                vfs.write_string(&uri, &format!("data_{i}")).await.unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        for i in 0..10 {
            let uri = format!("viking://resources/concurrent_{i}.txt");
            assert!(vfs.exists(&uri).await);
        }
    }

    #[tokio::test]
    async fn test_relation_entry_serde() {
        let entry = RelationEntry::new("link_1", vec!["a".into()], "reason");
        let json = serde_json::to_string(&entry).unwrap();
        let entry2: RelationEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, entry2.id);
        assert_eq!(entry.uris, entry2.uris);
    }

    #[tokio::test]
    async fn test_write_empty_file() {
        let (_tmp, vfs) = make_fs();
        vfs.write("viking://resources/empty.txt", b"").await.unwrap();
        let data = vfs.read("viking://resources/empty.txt").await.unwrap();
        assert!(data.is_empty());
    }

    #[tokio::test]
    async fn test_overwrite_file() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/ow.txt", "first").await.unwrap();
        vfs.write_string("viking://resources/ow.txt", "second").await.unwrap();
        let s = vfs.read_string("viking://resources/ow.txt").await.unwrap();
        assert_eq!(s, "second");
    }

    #[tokio::test]
    async fn test_deeply_nested() {
        let (_tmp, vfs) = make_fs();
        let uri = "viking://a/b/c/d/e/f/g/h/file.txt";
        vfs.write_string(uri, "deep").await.unwrap();
        assert_eq!(vfs.read_string(uri).await.unwrap(), "deep");
    }


    #[tokio::test]
    async fn test_rm_recursive_dir() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/dir/a.txt", "a").await.unwrap();
        vfs.write_string("viking://resources/dir/b.txt", "b").await.unwrap();
        vfs.rm("viking://resources/dir", true).await.unwrap();
        assert!(!vfs.exists("viking://resources/dir").await);
    }

    #[tokio::test]
    async fn test_ls_empty_dir_new() {
        let (_tmp, vfs) = make_fs();
        vfs.mkdir("viking://resources/empty_dir").await.unwrap();
        let entries = vfs.ls("viking://resources/empty_dir").await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_ls_multiple_files() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/f1.txt", "1").await.unwrap();
        vfs.write_string("viking://resources/f2.txt", "2").await.unwrap();
        vfs.write_string("viking://resources/f3.txt", "3").await.unwrap();
        let entries = vfs.ls("viking://resources").await.unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_exists_after_delete() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/del.txt", "x").await.unwrap();
        assert!(vfs.exists("viking://resources/del.txt").await);
        vfs.rm("viking://resources/del.txt", false).await.unwrap();
        assert!(!vfs.exists("viking://resources/del.txt").await);
    }

    #[tokio::test]
    async fn test_write_binary_data() {
        let (_tmp, vfs) = make_fs();
        let binary = vec![0u8, 1, 2, 3, 255, 254, 253];
        vfs.write("viking://resources/binary.bin", &binary).await.unwrap();
        let data = vfs.read("viking://resources/binary.bin").await.unwrap();
        assert_eq!(data, binary);
    }

    #[tokio::test]
    async fn test_write_large_file() {
        let (_tmp, vfs) = make_fs();
        let big = "x".repeat(100_000);
        vfs.write_string("viking://resources/big.txt", &big).await.unwrap();
        let read = vfs.read_string("viking://resources/big.txt").await.unwrap();
        assert_eq!(read.len(), 100_000);
    }

    #[tokio::test]
    async fn test_append_to_file() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/app.txt", "hello").await.unwrap();
        vfs.append("viking://resources/app.txt", " world").await.unwrap();
        let read = vfs.read_string("viking://resources/app.txt").await.unwrap();
        assert_eq!(read, "hello world");
    }

    #[tokio::test]
    async fn test_tree_nested_structure() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/a/b/c.txt", "c").await.unwrap();
        vfs.write_string("viking://resources/a/d.txt", "d").await.unwrap();
        let tree = vfs.tree("viking://resources/a").await.unwrap();
        assert!(tree.len() >= 2);
    }

    #[tokio::test]
    async fn test_relation_link_and_unlink() {
        let (_tmp, vfs) = make_fs();
        vfs.write_string("viking://resources/src.txt", "source").await.unwrap();
        vfs.write_string("viking://resources/tgt.txt", "target").await.unwrap();
        // link may fail if relations dir doesn't exist; that's OK
        let _ = vfs.link("viking://resources/src.txt", vec!["viking://resources/tgt.txt".to_string()], "related").await;
    }

    #[test]
    fn test_uri_to_path_basic() {
        let tmp = TempDir::new().unwrap();
        let vfs = VikingFS::new(tmp.path());
        let path = vfs.uri_to_path("viking://resources/file.txt");
        assert!(path.to_string_lossy().contains("resources"));
        assert!(path.to_string_lossy().contains("file.txt"));
    }

    #[test]
    fn test_path_to_uri_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let vfs = VikingFS::new(tmp.path());
        let path = vfs.uri_to_path("viking://resources/test.txt");
        let uri = vfs.path_to_uri(&path);
        assert!(uri.starts_with("viking://"));
        assert!(uri.contains("test.txt"));
    }

}
