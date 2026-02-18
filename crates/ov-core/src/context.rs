//! Unified context class for OpenViking.
//!
//! Port of `openviking/core/context.py`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Resource content type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ResourceContentType {
    /// Plain text content.
    Text,
    /// Image content.
    Image,
    /// Video content.
    Video,
    /// Audio content.
    Audio,
    /// Binary/opaque content.
    Binary,
}

impl ResourceContentType {
    /// Return the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Binary => "binary",
        }
    }
}

impl std::fmt::Display for ResourceContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ResourceContentType {
    type Err = crate::error::OvError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            "video" => Ok(Self::Video),
            "audio" => Ok(Self::Audio),
            "binary" => Ok(Self::Binary),
            _ => Err(crate::error::OvError::InvalidUri(format!("unknown content type: {s}"))),
        }
    }
}

/// Context type — skill, memory, or resource.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ContextType {
    /// A callable skill.
    Skill,
    /// A memory entry (user or agent).
    Memory,
    /// A generic resource.
    Resource,
}

impl ContextType {
    /// Return the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Memory => "memory",
            Self::Resource => "resource",
        }
    }
}

impl std::fmt::Display for ContextType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ContextType {
    type Err = crate::error::OvError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "skill" => Ok(Self::Skill),
            "memory" => Ok(Self::Memory),
            "resource" => Ok(Self::Resource),
            _ => Err(crate::error::OvError::InvalidUri(format!("unknown context type: {s}"))),
        }
    }
}

/// Vectorization payload — text (and future multi-modal) to embed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Vectorize {
    /// Text to vectorize.
    pub text: String,
}

impl Vectorize {
    /// Create a new vectorize payload with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Unified context — the core data record in OpenViking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Unique identifier.
    pub id: Uuid,
    /// Viking URI (e.g. `viking://user/memories/preferences`).
    pub uri: String,
    /// Parent URI, if any.
    pub parent_uri: Option<String>,
    /// Whether this is a leaf node.
    pub is_leaf: bool,
    /// L0 abstract / summary text.
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    /// Context type.
    pub context_type: ContextType,
    /// Derived category (e.g. "patterns", "preferences").
    pub category: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-updated timestamp.
    pub updated_at: DateTime<Utc>,
    /// Number of times this context was accessed.
    pub active_count: u64,
    /// Related URIs.
    pub related_uri: Vec<String>,
    /// Arbitrary metadata.
    pub meta: HashMap<String, serde_json::Value>,
    /// Session identifier.
    pub session_id: Option<String>,
    /// Dense embedding vector.
    pub vector: Option<Vec<f32>>,
    /// Vectorization payload (not persisted to JSON).
    #[serde(skip)]
    pub vectorize: Vectorize,
}

impl Context {
    /// Create a new context with the given URI and abstract text.
    pub fn new(uri: impl Into<String>, abstract_text: impl Into<String>) -> Self {
        let uri = uri.into();
        let abs = abstract_text.into();
        let context_type = Self::derive_context_type(&uri);
        let category = Self::derive_category(&uri);
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            uri,
            parent_uri: None,
            is_leaf: false,
            abstract_text: abs.clone(),
            context_type,
            category,
            created_at: now,
            updated_at: now,
            active_count: 0,
            related_uri: Vec::new(),
            meta: HashMap::new(),
            session_id: None,
            vector: None,
            vectorize: Vectorize { text: abs },
        }
    }

    /// Create a context builder for more fine-grained construction.
    pub fn builder(uri: impl Into<String>) -> ContextBuilder {
        ContextBuilder::new(uri)
    }

    /// Increment activity count and update timestamp.
    pub fn update_activity(&mut self) {
        self.active_count += 1;
        self.updated_at = Utc::now();
    }

    /// Get the context type (alias).
    pub fn get_context_type(&self) -> &ContextType {
        &self.context_type
    }

    /// Set the vectorization payload.
    pub fn set_vectorize(&mut self, vectorize: Vectorize) {
        self.vectorize = vectorize;
    }

    /// Get text for vectorization.
    pub fn get_vectorization_text(&self) -> &str {
        &self.vectorize.text
    }

    /// Derive context type from URI prefix.
    pub fn derive_context_type(uri: &str) -> ContextType {
        if uri.starts_with("viking://agent/skills") {
            ContextType::Skill
        } else if uri.contains("memories") {
            ContextType::Memory
        } else {
            ContextType::Resource
        }
    }

    /// Derive category from URI prefix.
    pub fn derive_category(uri: &str) -> String {
        if uri.starts_with("viking://agent/memories") {
            if uri.contains("patterns") {
                return "patterns".into();
            }
            if uri.contains("cases") {
                return "cases".into();
            }
        } else if uri.starts_with("viking://user/memories") {
            if uri.contains("profile") {
                return "profile".into();
            }
            if uri.contains("preferences") {
                return "preferences".into();
            }
            if uri.contains("entities") {
                return "entities".into();
            }
            if uri.contains("events") {
                return "events".into();
            }
        }
        String::new()
    }
}

/// Builder for [`Context`].
pub struct ContextBuilder {
    uri: String,
    parent_uri: Option<String>,
    is_leaf: bool,
    abstract_text: String,
    context_type: Option<ContextType>,
    category: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    active_count: u64,
    related_uri: Vec<String>,
    meta: HashMap<String, serde_json::Value>,
    session_id: Option<String>,
    id: Option<Uuid>,
}

impl ContextBuilder {
    /// Create a new builder with the required URI.
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            parent_uri: None,
            is_leaf: false,
            abstract_text: String::new(),
            context_type: None,
            category: None,
            created_at: None,
            updated_at: None,
            active_count: 0,
            related_uri: Vec::new(),
            meta: HashMap::new(),
            session_id: None,
            id: None,
        }
    }

    /// Set the parent URI.
    pub fn parent_uri(mut self, v: impl Into<String>) -> Self {
        self.parent_uri = Some(v.into());
        self
    }

    /// Mark as leaf node.
    pub fn is_leaf(mut self, v: bool) -> Self {
        self.is_leaf = v;
        self
    }

    /// Set the abstract text.
    pub fn abstract_text(mut self, v: impl Into<String>) -> Self {
        self.abstract_text = v.into();
        self
    }

    /// Override context type.
    pub fn context_type(mut self, v: ContextType) -> Self {
        self.context_type = Some(v);
        self
    }

    /// Override category.
    pub fn category(mut self, v: impl Into<String>) -> Self {
        self.category = Some(v.into());
        self
    }

    /// Set creation time.
    pub fn created_at(mut self, v: DateTime<Utc>) -> Self {
        self.created_at = Some(v);
        self
    }

    /// Set active count.
    pub fn active_count(mut self, v: u64) -> Self {
        self.active_count = v;
        self
    }

    /// Set related URIs.
    pub fn related_uri(mut self, v: Vec<String>) -> Self {
        self.related_uri = v;
        self
    }

    /// Set metadata.
    pub fn meta(mut self, v: HashMap<String, serde_json::Value>) -> Self {
        self.meta = v;
        self
    }

    /// Set session id.
    pub fn session_id(mut self, v: impl Into<String>) -> Self {
        self.session_id = Some(v.into());
        self
    }

    /// Override the UUID.
    pub fn id(mut self, v: Uuid) -> Self {
        self.id = Some(v);
        self
    }

    /// Build the [`Context`].
    pub fn build(self) -> Context {
        let context_type = self
            .context_type
            .unwrap_or_else(|| Context::derive_context_type(&self.uri));
        let category = self
            .category
            .unwrap_or_else(|| Context::derive_category(&self.uri));
        let now = Utc::now();
        let created = self.created_at.unwrap_or(now);
        let updated = self.updated_at.unwrap_or(created);
        Context {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            uri: self.uri,
            parent_uri: self.parent_uri,
            is_leaf: self.is_leaf,
            abstract_text: self.abstract_text.clone(),
            context_type,
            category,
            created_at: created,
            updated_at: updated,
            active_count: self.active_count,
            related_uri: self.related_uri,
            meta: self.meta,
            session_id: self.session_id,
            vector: None,
            vectorize: Vectorize {
                text: self.abstract_text,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_new_resource() {
        let ctx = Context::new("viking://resources/docs", "Documentation");
        assert_eq!(ctx.context_type, ContextType::Resource);
        assert_eq!(ctx.abstract_text, "Documentation");
        assert!(ctx.category.is_empty());
    }

    #[test]
    fn test_context_new_skill() {
        let ctx = Context::new("viking://agent/skills/search", "Search skill");
        assert_eq!(ctx.context_type, ContextType::Skill);
    }

    #[test]
    fn test_context_new_memory() {
        let ctx = Context::new("viking://user/memories/preferences/style", "Style pref");
        assert_eq!(ctx.context_type, ContextType::Memory);
        assert_eq!(ctx.category, "preferences");
    }

    #[test]
    fn test_context_agent_memory_patterns() {
        let ctx = Context::new("viking://agent/memories/patterns/p1", "Pattern");
        assert_eq!(ctx.context_type, ContextType::Memory);
        assert_eq!(ctx.category, "patterns");
    }

    #[test]
    fn test_context_agent_memory_cases() {
        let ctx = Context::new("viking://agent/memories/cases/c1", "Case");
        assert_eq!(ctx.category, "cases");
    }

    #[test]
    fn test_context_user_memory_entities() {
        let ctx = Context::new("viking://user/memories/entities/proj", "Project");
        assert_eq!(ctx.category, "entities");
    }

    #[test]
    fn test_context_user_memory_events() {
        let ctx = Context::new("viking://user/memories/events/e1", "Event");
        assert_eq!(ctx.category, "events");
    }

    #[test]
    fn test_context_user_memory_profile() {
        let ctx = Context::new("viking://user/memories/profile", "Profile");
        assert_eq!(ctx.category, "profile");
    }

    #[test]
    fn test_update_activity() {
        let mut ctx = Context::new("viking://resources/test", "test");
        let old = ctx.updated_at;
        assert_eq!(ctx.active_count, 0);
        ctx.update_activity();
        assert_eq!(ctx.active_count, 1);
        assert!(ctx.updated_at >= old);
    }

    #[test]
    fn test_vectorize_default() {
        let v = Vectorize::default();
        assert!(v.text.is_empty());
    }

    #[test]
    fn test_vectorize_new() {
        let v = Vectorize::new("hello");
        assert_eq!(v.text, "hello");
    }

    #[test]
    fn test_set_vectorize() {
        let mut ctx = Context::new("viking://resources/x", "abs");
        ctx.set_vectorize(Vectorize::new("custom"));
        assert_eq!(ctx.get_vectorization_text(), "custom");
    }

    #[test]
    fn test_context_serde_roundtrip() {
        let ctx = Context::new("viking://resources/test", "Test abstract");
        let json = serde_json::to_string(&ctx).unwrap();
        let ctx2: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.uri, ctx2.uri);
        assert_eq!(ctx.abstract_text, ctx2.abstract_text);
        assert_eq!(ctx.context_type, ctx2.context_type);
        assert_eq!(ctx.id, ctx2.id);
    }

    #[test]
    fn test_context_serde_with_vector() {
        let mut ctx = Context::new("viking://resources/v", "vec");
        ctx.vector = Some(vec![0.1, 0.2, 0.3]);
        let json = serde_json::to_string(&ctx).unwrap();
        let ctx2: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx2.vector, Some(vec![0.1, 0.2, 0.3]));
    }

    #[test]
    fn test_context_serde_null_vector() {
        let ctx = Context::new("viking://resources/n", "no vec");
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains(r#""vector":null"#));
    }

    #[test]
    fn test_context_builder_basic() {
        let ctx = Context::builder("viking://resources/b")
            .abstract_text("Builder test")
            .build();
        assert_eq!(ctx.uri, "viking://resources/b");
        assert_eq!(ctx.abstract_text, "Builder test");
    }

    #[test]
    fn test_context_builder_full() {
        let id = Uuid::new_v4();
        let ctx = Context::builder("viking://agent/skills/s1")
            .id(id)
            .parent_uri("viking://agent/skills")
            .is_leaf(true)
            .abstract_text("Skill desc")
            .active_count(5)
            .session_id("sess-1")
            .build();
        assert_eq!(ctx.id, id);
        assert_eq!(ctx.parent_uri.as_deref(), Some("viking://agent/skills"));
        assert!(ctx.is_leaf);
        assert_eq!(ctx.active_count, 5);
        assert_eq!(ctx.session_id.as_deref(), Some("sess-1"));
        assert_eq!(ctx.context_type, ContextType::Skill);
    }

    #[test]
    fn test_context_builder_override_type() {
        let ctx = Context::builder("viking://resources/x")
            .context_type(ContextType::Memory)
            .category("custom")
            .build();
        assert_eq!(ctx.context_type, ContextType::Memory);
        assert_eq!(ctx.category, "custom");
    }

    #[test]
    fn test_resource_content_type_roundtrip() {
        for rct in [ResourceContentType::Text, ResourceContentType::Image,
                    ResourceContentType::Video, ResourceContentType::Audio,
                    ResourceContentType::Binary] {
            let s = rct.as_str();
            let parsed: ResourceContentType = s.parse().unwrap();
            assert_eq!(parsed, rct);
        }
    }

    #[test]
    fn test_resource_content_type_display() {
        assert_eq!(ResourceContentType::Text.to_string(), "text");
        assert_eq!(ResourceContentType::Binary.to_string(), "binary");
    }

    #[test]
    fn test_resource_content_type_invalid() {
        assert!("unknown".parse::<ResourceContentType>().is_err());
    }

    #[test]
    fn test_context_type_roundtrip() {
        for ct in [ContextType::Skill, ContextType::Memory, ContextType::Resource] {
            let s = ct.as_str();
            let parsed: ContextType = s.parse().unwrap();
            assert_eq!(parsed, ct);
        }
    }

    #[test]
    fn test_context_type_display() {
        assert_eq!(ContextType::Skill.to_string(), "skill");
    }

    #[test]
    fn test_context_type_invalid() {
        assert!("bogus".parse::<ContextType>().is_err());
    }

    #[test]
    fn test_context_type_serde() {
        let ct = ContextType::Memory;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, r#""memory""#);
        let ct2: ContextType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct2, ct);
    }

    #[test]
    fn test_resource_content_type_serde() {
        let rct = ResourceContentType::Image;
        let json = serde_json::to_string(&rct).unwrap();
        assert_eq!(json, r#""image""#);
    }

    #[test]
    fn test_context_empty_uri() {
        let ctx = Context::new("", "");
        assert_eq!(ctx.context_type, ContextType::Resource);
        assert!(ctx.category.is_empty());
    }

    #[test]
    fn test_context_special_chars() {
        let ctx = Context::new("viking://resources/日本語/テスト", "特殊文字テスト");
        assert_eq!(ctx.abstract_text, "特殊文字テスト");
        let json = serde_json::to_string(&ctx).unwrap();
        let ctx2: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx2.uri, "viking://resources/日本語/テスト");
    }

    #[test]
    fn test_context_long_abstract() {
        let long = "x".repeat(100_000);
        let ctx = Context::new("viking://resources/long", long.as_str());
        assert_eq!(ctx.abstract_text.len(), 100_000);
    }

    #[test]
    fn test_context_meta_round_trip() {
        let mut ctx = Context::new("viking://resources/m", "meta test");
        ctx.meta.insert("name".into(), serde_json::json!("tool"));
        ctx.meta.insert("version".into(), serde_json::json!(42));
        let json = serde_json::to_string(&ctx).unwrap();
        let ctx2: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx2.meta["name"], "tool");
        assert_eq!(ctx2.meta["version"], 42);
    }

    #[test]
    fn test_context_related_uri() {
        let mut ctx = Context::new("viking://resources/r", "rel");
        ctx.related_uri.push("viking://resources/other".into());
        let json = serde_json::to_string(&ctx).unwrap();
        let ctx2: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx2.related_uri, vec!["viking://resources/other"]);
    }

    #[test]
    fn test_derive_context_type_session() {
        // session URIs without "memories" default to Resource
        let ct = Context::derive_context_type("viking://session/123");
        assert_eq!(ct, ContextType::Resource);
    }

    // ========== Extended Context Tests ==========

    #[test]
    fn test_context_builder_with_all_fields() {
        let ctx = Context::builder("viking://resources/full")
            .abstract_text("Full context")
            .context_type(ContextType::Resource)
            .is_leaf(true)
            .build();
        assert_eq!(ctx.uri, "viking://resources/full");
        assert_eq!(ctx.abstract_text.as_str(), "Full context");
        assert!(ctx.is_leaf);
    }

    #[test]
    fn test_context_derive_type_memory() {
        let ct = Context::derive_context_type("viking://user/memories/preferences/theme");
        assert_eq!(ct, ContextType::Memory);
    }

    #[test]
    fn test_context_derive_type_resource() {
        let ct = Context::derive_context_type("viking://resources/doc.md");
        assert_eq!(ct, ContextType::Resource);
    }

    #[test]
    fn test_context_derive_type_skill() {
        let ct = Context::derive_context_type("viking://agent/skills/search");
        assert_eq!(ct, ContextType::Skill);
    }

    #[test]
    fn test_context_empty_abstract() {
        let ctx = Context::new("viking://resources/empty", "");
        assert_eq!(ctx.abstract_text.as_str(), "");
    }

    #[test]
    fn test_context_very_long_abstract() {
        let big = "x".repeat(50_000);
        let ctx = Context::new("viking://resources/big", &big);
        assert_eq!(ctx.abstract_text.as_str().len(), 50_000);
    }

    #[test]
    fn test_context_unicode_uri() {
        let ctx = Context::new("viking://resources/日本語/テスト", "Unicode");
        assert!(ctx.uri.contains("日本語"));
    }

    #[test]
    fn test_context_type_display_all() {
        assert_eq!(ContextType::Resource.to_string(), "resource");
        assert_eq!(ContextType::Memory.to_string(), "memory");
        assert_eq!(ContextType::Skill.to_string(), "skill");
    }

    #[test]
    fn test_context_multiple_related_uris() {
        let mut ctx = Context::new("viking://resources/a", "A");
        ctx.related_uri.push("viking://resources/b".into());
        ctx.related_uri.push("viking://resources/c".into());
        ctx.related_uri.push("viking://resources/d".into());
        assert_eq!(ctx.related_uri.len(), 3);
    }

    #[test]
    fn test_context_meta_nested_json() {
        let mut ctx = Context::new("viking://resources/meta", "Meta test");
        ctx.meta.insert("nested".into(), serde_json::json!({"a": {"b": 1}}));
        let json = serde_json::to_string(&ctx).unwrap();
        let ctx2: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx2.meta["nested"]["a"]["b"], 1);
    }




}
