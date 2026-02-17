//! Code parser with syntax-aware splitting.

use anyhow::Result;
use regex::Regex;
use crate::{Chunk, ChunkType, ParseResult, traits::DocumentParser};

/// Code parser — splits by functions/classes/top-level blocks.
pub struct CodeParser {
    function_re: Regex,
    class_re: Regex,
}

impl CodeParser {
    pub fn new() -> Self {
        Self {
            function_re: Regex::new(
                r"(?m)^(?:pub\s+)?(?:async\s+)?(?:fn|def|function|func)\s+\w+"
            ).unwrap(),
            class_re: Regex::new(
                r"(?m)^(?:pub\s+)?(?:class|struct|enum|trait|interface|impl)\s+\w+"
            ).unwrap(),
        }
    }

    fn detect_language(&self, content: &str) -> String {
        if content.contains("fn ") && content.contains("->") {
            "rust".into()
        } else if content.contains("def ") && content.contains(":") {
            "python".into()
        } else if content.contains("function ") || content.contains("const ") {
            "javascript".into()
        } else if content.contains("func ") {
            "go".into()
        } else {
            "unknown".into()
        }
    }
}

impl DocumentParser for CodeParser {
    fn parse_content(&self, content: &str) -> Result<ParseResult> {
        let lang = self.detect_language(content);
        let mut result = ParseResult::new("CodeParser", "code");
        result.metadata.insert("language".into(), lang);

        // Find all function/class boundaries
        let mut boundaries: Vec<(usize, &str)> = Vec::new();
        for m in self.function_re.find_iter(content) {
            boundaries.push((m.start(), "function"));
        }
        for m in self.class_re.find_iter(content) {
            boundaries.push((m.start(), "class"));
        }
        boundaries.sort_by_key(|b| b.0);

        if boundaries.is_empty() {
            // No structure found, return whole content as one chunk
            if !content.trim().is_empty() {
                result.chunks.push(
                    Chunk::new(content.trim(), ChunkType::Code)
                        .with_offsets(0, content.len())
                );
            }
            return Ok(result);
        }

        // Pre-boundary content
        if boundaries[0].0 > 0 {
            let pre = content[..boundaries[0].0].trim();
            if !pre.is_empty() {
                result.chunks.push(
                    Chunk::new(pre, ChunkType::Code)
                        .with_offsets(0, boundaries[0].0)
                );
            }
        }

        // Each boundary to next
        for i in 0..boundaries.len() {
            let start = boundaries[i].0;
            let end = if i + 1 < boundaries.len() {
                boundaries[i + 1].0
            } else {
                content.len()
            };
            let text = content[start..end].trim();
            if !text.is_empty() {
                result.chunks.push(
                    Chunk::new(text, ChunkType::Code)
                        .with_offsets(start, end)
                        .with_meta("kind", boundaries[i].1)
                );
            }
        }

        Ok(result)
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec![
            ".rs".into(), ".py".into(), ".js".into(), ".ts".into(),
            ".go".into(), ".java".into(), ".cpp".into(), ".c".into(),
            ".rb".into(), ".swift".into(),
        ]
    }
}

impl Default for CodeParser {
    fn default() -> Self { Self::new() }
}
