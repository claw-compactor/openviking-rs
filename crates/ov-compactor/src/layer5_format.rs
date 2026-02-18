//! Layer 5: Format cleanup — whitespace, markdown, emoji optimization.

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

static RE_MULTI_NEWLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());
static RE_EMOJI: LazyLock<Regex> = LazyLock::new(|| Regex::new(
    "[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\
     \u{1F1E0}-\u{1F1FF}\u{2702}-\u{27B0}\u{1F900}-\u{1F9FF}\
     \u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}]+"
).unwrap());
static RE_MULTI_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"  +").unwrap());

/// Chinese punctuation → ASCII mapping.
fn zh_punct_map() -> Vec<(&'static str, &'static str)> {
    vec![
        ("\u{FF0C}", ","), ("\u{3002}", "."), ("\u{FF1B}", ";"),
        ("\u{FF1A}", ":"), ("\u{FF01}", "!"), ("\u{FF1F}", "?"),
        ("\u{201C}", "\""), ("\u{201D}", "\""), ("\u{2018}", "'"), ("\u{2019}", "'"),
        ("\u{FF08}", "("), ("\u{FF09}", ")"), ("\u{3010}", "["), ("\u{3011}", "]"),
        ("\u{3001}", ","), ("\u{2026}", "..."), ("\u{FF5E}", "~"),
    ]
}

/// Normalize Chinese punctuation to ASCII.
pub fn normalize_chinese_punct(text: &str) -> String {
    let mut result = text.replace("\u{2014}\u{2014}", "--");
    for (zh, en) in zh_punct_map() {
        result = result.replace(zh, en);
    }
    result
}

/// Strip excessive blank lines (3+ → 2).
pub fn strip_redundant_whitespace(text: &str) -> String {
    let result = RE_MULTI_NEWLINE.replace_all(text, "\n\n");
    result.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Remove exact duplicate non-blank lines.
pub fn remove_duplicate_lines(text: &str) -> String {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for line in text.lines() {
        let stripped = line.trim();
        if stripped.is_empty() {
            result.push(line.to_string());
            continue;
        }
        if seen.contains(stripped) { continue; }
        seen.insert(stripped.to_string());
        result.push(line.to_string());
    }
    result.join("\n")
}

/// Strip emoji characters.
pub fn strip_emoji(text: &str) -> String {
    let result = RE_EMOJI.replace_all(text, "");
    RE_MULTI_SPACE.replace_all(&result, " ").to_string()
}

/// Apply all format cleanup passes.
pub fn compress(text: &str) -> String {
    if text.is_empty() { return String::new(); }
    let mut result = normalize_chinese_punct(text);
    result = strip_redundant_whitespace(&result);
    result = remove_duplicate_lines(&result);
    result = strip_emoji(&result);
    result = strip_redundant_whitespace(&result);
    result
}
