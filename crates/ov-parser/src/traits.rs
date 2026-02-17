use anyhow::Result;
use crate::ParseResult;

/// Trait for document parsers.
pub trait DocumentParser: Send + Sync {
    /// Parse content string.
    fn parse_content(&self, content: &str) -> Result<ParseResult>;

    /// Parse from file path.
    fn parse_file(&self, path: &str) -> Result<ParseResult> {
        let content = std::fs::read_to_string(path)?;
        self.parse_content(&content)
    }

    /// Supported file extensions.
    fn supported_extensions(&self) -> Vec<String>;

    /// Check if a file can be parsed.
    fn can_parse(&self, path: &str) -> bool {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let ext_with_dot = format!(".{}", ext);
        self.supported_extensions().contains(&ext_with_dot)
    }
}
