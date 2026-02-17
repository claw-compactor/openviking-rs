//! Markdown parser with structure-aware splitting.

use anyhow::Result;
use regex::Regex;
use crate::{Chunk, ChunkType, ParseResult, estimate_tokens, traits::DocumentParser};
use std::collections::HashMap;

/// Markdown parser.
pub struct MarkdownParser {
    heading_re: Regex,
    code_block_re: Regex,
    frontmatter_re: Regex,
    pub max_section_size: usize,
    pub min_section_tokens: usize,
}

impl MarkdownParser {
    pub fn new() -> Self {
        Self {
            heading_re: Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap(),
            code_block_re: Regex::new(r"(?s)```(\w*)\n(.*?)```").unwrap(),
            frontmatter_re: Regex::new(r"(?s)^---\n(.*?)\n---\n").unwrap(),
            max_section_size: 1024,
            min_section_tokens: 512,
        }
    }

    /// Extract frontmatter.
    pub fn extract_frontmatter<'a>(&self, content: &'a str) -> (&'a str, Option<HashMap<String, String>>) {
        if let Some(m) = self.frontmatter_re.find(content) {
            let fm_text = &content[4..m.end() - 5]; // skip --- and ---\n
            let mut fm = HashMap::new();
            for line in fm_text.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    fm.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
            (&content[m.end()..], Some(fm))
        } else {
            (content, None)
        }
    }

    /// Find headings (excluding those inside code blocks).
    pub fn find_headings(&self, content: &str) -> Vec<(usize, usize, String, usize)> {
        // Collect code block ranges
        let excluded: Vec<(usize, usize)> = self.code_block_re
            .find_iter(content)
            .map(|m| (m.start(), m.end()))
            .collect();

        self.heading_re
            .captures_iter(content)
            .filter_map(|cap| {
                let m = cap.get(0)?;
                let pos = m.start();
                if excluded.iter().any(|(s, e)| pos >= *s && pos < *e) {
                    return None;
                }
                if pos > 0 && content.as_bytes()[pos - 1] == b'\\' {
                    return None;
                }
                let level = cap[1].len();
                let title = cap[2].trim().to_string();
                Some((m.start(), m.end(), title, level))
            })
            .collect()
    }

    /// Split content by paragraphs for oversized sections.
    pub fn smart_split(&self, content: &str, max_size: usize) -> Vec<String> {
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut current_tokens = 0;

        for para in paragraphs {
            let para_tokens = estimate_tokens(para);
            if para_tokens > max_size {
                if !current.is_empty() {
                    parts.push(current.trim().to_string());
                    current = String::new();
                    current_tokens = 0;
                }
                // Force split by chars
                let char_size = max_size * 3;
                let chars: Vec<char> = para.chars().collect();
                for chunk in chars.chunks(char_size) {
                    let s: String = chunk.iter().collect();
                    parts.push(s.trim().to_string());
                }
            } else if current_tokens + para_tokens > max_size && !current.is_empty() {
                parts.push(current.trim().to_string());
                current = para.to_string();
                current_tokens = para_tokens;
            } else {
                if !current.is_empty() {
                    current.push_str("\n\n");
                }
                current.push_str(para);
                current_tokens += para_tokens;
            }
        }
        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }
        if parts.is_empty() {
            parts.push(content.to_string());
        }
        parts
    }
}

impl DocumentParser for MarkdownParser {
    fn parse_content(&self, content: &str) -> Result<ParseResult> {
        let mut result = ParseResult::new("MarkdownParser", "markdown");

        // Extract frontmatter
        let (content, frontmatter) = self.extract_frontmatter(content);
        if let Some(fm) = frontmatter {
            let fm_str = fm.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("\n");
            result.chunks.push(Chunk::new(fm_str, ChunkType::Frontmatter));
        }

        let headings = self.find_headings(content);

        if headings.is_empty() {
            // No headings — split by paragraphs
            for part in self.smart_split(content, self.max_section_size) {
                if !part.is_empty() {
                    result.chunks.push(Chunk::new(part, ChunkType::Paragraph));
                }
            }
            return Ok(result);
        }

        // Pre-heading content
        if headings[0].0 > 0 {
            let pre = content[..headings[0].0].trim();
            if !pre.is_empty() {
                result.chunks.push(Chunk::new(pre, ChunkType::Paragraph));
            }
        }

        // Split by headings
        for i in 0..headings.len() {
            let (start, _, ref title, level) = headings[i];
            let end = if i + 1 < headings.len() {
                headings[i + 1].0
            } else {
                content.len()
            };

            let section = content[start..end].trim();
            let tokens = estimate_tokens(section);

            if tokens > self.max_section_size {
                // Split oversized sections
                for part in self.smart_split(section, self.max_section_size) {
                    result.chunks.push(
                        Chunk::new(part, ChunkType::Heading)
                            .with_meta("heading", title)
                            .with_meta("level", &level.to_string())
                    );
                }
            } else {
                result.chunks.push(
                    Chunk::new(section, ChunkType::Heading)
                        .with_offsets(start, end)
                        .with_meta("heading", title)
                        .with_meta("level", &level.to_string())
                );
            }
        }

        Ok(result)
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec![".md".into(), ".markdown".into(), ".mdown".into(), ".mkd".into()]
    }
}

impl Default for MarkdownParser {
    fn default() -> Self { Self::new() }
}
