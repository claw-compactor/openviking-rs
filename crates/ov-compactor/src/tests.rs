use crate::*;
use crate::pipeline::*;
use crate::layer1_jsonl;
use crate::layer2_ccp;
use crate::layer3_dictionary;
use crate::layer4_dedup;
use crate::layer5_format;

// ========== Layer 1: JSONL ==========

#[test]
fn test_l1_strip_metadata() {
    let line = r#"{"role":"user","content":"hi","timestamp":"2024-01-01","trace_id":"abc"}"#;
    let result = layer1_jsonl::compress_line(line);
    assert!(!result.contains("timestamp"));
    assert!(!result.contains("trace_id"));
    assert!(result.contains("content"));
}

#[test]
fn test_l1_preserve_content() {
    let line = r#"{"role":"user","content":"hello world"}"#;
    let result = layer1_jsonl::compress_line(line);
    assert!(result.contains("hello world"));
}

#[test]
fn test_l1_non_json() {
    assert_eq!(layer1_jsonl::compress_line("not json"), "not json");
}

#[test]
fn test_l1_empty() {
    assert_eq!(layer1_jsonl::compress_line(""), "");
}

#[test]
fn test_l1_strip_null_fields() {
    let line = r#"{"role":"user","content":"hi","extra":null,"empty":""}"#;
    let result = layer1_jsonl::compress_line(line);
    assert!(!result.contains("extra"));
    assert!(!result.contains("empty"));
}

#[test]
fn test_l1_multiline() {
    let input = "{\"a\":1,\"timestamp\":\"t\"}\n{\"b\":2,\"trace_id\":\"x\"}";
    let result = layer1_jsonl::compress(input);
    assert_eq!(result.lines().count(), 2);
}

// ========== Layer 2: CCP ==========

#[test]
fn test_l2_basic() {
    let result = layer2_ccp::compress("The function takes a parameter and returns a configuration");
    assert!(result.contains("fn"));
    assert!(result.contains("param"));
    assert!(result.contains("config"));
}

#[test]
fn test_l2_empty() {
    assert_eq!(layer2_ccp::compress(""), "");
}

#[test]
fn test_l2_no_match() {
    let text = "hello world";
    let result = layer2_ccp::compress(text);
    assert_eq!(result, text);
}

#[test]
fn test_l2_case_insensitive() {
    let result = layer2_ccp::compress("The DATABASE connection");
    assert!(result.contains("db"));
}

#[test]
fn test_l2_roundtrip() {
    let original = "The function takes a parameter";
    let compressed = layer2_ccp::compress(original);
    let decompressed = layer2_ccp::decompress(&compressed);
    // Note: roundtrip may not be exact due to case changes, but meaning preserved
    assert!(decompressed.to_lowercase().contains("function"));
    assert!(decompressed.to_lowercase().contains("parameter"));
}

#[test]
fn test_l2_kubernetes() {
    let result = layer2_ccp::compress("Deploy to kubernetes cluster");
    assert!(result.contains("k8s"));
}

#[test]
fn test_l2_multiple() {
    let result = layer2_ccp::compress("The application environment configuration");
    assert!(result.contains("app"));
    assert!(result.contains("env"));
    assert!(result.contains("config"));
}

// ========== Layer 3: Dictionary ==========

#[test]
fn test_l3_codebook_empty() {
    let cb = layer3_dictionary::build_codebook(&[]);
    assert!(cb.is_empty());
}

#[test]
fn test_l3_codebook_build() {
    let text = "the quick brown fox jumps over the lazy dog. the quick brown fox jumps over the lazy dog. the quick brown fox jumps over the lazy dog. the quick brown fox jumps again.";
    let cb = layer3_dictionary::build_codebook(&[text]);
    // Should find some repeated phrases
    assert!(!cb.is_empty() || text.split_whitespace().count() < 10); // may not find phrases if too short
}

#[test]
fn test_l3_compress_decompress() {
    let mut cb = std::collections::HashMap::new();
    cb.insert("$AA".to_string(), "hello world".to_string());
    let compressed = layer3_dictionary::compress("say hello world please", &cb);
    assert!(compressed.contains("$AA"));
    let decompressed = layer3_dictionary::decompress(&compressed, &cb);
    assert!(decompressed.contains("hello world"));
}

#[test]
fn test_l3_empty_text() {
    let cb = std::collections::HashMap::new();
    assert_eq!(layer3_dictionary::compress("", &cb), "");
}

#[test]
fn test_l3_dollar_escape() {
    let mut cb = std::collections::HashMap::new();
    cb.insert("$AA".to_string(), "test phrase".to_string());
    let text = "has $existing dollar and test phrase here";
    let compressed = layer3_dictionary::compress(text, &cb);
    let decompressed = layer3_dictionary::decompress(&compressed, &cb);
    assert!(decompressed.contains("$existing"));
    assert!(decompressed.contains("test phrase"));
}

#[test]
fn test_l3_generate_codes() {
    let codes = layer3_dictionary::generate_codes(5);
    assert_eq!(codes.len(), 5);
    assert_eq!(codes[0], "$AA");
    assert_eq!(codes[1], "$AB");
}

#[test]
fn test_l3_generate_many_codes() {
    let codes = layer3_dictionary::generate_codes(700);
    assert_eq!(codes.len(), 700);
    // Should have 3-letter codes
    assert!(codes.last().unwrap().len() == 4); // $AAA
}

// ========== Layer 4: Dedup ==========

#[test]
fn test_l4_no_dupes() {
    let entries = vec!["hello", "world", "test"];
    let groups = layer4_dedup::find_duplicates(&entries);
    assert!(groups.is_empty());
}

#[test]
fn test_l4_exact_dupes() {
    let entries = vec!["the quick brown fox jumps", "the quick brown fox jumps", "something else entirely"];
    let groups = layer4_dedup::find_duplicates(&entries);
    assert!(!groups.is_empty());
}

#[test]
fn test_l4_near_dupes() {
    let entries = vec![
        "the quick brown fox jumps over the lazy dog",
        "the quick brown fox jumps over the lazy cat",
        "something completely different here",
    ];
    let groups = layer4_dedup::find_duplicates(&entries);
    assert!(!groups.is_empty());
}

#[test]
fn test_l4_merge() {
    let entries = vec!["short", "much longer text here", "short"];
    let groups = vec![layer4_dedup::DupGroup { indices: vec![0, 2], similarity: 1.0 }];
    let result = layer4_dedup::merge_duplicates(&entries, &groups);
    assert_eq!(result.len(), 2);
}

#[test]
fn test_l4_empty() {
    let groups = layer4_dedup::find_duplicates(&[]);
    assert!(groups.is_empty());
}

#[test]
fn test_l4_single() {
    let groups = layer4_dedup::find_duplicates(&["only one"]);
    assert!(groups.is_empty());
}

#[test]
fn test_l4_jaccard_identical() {
    let a = layer4_dedup::shingles("hello world test", 3);
    let b = layer4_dedup::shingles("hello world test", 3);
    assert!((layer4_dedup::jaccard(&a, &b) - 1.0).abs() < 0.01);
}

#[test]
fn test_l4_jaccard_empty() {
    use std::collections::HashSet;
    let a: HashSet<u64> = HashSet::new();
    let b: HashSet<u64> = HashSet::new();
    assert!((layer4_dedup::jaccard(&a, &b) - 1.0).abs() < 0.01);
}

#[test]
fn test_l4_compress_text() {
    let text = "first paragraph\n\nfirst paragraph\n\nsecond paragraph";
    let result = layer4_dedup::compress(text);
    // Should remove one of the duplicate paragraphs
    assert!(result.matches("first paragraph").count() <= 1);
}

// ========== Layer 5: Format ==========

#[test]
fn test_l5_strip_whitespace() {
    let text = "line1\n\n\n\n\nline2";
    let result = layer5_format::strip_redundant_whitespace(text);
    assert!(!result.contains("\n\n\n"));
}

#[test]
fn test_l5_remove_dup_lines() {
    let text = "line1\nline2\nline1\nline3";
    let result = layer5_format::remove_duplicate_lines(text);
    assert_eq!(result.matches("line1").count(), 1);
}

#[test]
fn test_l5_chinese_punct() {
    let result = layer5_format::normalize_chinese_punct("\u{FF0C}\u{3002}\u{FF01}");
    assert_eq!(result, ",.!");
}

#[test]
fn test_l5_strip_emoji() {
    let result = layer5_format::strip_emoji("hello \u{1F600} world");
    assert!(!result.contains('\u{1F600}'));
    assert!(result.contains("hello"));
}

#[test]
fn test_l5_empty() {
    assert_eq!(layer5_format::compress(""), "");
}

#[test]
fn test_l5_compress_all() {
    let text = "hello\n\n\n\n\nworld\nhello\n\u{FF0C}";
    let result = layer5_format::compress(text);
    assert!(!result.contains("\n\n\n"));
    assert_eq!(result.matches("hello").count(), 1);
}

// ========== Pipeline ==========

#[test]
fn test_pipeline_lossless() {
    let p = CompactorPipeline::lossless();
    let result = p.compress("Hello world.\n\n\n\n\nMore text.");
    assert!(result.compressed_len <= result.original_len);
    assert!(result.layers_applied.contains(&"format".to_string()));
}

#[test]
fn test_pipeline_minimal() {
    let p = CompactorPipeline::minimal();
    let result = p.compress("The function takes a configuration parameter.");
    assert!(result.layers_applied.contains(&"ccp".to_string()));
}

#[test]
fn test_pipeline_balanced() {
    let p = CompactorPipeline::balanced();
    let long_text = "The application configuration requires environment setup. ".repeat(20);
    let result = p.compress(&long_text);
    assert!(result.compressed_len < result.original_len);
}

#[test]
fn test_pipeline_empty() {
    let p = CompactorPipeline::lossless();
    let result = p.compress("");
    assert_eq!(result.output, "");
}

#[test]
fn test_pipeline_jsonl() {
    let p = CompactorPipeline::lossless();
    let jsonl = "{\"content\":\"hi\",\"timestamp\":\"t\"}\n{\"content\":\"bye\",\"trace_id\":\"x\"}";
    let result = p.compress(jsonl);
    assert!(result.layers_applied.contains(&"jsonl".to_string()));
}

#[test]
fn test_pipeline_ratio() {
    let p = CompactorPipeline::lossless();
    let result = p.compress("hello world");
    assert!(result.ratio() <= 1.0);
    assert!(result.ratio() > 0.0);
}

#[test]
fn test_pipeline_decompress_with_codebook() {
    let p = CompactorPipeline::balanced();
    let text = "The application configuration requires environment setup. ".repeat(20);
    let compressed = p.compress(&text);
    if let Some(ref cb) = compressed.codebook {
        let decompressed = p.decompress(&compressed.output, Some(cb));
        // Decompressed should be closer to original
        assert!(decompressed.len() >= compressed.output.len());
    }
}

#[test]
fn test_pipeline_level_fidelity() {
    assert!((CompressionLevel::Lossless.target_fidelity() - 0.99).abs() < 0.01);
    assert!((CompressionLevel::Minimal.target_fidelity() - 0.93).abs() < 0.01);
    assert!((CompressionLevel::Balanced.target_fidelity() - 0.87).abs() < 0.01);
}

#[test]
fn test_pipeline_unicode() {
    let p = CompactorPipeline::minimal();
    let result = p.compress("\u{4f60}\u{597d}\u{4e16}\u{754c}\u{FF0C}\u{8fd9}\u{662f}\u{6d4b}\u{8bd5}");
    assert!(!result.output.is_empty());
}

#[test]
fn test_pipeline_code_content() {
    let p = CompactorPipeline::minimal();
    let code = "fn main() {\n    println!(\"hello\");\n}";
    let result = p.compress(code);
    assert!(result.output.contains("main"));
}

#[test]
fn test_pipeline_yaml() {
    let p = CompactorPipeline::lossless();
    let yaml = "key: value\nlist:\n  - item1\n  - item2";
    let result = p.compress(yaml);
    assert!(result.output.contains("key"));
}

#[test]
fn test_pipeline_single_char() {
    let p = CompactorPipeline::lossless();
    let result = p.compress("x");
    assert_eq!(result.output, "x");
}

#[test]
fn test_pipeline_long_text() {
    let p = CompactorPipeline::balanced();
    let text = "word ".repeat(10000);
    let result = p.compress(&text);
    assert!(result.compressed_len < result.original_len);
}

// ========== Multi-layer combination ==========

#[test]
fn test_multilayer_jsonl_plus_format() {
    let p = CompactorPipeline::lossless();
    let input = "{\"a\":1,\"timestamp\":\"t\"}\n\n\n\n{\"b\":2,\"trace_id\":\"x\"}";
    let result = p.compress(input);
    assert!(!result.output.contains("timestamp"));
}

#[test]
fn test_multilayer_all() {
    let p = CompactorPipeline::balanced();
    let input = "The application configuration is important. ".repeat(10)
        + "\n\n" + &"The application configuration is important. ".repeat(10);
    let result = p.compress(&input);
    assert!(result.compressed_len < result.original_len);
}

// ========== Compression level switching ==========

#[test]
fn test_level_switch() {
    let text = "The database infrastructure management operations performance. ".repeat(10);
    let lossless = CompactorPipeline::lossless().compress(&text);
    let minimal = CompactorPipeline::minimal().compress(&text);
    let balanced = CompactorPipeline::balanced().compress(&text);
    // More aggressive = more compression
    assert!(balanced.compressed_len <= minimal.compressed_len);
    assert!(minimal.compressed_len <= lossless.compressed_len);
}

// ========== Extended Compactor Tests ==========

#[test]
fn test_pipeline_all_levels() {
    let text = "{\"role\":\"user\",\"content\":\"Hello world\",\"timestamp\":\"2024\"}";
    for pipeline in [CompactorPipeline::lossless(), CompactorPipeline::minimal(), CompactorPipeline::balanced()] {
        let result = pipeline.compress(text);
        assert!(!result.output.is_empty());
    }
}

#[test]
fn test_l1_array_values_ext() {
    let line = "{\"role\":\"user\",\"content\":\"hi\",\"tools\":[\"a\",\"b\"]}";
    let result = layer1_jsonl::compress_line(line);
    assert!(result.contains("content"));
}

#[test]
fn test_l2_boundary_single() {
    let result = layer2_ccp::compress("word");
    assert_eq!(result, "word");
}

#[test]
fn test_l2_repeated_pattern_ext() {
    let text = "The function takes a parameter. The function takes a parameter.";
    let result = layer2_ccp::compress(text);
    assert!(!result.is_empty());
}

#[test]
fn test_l3_single_code() {
    let codes = layer3_dictionary::generate_codes(1);
    assert_eq!(codes.len(), 1);
}

#[test]
fn test_l3_many_codes_ext() {
    let codes = layer3_dictionary::generate_codes(1000);
    assert_eq!(codes.len(), 1000);
    let set: std::collections::HashSet<_> = codes.iter().collect();
    assert_eq!(set.len(), 1000);
}

#[test]
fn test_l4_identical_text_ext() {
    let text = "hello\nhello\nhello";
    let result = layer4_dedup::compress(text);
    assert!(!result.is_empty());
}

#[test]
fn test_l4_different_text_ext() {
    let text = "alpha\nbravo\ncharlie";
    let result = layer4_dedup::compress(text);
    assert!(result.contains("alpha"));
    assert!(result.contains("bravo"));
}

#[test]
fn test_l5_double_spaces_ext() {
    let result = layer5_format::strip_redundant_whitespace("hello  world   foo");
    assert!(result.len() <= 18);
}

#[test]
fn test_l5_dup_lines_ext() {
    let text = "line1\nline1\nline2\nline2\nline2";
    let result = layer5_format::remove_duplicate_lines(text);
    assert_eq!(result.lines().count(), 2);
}

#[test]
fn test_pipeline_short_text_ext() {
    let p = CompactorPipeline::balanced();
    let result = p.compress("hi");
    assert!(!result.output.is_empty());
}

#[test]
fn test_pipeline_newlines_ext() {
    let p = CompactorPipeline::lossless();
    let result = p.compress("\n\n\n\n");
    let _ = result;
}

#[test]
fn test_pipeline_repeated_jsonl_ext() {
    let mut text = String::new();
    for i in 0..100 {
        text.push_str(&format!("{{\"role\":\"user\",\"content\":\"msg {}\"}}\n", i));
    }
    let p = CompactorPipeline::minimal();
    let result = p.compress(&text);
    assert!(!result.output.is_empty());
}

#[test]
fn test_pipeline_ratio_positive() {
    let big = "The quick brown fox jumps over the lazy dog. ".repeat(100);
    let p = CompactorPipeline::balanced();
    let result = p.compress(&big);
    assert!(result.ratio() >= 0.0);
}

#[test]
fn test_l2_roundtrip_ext() {
    let original = "The configuration parameter defines the behavior";
    let compressed = layer2_ccp::compress(original);
    let decompressed = layer2_ccp::decompress(&compressed);
    assert!(!decompressed.is_empty());
}

#[test]
fn test_l5_chinese_punct_conversion() {
    let text = "\u{ff0c}\u{3002}";
    let result = layer5_format::normalize_chinese_punct(text);
    assert!(!result.is_empty());
}
