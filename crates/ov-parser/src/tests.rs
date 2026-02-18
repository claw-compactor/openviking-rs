use crate::*;
use crate::traits::DocumentParser;

// ========== Text Parser ==========

#[test]
fn test_text_parse_simple() {
    let parser = TextParser::new();
    let result = parser.parse_content("Hello world").unwrap();
    assert_eq!(result.chunks.len(), 1);
    assert_eq!(result.chunks[0].text, "Hello world");
}

#[test]
fn test_text_parse_paragraphs() {
    let parser = TextParser::new();
    let result = parser.parse_content("Para 1\n\nPara 2\n\nPara 3").unwrap();
    assert_eq!(result.chunks.len(), 3);
}

#[test]
fn test_text_parse_empty() {
    let parser = TextParser::new();
    let result = parser.parse_content("").unwrap();
    assert!(result.chunks.is_empty());
}

#[test]
fn test_text_parse_whitespace_only() {
    let parser = TextParser::new();
    let result = parser.parse_content("   \n\n   ").unwrap();
    assert!(result.chunks.is_empty());
}

#[test]
fn test_text_supported_extensions() {
    let parser = TextParser::new();
    assert!(parser.can_parse("file.txt"));
    assert!(!parser.can_parse("file.md"));
}

#[test]
fn test_text_unicode() {
    let parser = TextParser::new();
    let result = parser.parse_content("你好世界\n\nこんにちは").unwrap();
    assert_eq!(result.chunks.len(), 2);
}

// ========== Code Parser ==========

#[test]
fn test_code_parse_rust() {
    let parser = CodeParser::new();
    let code = "fn main() -> i32 {\n    println!(\"hello\");\n    0\n}\n\nfn helper() -> bool {\n    true\n}";
    let result = parser.parse_content(code).unwrap();
    assert!(result.chunks.len() >= 2);
    assert_eq!(result.metadata.get("language").unwrap(), "rust");
}

#[test]
fn test_code_parse_python() {
    let parser = CodeParser::new();
    let code = "def hello():\n    print('hi')\n\ndef world():\n    pass";
    let result = parser.parse_content(code).unwrap();
    assert!(result.chunks.len() >= 2);
    assert_eq!(result.metadata.get("language").unwrap(), "python");
}

#[test]
fn test_code_parse_no_structure() {
    let parser = CodeParser::new();
    let result = parser.parse_content("just some text").unwrap();
    assert_eq!(result.chunks.len(), 1);
}

#[test]
fn test_code_parse_empty() {
    let parser = CodeParser::new();
    let result = parser.parse_content("").unwrap();
    assert!(result.chunks.is_empty());
}

#[test]
fn test_code_parse_class() {
    let parser = CodeParser::new();
    let code = "class Foo:\n    pass\n\nclass Bar:\n    pass";
    let result = parser.parse_content(code).unwrap();
    assert!(result.chunks.len() >= 2);
}

#[test]
fn test_code_supported_extensions() {
    let parser = CodeParser::new();
    assert!(parser.can_parse("main.rs"));
    assert!(parser.can_parse("app.py"));
    assert!(!parser.can_parse("readme.md"));
}

#[test]
fn test_code_detect_js() {
    let parser = CodeParser::new();
    let code = "const x = 1;\nfunction hello() { return x; }";
    let result = parser.parse_content(code).unwrap();
    assert_eq!(result.metadata.get("language").unwrap(), "javascript");
}

// ========== Markdown Parser ==========

#[test]
fn test_md_parse_simple() {
    let parser = MarkdownParser::new();
    let result = parser.parse_content("# Title\n\nSome text").unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_md_parse_multiple_headings() {
    let parser = MarkdownParser::new();
    let md = "# H1\n\nText 1\n\n## H2\n\nText 2\n\n## H3\n\nText 3";
    let result = parser.parse_content(md).unwrap();
    assert!(result.chunks.len() >= 3);
}

#[test]
fn test_md_parse_frontmatter() {
    let parser = MarkdownParser::new();
    let md = "---\ntitle: Test\nauthor: Me\n---\n# Hello\n\nWorld";
    let result = parser.parse_content(md).unwrap();
    assert!(result.chunks.iter().any(|c| c.chunk_type == ChunkType::Frontmatter));
}

#[test]
fn test_md_parse_no_headings() {
    let parser = MarkdownParser::new();
    let result = parser.parse_content("Just plain text without headings.").unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_md_parse_code_blocks() {
    let parser = MarkdownParser::new();
    let md = "# Code\n\n```rust\nfn main() {}\n```\n\nMore text";
    let result = parser.parse_content(md).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_md_parse_empty() {
    let parser = MarkdownParser::new();
    let result = parser.parse_content("").unwrap();
    assert!(result.chunks.is_empty());
}

#[test]
fn test_md_heading_inside_code_block() {
    let parser = MarkdownParser::new();
    let md = "# Real\n\n```\n# Not a heading\n```\n\n## Also Real";
    let headings = parser.find_headings(md);
    assert_eq!(headings.len(), 2);
}

#[test]
fn test_md_extract_frontmatter() {
    let parser = MarkdownParser::new();
    let (content, fm) = parser.extract_frontmatter("---\nkey: val\n---\nBody");
    assert_eq!(content, "Body");
    assert!(fm.is_some());
    assert_eq!(fm.unwrap().get("key").unwrap(), "val");
}

#[test]
fn test_md_no_frontmatter() {
    let parser = MarkdownParser::new();
    let (content, fm) = parser.extract_frontmatter("# Title\n\nBody");
    assert!(fm.is_none());
    assert!(content.contains("Title"));
}

#[test]
fn test_md_supported_extensions() {
    let parser = MarkdownParser::new();
    assert!(parser.can_parse("readme.md"));
    assert!(parser.can_parse("doc.markdown"));
    assert!(!parser.can_parse("code.rs"));
}

#[test]
fn test_md_smart_split() {
    let parser = MarkdownParser::new();
    let text = (0..100).map(|i| format!("Paragraph {}. ", i)).collect::<String>();
    let parts = parser.smart_split(&text, 50);
    assert!(parts.len() > 1);
}

#[test]
fn test_md_unicode_headings() {
    let parser = MarkdownParser::new();
    let md = "# 中文标题\n\n内容\n\n## 日本語\n\nコンテンツ";
    let result = parser.parse_content(md).unwrap();
    assert!(result.chunks.len() >= 2);
}

// ========== Chunker ==========

#[test]
fn test_chunker_fixed_small() {
    let chunker = TextChunker::new(1000, 0);
    let chunks = chunker.chunk_fixed("Hello world");
    assert_eq!(chunks.len(), 1);
}

#[test]
fn test_chunker_fixed_large() {
    let chunker = TextChunker::new(10, 0);
    let text = (0..100).map(|i| format!("Sentence number {}. ", i)).collect::<String>();
    let chunks = chunker.chunk_fixed(&text);
    assert!(!chunks.is_empty());
}

#[test]
fn test_chunker_fixed_empty() {
    let chunker = TextChunker::new(100, 0);
    let chunks = chunker.chunk_fixed("");
    assert!(chunks.is_empty());
}

#[test]
fn test_chunker_semantic() {
    let chunker = TextChunker::new(20, 0);
    let text = "Para one.\n\nPara two.\n\nPara three.";
    let chunks = chunker.chunk_semantic(text);
    assert!(!chunks.is_empty());
}

#[test]
fn test_chunker_semantic_empty() {
    let chunker = TextChunker::new(100, 0);
    let chunks = chunker.chunk_semantic("");
    assert!(chunks.is_empty());
}

#[test]
fn test_chunker_overlap() {
    let chunker = TextChunker::new(10, 5);
    let text = (0..50).map(|i| format!("Word{}. ", i)).collect::<String>();
    let chunks = chunker.chunk_fixed(&text);
    assert!(!chunks.is_empty());
}

#[test]
fn test_chunker_default() {
    let chunker = TextChunker::default();
    assert_eq!(chunker.max_chunk_tokens, 512);
    assert_eq!(chunker.overlap_tokens, 50);
}

// ========== Token Estimation ==========

#[test]
fn test_estimate_tokens_english() {
    let t = estimate_tokens("Hello world this is a test");
    assert!(t > 0);
    assert!(t < 100);
}

#[test]
fn test_estimate_tokens_chinese() {
    let t = estimate_tokens("你好世界这是测试");
    assert!(t > 0);
}

#[test]
fn test_estimate_tokens_empty() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn test_estimate_tokens_mixed() {
    let t = estimate_tokens("Hello 你好 World 世界");
    assert!(t > 0);
}

// ========== ParseResult ==========

#[test]
fn test_parse_result_total_tokens() {
    let mut r = ParseResult::new("test", "text");
    r.chunks.push(Chunk::new("hello world", ChunkType::Text));
    r.chunks.push(Chunk::new("more text here", ChunkType::Text));
    assert!(r.total_tokens() > 0);
}

// ========== Chunk ==========

#[test]
fn test_chunk_with_meta() {
    let c = Chunk::new("test", ChunkType::Code).with_meta("lang", "rust");
    assert_eq!(c.metadata.get("lang").unwrap(), "rust");
}

#[test]
fn test_chunk_with_offsets() {
    let c = Chunk::new("test", ChunkType::Text).with_offsets(10, 20);
    assert_eq!(c.start_offset, 10);
    assert_eq!(c.end_offset, 20);
}

// ========== Extended Parser Tests ==========

#[test]
fn test_text_parse_single_line() {
    let parser = TextParser::new();
    let result = parser.parse_content("Single line without newlines").unwrap();
    assert_eq!(result.chunks.len(), 1);
}

#[test]
fn test_text_parse_many_paragraphs() {
    let parser = TextParser::new();
    let text = (0..20).map(|i| format!("Paragraph {}", i)).collect::<Vec<_>>().join("\n\n");
    let result = parser.parse_content(&text).unwrap();
    assert_eq!(result.chunks.len(), 20);
}

#[test]
fn test_code_parse_multiple_languages() {
    let parser = CodeParser::new();
    // Test with various language constructs
    let go_code = "func main() {\n    fmt.Println(\"hello\")\n}";
    let result = parser.parse_content(go_code).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_code_parse_nested_functions() {
    let parser = CodeParser::new();
    let code = "fn outer() {\n    fn inner() {\n        println!(\"nested\");\n    }\n    inner();\n}";
    let result = parser.parse_content(code).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_md_parse_only_headings() {
    let parser = MarkdownParser::new();
    let md = "# H1\n## H2\n### H3";
    let result = parser.parse_content(md).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_md_parse_deep_headings() {
    let parser = MarkdownParser::new();
    let md = "# H1\n\n## H2\n\n### H3\n\n#### H4\n\n##### H5\n\n###### H6\n\nDeepest level";
    let result = parser.parse_content(md).unwrap();
    assert!(result.chunks.len() >= 6);
}

#[test]
fn test_md_parse_lists() {
    let parser = MarkdownParser::new();
    let md = "# List\n\n- Item 1\n- Item 2\n- Item 3\n\n1. Ordered 1\n2. Ordered 2";
    let result = parser.parse_content(md).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_md_parse_table() {
    let parser = MarkdownParser::new();
    let md = "# Table\n\n| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
    let result = parser.parse_content(md).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_md_parse_links_and_images() {
    let parser = MarkdownParser::new();
    let md = "# Doc\n\n[Link](https://example.com)\n\n![Image](image.png)";
    let result = parser.parse_content(md).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn test_chunker_very_large_text() {
    let chunker = TextChunker::new(100, 10);
    let text = "word ".repeat(10000);
    let chunks = chunker.chunk_fixed(&text);
    assert!(!chunks.is_empty());
    // All chunks should be non-empty
    for c in &chunks {
        assert!(!c.text.is_empty());
    }
}

#[test]
fn test_chunker_single_word() {
    let chunker = TextChunker::new(100, 0);
    let chunks = chunker.chunk_fixed("hello");
    assert_eq!(chunks.len(), 1);
}

#[test]
fn test_estimate_tokens_code() {
    let code = "fn main() { println!(\"hello\"); }";
    let t = estimate_tokens(code);
    assert!(t > 0 && t < 50);
}

#[test]
fn test_estimate_tokens_long_text() {
    let text = "word ".repeat(1000);
    let t = estimate_tokens(&text);
    assert!(t >= 500 && t <= 2000);
}

#[test]
fn test_parse_result_metadata() {
    let mut r = ParseResult::new("test.md", "markdown");
    assert_eq!(r.parser_name, "test.md");
    assert_eq!(r.source_format, "markdown");
    r.metadata.insert("key".into(), "value".into());
    assert_eq!(r.metadata["key"], "value");
}

#[test]
fn test_chunk_types() {
    let c1 = Chunk::new("text", ChunkType::Text);
    let c2 = Chunk::new("code", ChunkType::Code);
    let c3 = Chunk::new("front", ChunkType::Frontmatter);
    assert_eq!(c1.chunk_type, ChunkType::Text);
    assert_eq!(c2.chunk_type, ChunkType::Code);
    assert_eq!(c3.chunk_type, ChunkType::Frontmatter);
}

#[test]
fn test_chunk_token_count() {
    let c = Chunk::new("hello world this is a test", ChunkType::Text);
    assert!(!c.text.is_empty());
}

#[test]
fn test_text_parser_long_lines() {
    let parser = TextParser::new();
    let long_line = "x".repeat(10000);
    let result = parser.parse_content(&long_line).unwrap();
    assert_eq!(result.chunks.len(), 1);
}

#[test]
fn test_code_parse_typescript() {
    let parser = CodeParser::new();
    assert!(parser.can_parse("file.ts"));
    // tsx not necessarily supported
}

#[test]
fn test_md_parse_html_in_markdown() {
    let parser = MarkdownParser::new();
    let md = "# Title\n\n<div>HTML content</div>\n\nRegular text";
    let result = parser.parse_content(md).unwrap();
    assert!(!result.chunks.is_empty());
}
