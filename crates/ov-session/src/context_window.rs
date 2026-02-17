//! Context window management — L0/L1/L2 layered loading.

use crate::session::{Message, Session};
use serde::{Deserialize, Serialize};

/// Context layer levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextLayer {
    /// L0: One-line abstract.
    L0,
    /// L1: Medium detail overview.
    L1,
    /// L2: Full content.
    L2,
}

/// A context window entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub uri: String,
    pub layer: ContextLayer,
    pub content: String,
}

/// Context window manager.
pub struct ContextWindow {
    pub max_tokens: usize,
    entries: Vec<ContextEntry>,
    current_tokens: usize,
}

impl ContextWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            entries: Vec::new(),
            current_tokens: 0,
        }
    }

    /// Estimate tokens (chars / 4).
    fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }

    /// Add an entry if it fits.
    pub fn add(&mut self, entry: ContextEntry) -> bool {
        let tokens = Self::estimate_tokens(&entry.content);
        if self.current_tokens + tokens > self.max_tokens {
            return false;
        }
        self.current_tokens += tokens;
        self.entries.push(entry);
        true
    }

    /// Try to add at L2, fallback to L1, then L0.
    pub fn add_adaptive(
        &mut self,
        uri: &str,
        l0: &str,
        l1: &str,
        l2: &str,
    ) -> ContextLayer {
        let l2_entry = ContextEntry {
            uri: uri.to_string(),
            layer: ContextLayer::L2,
            content: l2.to_string(),
        };
        if self.add(l2_entry) {
            return ContextLayer::L2;
        }

        let l1_entry = ContextEntry {
            uri: uri.to_string(),
            layer: ContextLayer::L1,
            content: l1.to_string(),
        };
        if self.add(l1_entry) {
            return ContextLayer::L1;
        }

        let l0_entry = ContextEntry {
            uri: uri.to_string(),
            layer: ContextLayer::L0,
            content: l0.to_string(),
        };
        self.add(l0_entry);
        ContextLayer::L0
    }

    /// Get all entries.
    pub fn entries(&self) -> &[ContextEntry] {
        &self.entries
    }

    /// Get remaining token budget.
    pub fn remaining_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.current_tokens)
    }

    /// Get current token usage.
    pub fn used_tokens(&self) -> usize {
        self.current_tokens
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_tokens = 0;
    }

    /// Build context string for session.
    pub fn build_session_context(
        session: &Session,
        max_recent: usize,
        max_archives: usize,
        _query: &str,
    ) -> SessionContext {
        let recent: Vec<Message> = session.messages
            .iter()
            .rev()
            .take(max_recent)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        // Summaries from compression history would come from storage;
        // here we return what we have.
        let summaries = if !session.compression.summary.is_empty() {
            vec![session.compression.summary.clone()]
        } else {
            Vec::new()
        };

        SessionContext {
            recent_messages: recent,
            summaries,
            max_archives,
        }
    }
}

/// Session context for search/analysis.
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub recent_messages: Vec<Message>,
    pub summaries: Vec<String>,
    pub max_archives: usize,
}
