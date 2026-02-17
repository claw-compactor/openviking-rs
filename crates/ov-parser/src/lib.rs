//! Document parsing for OpenViking

pub mod traits;

use anyhow::Result;

/// Parsed document chunk
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub metadata: serde_json::Value,
}

/// Parse text into chunks
pub fn parse_text(input: &str, _max_chunk_size: usize) -> Result<Vec<Chunk>> {
    // Simple placeholder: one chunk per paragraph
    Ok(input
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
        .map(|s| Chunk {
            text: s.trim().to_string(),
            metadata: serde_json::Value::Null,
        })
        .collect())
}
