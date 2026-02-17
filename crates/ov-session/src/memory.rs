//! Memory extraction from conversations

use ov_core::context::Context;

/// Extract memories from a conversation turn
pub async fn extract_memories(_session_id: &str, _messages: &[serde_json::Value]) -> anyhow::Result<Vec<Context>> {
    // Placeholder: will use LLM to extract memories
    Ok(Vec::new())
}
