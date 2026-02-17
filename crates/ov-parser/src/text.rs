//! Plain text parser.

use anyhow::Result;
use crate::{Chunk, ChunkType, ParseResult, traits::DocumentParser};

/// Plain text parser — splits by paragraphs.
pub struct TextParser;

impl TextParser {
    pub fn new() -> Self { Self }
}

impl DocumentParser for TextParser {
    fn parse_content(&self, content: &str) -> Result<ParseResult> {
        let mut result = ParseResult::new("TextParser", "text");
        let mut offset = 0;

        for para in content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() {
                offset += para.len() + 2;
                continue;
            }
            let chunk = Chunk::new(trimmed, ChunkType::Paragraph)
                .with_offsets(offset, offset + para.len());
            result.chunks.push(chunk);
            offset += para.len() + 2;
        }

        Ok(result)
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec![".txt".into(), ".text".into()]
    }
}

impl Default for TextParser {
    fn default() -> Self { Self::new() }
}
