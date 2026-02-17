//! Layer 1: JSONL compression — strip redundant metadata fields.

use serde_json::Value;

/// Fields to strip from JSONL entries.
const STRIP_FIELDS: &[&str] = &[
    "timestamp", "trace_id", "span_id", "request_id",
    "client_version", "sdk_version", "log_level",
];

/// Compress a single JSONL line by stripping redundant fields.
pub fn compress_line(line: &str) -> String {
    let line = line.trim();
    if line.is_empty() {
        return String::new();
    }

    match serde_json::from_str::<Value>(line) {
        Ok(Value::Object(mut map)) => {
            for field in STRIP_FIELDS {
                map.remove(*field);
            }
            // Strip empty/null fields
            map.retain(|_, v| !v.is_null() && *v != Value::String(String::new()));
            serde_json::to_string(&Value::Object(map)).unwrap_or_else(|_| line.to_string())
        }
        _ => line.to_string(),
    }
}

/// Compress multiple JSONL lines.
pub fn compress(content: &str) -> String {
    content
        .lines()
        .map(|line| compress_line(line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Decompress — JSONL stripping is lossy for removed fields,
/// but the core content is preserved.
pub fn is_reversible() -> bool {
    false // Metadata stripping is lossy by design
}
