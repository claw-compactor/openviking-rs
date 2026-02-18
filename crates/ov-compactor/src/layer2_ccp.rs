//! Layer 2: CCP (Context Compression Protocol) — abbreviate technical terms.

use std::collections::HashMap;
use std::sync::LazyLock;
use regex::Regex;

/// Build the default CCP abbreviation map.
pub fn default_abbreviations() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Technical terms
    m.insert("function", "fn");
    m.insert("variable", "var");
    m.insert("constant", "const");
    m.insert("parameter", "param");
    m.insert("argument", "arg");
    m.insert("configuration", "config");
    m.insert("environment", "env");
    m.insert("application", "app");
    m.insert("database", "db");
    m.insert("repository", "repo");
    m.insert("directory", "dir");
    m.insert("document", "doc");
    m.insert("information", "info");
    m.insert("authentication", "auth");
    m.insert("authorization", "authz");
    m.insert("implementation", "impl");
    m.insert("specification", "spec");
    m.insert("development", "dev");
    m.insert("production", "prod");
    m.insert("dependency", "dep");
    m.insert("dependencies", "deps");
    m.insert("kubernetes", "k8s");
    m.insert("container", "ctr");
    m.insert("microservice", "μsvc");
    m.insert("infrastructure", "infra");
    m.insert("management", "mgmt");
    m.insert("operation", "op");
    m.insert("operations", "ops");
    m.insert("organization", "org");
    m.insert("technology", "tech");
    m.insert("communication", "comm");
    m.insert("performance", "perf");
    m.insert("distribution", "dist");
    m.insert("distributed", "dist'd");
    m.insert("approximately", "~");
    m.insert("reference", "ref");
    m.insert("message", "msg");
    m.insert("command", "cmd");
    m.insert("request", "req");
    m.insert("response", "resp");
    m.insert("memory", "mem");
    m.insert("maximum", "max");
    m.insert("minimum", "min");
    m.insert("number", "num");
    m.insert("string", "str");
    m.insert("integer", "int");
    m.insert("boolean", "bool");
    m
}

struct CachedRegexMap {
    /// (regex, replacement) sorted by key length descending
    compress_pairs: Vec<(Regex, &'static str)>,
    /// (regex, replacement) sorted by value length descending
    decompress_pairs: Vec<(Regex, &'static str)>,
}

static DEFAULT_REGEX_MAP: LazyLock<CachedRegexMap> = LazyLock::new(|| {
    let abbrevs = default_abbreviations();
    let mut sorted: Vec<_> = abbrevs.iter().map(|(k, v)| (*k, *v)).collect::<Vec<(&str, &str)>>();
    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let compress_pairs: Vec<_> = sorted.iter().map(|(long, short)| {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(long));
        (Regex::new(&pattern).unwrap(), *short)
    }).collect();

    let mut reverse: Vec<_> = abbrevs.iter().map(|(k, v)| (*v, *k)).collect();
    reverse.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let decompress_pairs: Vec<_> = reverse.iter().map(|(short, long)| {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(short));
        (Regex::new(&pattern).unwrap(), *long)
    }).collect();

    CachedRegexMap { compress_pairs, decompress_pairs }
});

/// Apply CCP abbreviations to text.
pub fn compress(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut result = text.to_string();
    for (re, short) in &DEFAULT_REGEX_MAP.compress_pairs {
        result = re.replace_all(&result, *short).to_string();
    }
    result
}

/// Apply custom abbreviation map.
pub fn compress_with_map(text: &str, abbrevs: &HashMap<&str, &str>) -> String {
    if text.is_empty() || abbrevs.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();
    // Sort by length descending to avoid partial matches
    let mut sorted: Vec<_> = abbrevs.iter().map(|(k, v)| (*k, *v)).collect::<Vec<(&str, &str)>>();
    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (long, short) in sorted {
        // Case-insensitive word boundary replacement
        let pattern = format!(r"(?i)\b{}\b", regex::escape(long));
        if let Ok(re) = regex::Regex::new(&pattern) {
            result = re.replace_all(&result, short).to_string();
        }
    }
    result
}

/// Expand CCP abbreviations (reverse).
pub fn decompress(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut result = text.to_string();
    for (re, long) in &DEFAULT_REGEX_MAP.decompress_pairs {
        result = re.replace_all(&result, *long).to_string();
    }
    result
}

/// Expand with custom map.
pub fn decompress_with_map(text: &str, abbrevs: &HashMap<&str, &str>) -> String {
    if text.is_empty() || abbrevs.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();
    // Reverse map: short -> long
    let mut reverse: Vec<_> = abbrevs.iter().map(|(k, v)| (*v, *k)).collect();
    reverse.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (short, long) in reverse {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(short));
        if let Ok(re) = regex::Regex::new(&pattern) {
            result = re.replace_all(&result, long).to_string();
        }
    }
    result
}
