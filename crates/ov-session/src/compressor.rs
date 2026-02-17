//! Session compressor — handles compression of long sessions.

use crate::session::{Message, Role, Session};
use crate::memory::{CandidateMemory, ExtractionStats, extract_candidates};

/// Session compressor with configurable thresholds.
pub struct SessionCompressor {
    pub max_messages: usize,
    pub summary_ratio: f64,
}

impl SessionCompressor {
    pub fn new() -> Self {
        Self {
            max_messages: 100,
            summary_ratio: 0.3,
        }
    }

    pub fn with_max_messages(mut self, max: usize) -> Self {
        self.max_messages = max;
        self
    }

    /// Compress a session's messages if needed.
    /// Returns (kept_messages, summary_of_removed).
    pub fn compress(&self, messages: &[Message]) -> (Vec<Message>, Option<String>) {
        if messages.len() <= self.max_messages {
            return (messages.to_vec(), None);
        }

        let keep_count = (messages.len() as f64 * self.summary_ratio) as usize;
        let keep_count = keep_count.max(1);
        let remove_count = messages.len() - keep_count;

        let removed = &messages[..remove_count];
        let kept = &messages[remove_count..];

        let summary = self.generate_summary(removed);

        (kept.to_vec(), Some(summary))
    }

    /// Generate a text summary of messages.
    pub fn generate_summary(&self, messages: &[Message]) -> String {
        let turn_count = messages.iter().filter(|m| m.role == Role::User).count();
        let total = messages.len();

        let mut topics: Vec<String> = Vec::new();
        for msg in messages.iter().filter(|m| m.role == Role::User) {
            let content = msg.content();
            if content.len() > 50 {
                topics.push(format!("{}...", &content[..50]));
            } else if !content.is_empty() {
                topics.push(content);
            }
        }

        let topics_str = if topics.len() > 5 {
            let first_five = topics[..5].join("; ");
            format!("{} and {} more", first_five, topics.len() - 5)
        } else {
            topics.join("; ")
        };

        format!(
            "# Compressed Session Archive\n\n\
            **Overview**: {} turns, {} messages\n\
            **Topics**: {}\n",
            turn_count, total, topics_str
        )
    }

    /// Extract memories from committed messages.
    pub fn extract_memories(
        &self,
        messages: &[Message],
        session_id: &str,
        user_id: &str,
    ) -> (Vec<CandidateMemory>, ExtractionStats) {
        let candidates = extract_candidates(messages, session_id, user_id);
        let stats = ExtractionStats {
            created: candidates.len(),
            ..Default::default()
        };
        (candidates, stats)
    }
}

impl Default for SessionCompressor {
    fn default() -> Self {
        Self::new()
    }
}
