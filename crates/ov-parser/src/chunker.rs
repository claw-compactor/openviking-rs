//! Text chunker — fixed-size and semantic splitting.

use crate::{Chunk, ChunkType, estimate_tokens};

/// Text chunker with configurable parameters.
pub struct TextChunker {
    pub max_chunk_tokens: usize,
    pub overlap_tokens: usize,
}

impl TextChunker {
    pub fn new(max_chunk_tokens: usize, overlap_tokens: usize) -> Self {
        Self { max_chunk_tokens, overlap_tokens }
    }

    /// Fixed-size chunking by token estimate.
    pub fn chunk_fixed(&self, text: &str) -> Vec<Chunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let total_tokens = estimate_tokens(text);
        if total_tokens <= self.max_chunk_tokens {
            return vec![Chunk::new(text.trim(), ChunkType::Text)];
        }

        // Split by sentences first
        let sentences = self.split_sentences(text);
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut current_tokens = 0;

        for sentence in &sentences {
            let st = estimate_tokens(sentence);
            if current_tokens + st > self.max_chunk_tokens && !current.is_empty() {
                chunks.push(Chunk::new(current.trim(), ChunkType::Text));
                // Overlap: keep last portion
                current = self.get_overlap_text(&current);
                current_tokens = estimate_tokens(&current);
            }
            current.push_str(sentence);
            current.push(' ');
            current_tokens += st;
        }
        if !current.trim().is_empty() {
            chunks.push(Chunk::new(current.trim(), ChunkType::Text));
        }
        chunks
    }

    /// Semantic chunking — splits on paragraph boundaries.
    pub fn chunk_semantic(&self, text: &str) -> Vec<Chunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut current_tokens = 0;

        for para in paragraphs {
            let para = para.trim();
            if para.is_empty() { continue; }
            let pt = estimate_tokens(para);

            if pt > self.max_chunk_tokens {
                // Paragraph too large, sub-chunk it
                if !current.is_empty() {
                    chunks.push(Chunk::new(current.trim(), ChunkType::Paragraph));
                    current = String::new();
                    current_tokens = 0;
                }
                for sub in self.chunk_fixed(para) {
                    chunks.push(sub);
                }
                continue;
            }

            if current_tokens + pt > self.max_chunk_tokens && !current.is_empty() {
                chunks.push(Chunk::new(current.trim(), ChunkType::Paragraph));
                current = String::new();
                current_tokens = 0;
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
            current_tokens += pt;
        }
        if !current.trim().is_empty() {
            chunks.push(Chunk::new(current.trim(), ChunkType::Paragraph));
        }
        chunks
    }

    fn split_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        for c in text.chars() {
            current.push(c);
            if c == '.' || c == '!' || c == '?' || c == '\n' {
                if !current.trim().is_empty() {
                    sentences.push(current.trim().to_string());
                }
                current = String::new();
            }
        }
        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }
        sentences
    }

    fn get_overlap_text(&self, text: &str) -> String {
        if self.overlap_tokens == 0 {
            return String::new();
        }
        // Take last N tokens worth of text (approximate)
        let chars_approx = self.overlap_tokens * 4;
        if text.len() <= chars_approx {
            return text.to_string();
        }
        text[text.len() - chars_approx..].to_string()
    }
}

impl Default for TextChunker {
    fn default() -> Self {
        Self::new(512, 50)
    }
}
