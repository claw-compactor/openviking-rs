//! BuildingTree container for OpenViking context trees.
//!
//! Port of `openviking/core/building_tree.py`.

use crate::context::Context;
use std::collections::HashMap;

/// Container for a built context tree.
///
/// Maintains a flat list of contexts plus a URI-keyed lookup map
/// and tracks parent-child relationships.
#[derive(Debug, Clone)]
pub struct BuildingTree {
    /// Source file/directory path this tree was built from.
    pub source_path: Option<String>,
    /// Source format identifier (e.g. "markdown", "code").
    pub source_format: Option<String>,
    contexts: Vec<Context>,
    uri_map: HashMap<String, usize>,
    root_uri: Option<String>,
}

impl BuildingTree {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self {
            source_path: None,
            source_format: None,
            contexts: Vec::new(),
            uri_map: HashMap::new(),
            root_uri: None,
        }
    }

    /// Create a tree with source metadata.
    pub fn with_source(source_path: impl Into<String>, source_format: impl Into<String>) -> Self {
        Self {
            source_path: Some(source_path.into()),
            source_format: Some(source_format.into()),
            contexts: Vec::new(),
            uri_map: HashMap::new(),
            root_uri: None,
        }
    }

    /// Add a context to the tree.
    pub fn add_context(&mut self, context: Context) {
        let idx = self.contexts.len();
        self.uri_map.insert(context.uri.clone(), idx);
        self.contexts.push(context);
    }

    /// Set the root URI.
    pub fn set_root(&mut self, uri: impl Into<String>) {
        self.root_uri = Some(uri.into());
    }

    /// Get the root context, if set.
    pub fn root(&self) -> Option<&Context> {
        self.root_uri
            .as_ref()
            .and_then(|u| self.get(u))
    }

    /// Get all contexts.
    pub fn contexts(&self) -> &[Context] {
        &self.contexts
    }

    /// Look up a context by URI.
    pub fn get(&self, uri: &str) -> Option<&Context> {
        self.uri_map.get(uri).map(|&i| &self.contexts[i])
    }

    /// Get parent of a context.
    pub fn parent(&self, uri: &str) -> Option<&Context> {
        self.get(uri)
            .and_then(|c| c.parent_uri.as_deref())
            .and_then(|pu| self.get(pu))
    }

    /// Get direct children of a URI.
    pub fn children(&self, uri: &str) -> Vec<&Context> {
        self.contexts
            .iter()
            .filter(|c| c.parent_uri.as_deref() == Some(uri))
            .collect()
    }

    /// Get the path from a context up to the root.
    pub fn path_to_root(&self, uri: &str) -> Vec<&Context> {
        let mut path = Vec::new();
        let mut current_uri = Some(uri);
        while let Some(u) = current_uri {
            match self.get(u) {
                Some(ctx) => {
                    path.push(ctx);
                    current_uri = ctx.parent_uri.as_deref();
                }
                None => break,
            }
        }
        path
    }

    /// Convert to a JSON-serializable directory structure.
    pub fn to_directory_structure(&self) -> serde_json::Value {
        match &self.root_uri {
            Some(uri) => self.build_dir(uri),
            None => serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    fn build_dir(&self, uri: &str) -> serde_json::Value {
        let ctx = match self.get(uri) {
            Some(c) => c,
            None => return serde_json::Value::Object(serde_json::Map::new()),
        };
        let title = ctx
            .meta
            .get("semantic_title")
            .or_else(|| ctx.meta.get("source_title"))
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let children: Vec<serde_json::Value> = self
            .children(uri)
            .iter()
            .map(|c| self.build_dir(&c.uri))
            .collect();
        serde_json::json!({
            "uri": uri,
            "title": title,
            "type": ctx.context_type.as_str(),
            "children": children,
        })
    }

    /// Number of contexts in the tree.
    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }
}

impl Default for BuildingTree {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a BuildingTree {
    type Item = &'a Context;
    type IntoIter = std::slice::Iter<'a, Context>;
    fn into_iter(self) -> Self::IntoIter {
        self.contexts.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(uri: &str, parent: Option<&str>) -> Context {
        Context::builder(uri)
            .abstract_text(format!("abs-{uri}"))
            .is_leaf(parent.is_some())
            .build()
            .tap(|c| {
                // set parent after build
                let _ = c;
            })
    }

    fn make_tree() -> BuildingTree {
        let mut tree = BuildingTree::new();
        let mut root = Context::new("viking://resources", "root");
        root.parent_uri = None;
        tree.add_context(root);
        tree.set_root("viking://resources");

        let mut child = Context::new("viking://resources/docs", "docs");
        child.parent_uri = Some("viking://resources".into());
        tree.add_context(child);

        let mut leaf = Context::new("viking://resources/docs/readme", "readme");
        leaf.parent_uri = Some("viking://resources/docs".into());
        leaf.is_leaf = true;
        tree.add_context(leaf);

        tree
    }

    // Helper trait for inline mutation
    trait Tap: Sized {
        fn tap(self, _f: impl FnOnce(&Self)) -> Self { self }
    }
    impl<T> Tap for T {}

    #[test]
    fn test_empty_tree() {
        let tree = BuildingTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert!(tree.root().is_none());
    }

    #[test]
    fn test_add_and_get() {
        let mut tree = BuildingTree::new();
        let ctx = Context::new("viking://resources/x", "x");
        tree.add_context(ctx);
        assert_eq!(tree.len(), 1);
        assert!(tree.get("viking://resources/x").is_some());
        assert!(tree.get("viking://resources/y").is_none());
    }

    #[test]
    fn test_root() {
        let tree = make_tree();
        let root = tree.root().unwrap();
        assert_eq!(root.uri, "viking://resources");
    }

    #[test]
    fn test_children() {
        let tree = make_tree();
        let kids = tree.children("viking://resources");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].uri, "viking://resources/docs");
    }

    #[test]
    fn test_children_leaf() {
        let tree = make_tree();
        let kids = tree.children("viking://resources/docs/readme");
        assert!(kids.is_empty());
    }

    #[test]
    fn test_parent() {
        let tree = make_tree();
        let p = tree.parent("viking://resources/docs").unwrap();
        assert_eq!(p.uri, "viking://resources");
    }

    #[test]
    fn test_parent_of_root() {
        let tree = make_tree();
        assert!(tree.parent("viking://resources").is_none());
    }

    #[test]
    fn test_path_to_root() {
        let tree = make_tree();
        let path = tree.path_to_root("viking://resources/docs/readme");
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].uri, "viking://resources/docs/readme");
        assert_eq!(path[2].uri, "viking://resources");
    }

    #[test]
    fn test_path_to_root_nonexistent() {
        let tree = make_tree();
        let path = tree.path_to_root("viking://nonexistent");
        assert!(path.is_empty());
    }

    #[test]
    fn test_to_directory_structure() {
        let tree = make_tree();
        let dir = tree.to_directory_structure();
        assert_eq!(dir["uri"], "viking://resources");
        assert!(dir["children"].is_array());
    }

    #[test]
    fn test_to_directory_structure_empty() {
        let tree = BuildingTree::new();
        let dir = tree.to_directory_structure();
        assert!(dir.is_object());
    }

    #[test]
    fn test_iterator() {
        let tree = make_tree();
        let uris: Vec<&str> = tree.into_iter().map(|c| c.uri.as_str()).collect();
        assert_eq!(uris.len(), 3);
    }

    #[test]
    fn test_with_source() {
        let tree = BuildingTree::with_source("/path/to/dir", "markdown");
        assert_eq!(tree.source_path.as_deref(), Some("/path/to/dir"));
        assert_eq!(tree.source_format.as_deref(), Some("markdown"));
    }

    #[test]
    fn test_default() {
        let tree = BuildingTree::default();
        assert!(tree.is_empty());
    }


    // ========== Extended Tree Tests ==========

    #[test]
    fn test_tree_many_contexts() {
        let mut tree = BuildingTree::new();
        for i in 0..50 {
            tree.add_context(Context::new(
                format!("viking://resources/item_{}", i),
                format!("Item {}", i),
            ));
        }
        assert_eq!(tree.len(), 50);
    }

    #[test]
    fn test_tree_get_nonexistent() {
        let tree = BuildingTree::new();
        assert!(tree.get("viking://nonexistent").is_none());
    }

    #[test]
    fn test_tree_root_not_set() {
        let tree = BuildingTree::new();
        assert!(tree.root().is_none());
    }

    #[test]
    fn test_tree_root_set() {
        let mut tree = BuildingTree::new();
        tree.add_context(Context::new("viking://root", "Root"));
        tree.set_root("viking://root");
        assert!(tree.root().is_some());
        assert_eq!(tree.root().unwrap().uri, "viking://root");
    }

    #[test]
    fn test_tree_children_empty() {
        let tree = make_tree();
        let children = tree.children("viking://resources/doc/section1");
        assert!(children.is_empty()); // leaf node has no children
    }

    #[test]
    fn test_tree_duplicate_uri_overwrites() {
        let mut tree = BuildingTree::new();
        tree.add_context(Context::new("viking://a", "First"));
        tree.add_context(Context::new("viking://a", "Second"));
        // Should have 2 entries but uri_map points to last
        let ctx = tree.get("viking://a").unwrap();
        assert_eq!(ctx.abstract_text.as_str(), "Second");
    }

    #[test]
    fn test_tree_contexts_slice() {
        let tree = make_tree();
        let ctxs = tree.contexts();
        assert_eq!(ctxs.len(), 3);
    }

}
