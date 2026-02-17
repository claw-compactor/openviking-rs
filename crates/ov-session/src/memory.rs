//! Memory extraction from session conversations.
//!
//! Provides 6-category memory classification and candidate extraction.

use serde::{Deserialize, Serialize};
use crate::session::Message;

/// Memory category (6 types).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    Profile,
    Preferences,
    Entities,
    Events,
    Cases,
    Patterns,
}

impl MemoryCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Profile => "profile",
            Self::Preferences => "preferences",
            Self::Entities => "entities",
            Self::Events => "events",
            Self::Cases => "cases",
            Self::Patterns => "patterns",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "profile" => Self::Profile,
            "preferences" => Self::Preferences,
            "entities" => Self::Entities,
            "events" => Self::Events,
            "cases" => Self::Cases,
            "patterns" => Self::Patterns,
            _ => Self::Patterns,
        }
    }

    pub fn directory(&self) -> &str {
        match self {
            Self::Profile => "memories/profile.md",
            Self::Preferences => "memories/preferences",
            Self::Entities => "memories/entities",
            Self::Events => "memories/events",
            Self::Cases => "memories/cases",
            Self::Patterns => "memories/patterns",
        }
    }

    /// Categories that always merge (skip dedup).
    pub fn always_merge(&self) -> bool {
        matches!(self, Self::Profile)
    }

    /// Categories that support merge decisions.
    pub fn supports_merge(&self) -> bool {
        matches!(self, Self::Preferences | Self::Entities | Self::Patterns)
    }
}

/// Candidate memory extracted from session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMemory {
    pub category: MemoryCategory,
    pub abstract_text: String,
    pub overview: String,
    pub content: String,
    pub source_session: String,
    pub user: String,
    pub language: String,
}

/// Deduplication decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DedupDecision {
    Create,
    Merge,
    Skip,
}

/// Result of deduplication.
#[derive(Debug, Clone)]
pub struct DedupResult {
    pub decision: DedupDecision,
    pub reason: String,
}

/// Detect dominant language from user messages.
pub fn detect_language(messages: &[Message]) -> String {
    let user_text: String = messages
        .iter()
        .filter(|m| m.role == crate::session::Role::User)
        .map(|m| m.content())
        .collect::<Vec<_>>()
        .join("\n");

    if user_text.is_empty() {
        return "en".into();
    }

    // Check scripts
    let ko = user_text.chars().filter(|c| ('\u{ac00}'..='\u{d7af}').contains(c)).count();
    let ru = user_text.chars().filter(|c| ('\u{0400}'..='\u{04ff}').contains(c)).count();
    let ar = user_text.chars().filter(|c| ('\u{0600}'..='\u{06ff}').contains(c)).count();
    let kana = user_text.chars().filter(|c| ('\u{3040}'..='\u{30ff}').contains(c)).count();
    let han = user_text.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)).count();

    if ko > 0 { return "ko".into(); }
    if ru > 0 { return "ru".into(); }
    if ar > 0 { return "ar".into(); }
    if kana > 0 { return "ja".into(); }
    if han > 0 { return "zh-CN".into(); }

    "en".into()
}

/// Simple rule-based memory extraction (no LLM).
/// Extracts candidate memories from messages based on heuristics.
pub fn extract_candidates(
    messages: &[Message],
    session_id: &str,
    user_id: &str,
) -> Vec<CandidateMemory> {
    if messages.is_empty() {
        return Vec::new();
    }

    let language = detect_language(messages);
    let mut candidates = Vec::new();

    // Extract from user messages
    for msg in messages.iter().filter(|m| m.role == crate::session::Role::User) {
        let content = msg.content();
        if content.len() < 10 { continue; }

        // Heuristic: messages mentioning preferences
        let lower = content.to_lowercase();
        let category = if lower.contains("prefer") || lower.contains("偏好") || lower.contains("like") {
            MemoryCategory::Preferences
        } else if lower.contains("my name") || lower.contains("i am") || lower.contains("我是") {
            MemoryCategory::Profile
        } else if lower.contains("project") || lower.contains("项目") {
            MemoryCategory::Entities
        } else if lower.contains("decided") || lower.contains("决定") {
            MemoryCategory::Events
        } else if lower.contains("error") || lower.contains("fix") || lower.contains("bug") {
            MemoryCategory::Cases
        } else {
            continue; // Not memory-worthy
        };

        let abstract_text = if content.len() > 80 {
            format!("{}...", &content[..80])
        } else {
            content.clone()
        };

        candidates.push(CandidateMemory {
            category,
            abstract_text,
            overview: content.clone(),
            content,
            source_session: session_id.to_string(),
            user: user_id.to_string(),
            language: language.clone(),
        });
    }

    candidates
}

/// Extraction statistics.
#[derive(Debug, Clone, Default)]
pub struct ExtractionStats {
    pub created: usize,
    pub merged: usize,
    pub skipped: usize,
}
