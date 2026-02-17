use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Resource content type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResourceContentType {
    Text,
    Image,
    Video,
    Audio,
    Binary,
}

/// Context type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContextType {
    Skill,
    Memory,
    Resource,
}

/// Vectorization payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vectorize {
    pub text: String,
}

/// Unified context — the core data record in OpenViking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub id: Uuid,
    pub uri: String,
    pub parent_uri: Option<String>,
    pub is_leaf: bool,
    pub abstract_text: String,
    pub context_type: ContextType,
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active_count: u64,
    pub related_uri: Vec<String>,
    pub meta: HashMap<String, serde_json::Value>,
    pub session_id: Option<String>,
    pub vector: Option<Vec<f32>>,
    #[serde(skip)]
    pub vectorize: Vectorize,
}

impl Context {
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

    pub fn update_activity(&mut self) {
        self.active_count += 1;
        self.updated_at = Utc::now();
    }

    fn derive_context_type(uri: &str) -> ContextType {
        if uri.starts_with("viking://agent/skills") {
            ContextType::Skill
        } else if uri.contains("memories") {
            ContextType::Memory
        } else {
            ContextType::Resource
        }
    }

    fn derive_category(uri: &str) -> String {
        if uri.starts_with("viking://agent/memories") {
            if uri.contains("patterns") { return "patterns".into(); }
            if uri.contains("cases") { return "cases".into(); }
        } else if uri.starts_with("viking://user/memories") {
            if uri.contains("profile") { return "profile".into(); }
            if uri.contains("preferences") { return "preferences".into(); }
            if uri.contains("entities") { return "entities".into(); }
            if uri.contains("events") { return "events".into(); }
        }
        String::new()
    }
}
