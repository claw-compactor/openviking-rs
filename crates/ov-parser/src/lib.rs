//! Document parsing for OpenViking.
//!
//! Provides text, code, and markdown parsers with chunking support.

pub mod traits;
pub mod text;
pub mod code;
pub mod markdown;
pub mod chunker;

pub use traits::*;
pub use text::TextParser;
pub use code::CodeParser;
pub use markdown::MarkdownParser;
pub use chunker::TextChunker;

/// Parsed document chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub text: String,
    pub chunk_type: ChunkType,
    pub start_offset: usize,
    pub end_offset: usize,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Chunk {
    pub fn new(text: impl Into<String>, chunk_type: ChunkType) -> Self {
        let t = text.into();
        let len = t.len();
        Self {
            text: t,
            chunk_type,
            start_offset: 0,
            end_offset: len,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_offsets(mut self, start: usize, end: usize) -> Self {
        self.start_offset = start;
        self.end_offset = end;
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn token_estimate(&self) -> usize {
        estimate_tokens(&self.text)
    }
}

/// Chunk type classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkType {
    Text,
    Code,
    Heading,
    ListItem,
    Table,
    Frontmatter,
    Paragraph,
}

/// Parse result from any parser.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub chunks: Vec<Chunk>,
    pub source_format: String,
    pub parser_name: String,
    pub warnings: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ParseResult {
    pub fn new(parser_name: &str, source_format: &str) -> Self {
        Self {
            chunks: Vec::new(),
            source_format: source_format.into(),
            parser_name: parser_name.into(),
            warnings: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn total_tokens(&self) -> usize {
        self.chunks.iter().map(|c| c.token_estimate()).sum()
    }
}

/// Estimate token count.
pub fn estimate_tokens(text: &str) -> usize {
    let cjk: usize = text.chars()
        .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)
            || ('\u{3040}'..='\u{30ff}').contains(c)
            || ('\u{ac00}'..='\u{d7af}').contains(c))
        .count();
    let other: usize = text.chars()
        .filter(|c| !c.is_whitespace()
            && !('\u{4e00}'..='\u{9fff}').contains(c)
            && !('\u{3040}'..='\u{30ff}').contains(c)
            && !('\u{ac00}'..='\u{d7af}').contains(c))
        .count();
    ((cjk as f64 * 0.7) + (other as f64 * 0.3)) as usize
}

#[cfg(test)]
mod tests;
