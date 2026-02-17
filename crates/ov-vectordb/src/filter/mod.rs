//! Filter operations for scalar field filtering.
//!
//! Supports: must, must_not, range, range_out, prefix, contains, regex, and/or logic.

use serde_json::Value;
use std::collections::HashMap;

/// A filter condition tree.
#[derive(Debug, Clone)]
pub enum Filter {
    Must { field: String, values: Vec<Value> },
    MustNot { field: String, values: Vec<Value> },
    Range { field: String, gt: Option<Value>, gte: Option<Value>, lt: Option<Value>, lte: Option<Value> },
    RangeOut { field: String, gte: Option<Value>, lte: Option<Value> },
    Prefix { field: String, prefix: String },
    Contains { field: String, substring: String },
    Regex { field: String, pattern: String },
    And(Vec<Filter>),
    Or(Vec<Filter>),
}

impl Filter {
    /// Parse a filter from a JSON value (matching the Python DSL).
    pub fn from_json(v: &Value) -> Option<Self> {
        let obj = v.as_object()?;
        let op = obj.get("op")?.as_str()?;
        match op {
            "must" => {
                let field = obj.get("field")?.as_str()?.to_string();
                let conds = obj.get("conds")?.as_array()?.clone();
                Some(Filter::Must { field, values: conds })
            }
            "must_not" => {
                let field = obj.get("field")?.as_str()?.to_string();
                let conds = obj.get("conds")?.as_array()?.clone();
                Some(Filter::MustNot { field, values: conds })
            }
            "range" => {
                let field = obj.get("field")?.as_str()?.to_string();
                Some(Filter::Range {
                    field,
                    gt: obj.get("gt").cloned(),
                    gte: obj.get("gte").cloned(),
                    lt: obj.get("lt").cloned(),
                    lte: obj.get("lte").cloned(),
                })
            }
            "range_out" => {
                let field = obj.get("field")?.as_str()?.to_string();
                Some(Filter::RangeOut {
                    field,
                    gte: obj.get("gte").cloned(),
                    lte: obj.get("lte").cloned(),
                })
            }
            "prefix" => {
                let field = obj.get("field")?.as_str()?.to_string();
                let prefix = obj.get("prefix")?.as_str()?.to_string();
                Some(Filter::Prefix { field, prefix })
            }
            "contains" => {
                let field = obj.get("field")?.as_str()?.to_string();
                let substring = obj.get("substring")?.as_str()?.to_string();
                Some(Filter::Contains { field, substring })
            }
            "regex" => {
                let field = obj.get("field")?.as_str()?.to_string();
                let pattern = obj.get("pattern")?.as_str()?.to_string();
                Some(Filter::Regex { field, pattern })
            }
            "and" => {
                let conds = obj.get("conds")?.as_array()?;
                let filters: Vec<Filter> = conds.iter().filter_map(Filter::from_json).collect();
                Some(Filter::And(filters))
            }
            "or" => {
                let conds = obj.get("conds")?.as_array()?;
                let filters: Vec<Filter> = conds.iter().filter_map(Filter::from_json).collect();
                Some(Filter::Or(filters))
            }
            _ => None,
        }
    }

    /// Evaluate the filter against a set of field values.
    pub fn matches(&self, fields: &HashMap<String, Value>) -> bool {
        match self {
            Filter::Must { field, values } => {
                if let Some(field_val) = fields.get(field) {
                    // If field_val is an array (list field), check intersection
                    if let Some(arr) = field_val.as_array() {
                        return values.iter().any(|v| arr.contains(v));
                    }
                    values.iter().any(|v| values_match(field_val, v))
                } else {
                    false
                }
            }
            Filter::MustNot { field, values } => {
                if let Some(field_val) = fields.get(field) {
                    if let Some(arr) = field_val.as_array() {
                        return !values.iter().any(|v| arr.contains(v));
                    }
                    !values.iter().any(|v| values_match(field_val, v))
                } else {
                    true
                }
            }
            Filter::Range { field, gt, gte, lt, lte } => {
                if let Some(field_val) = fields.get(field) {
                    range_check(field_val, gt.as_ref(), gte.as_ref(), lt.as_ref(), lte.as_ref())
                } else {
                    false
                }
            }
            Filter::RangeOut { field, gte, lte } => {
                // NOT in [gte, lte] means < gte OR > lte
                if let Some(field_val) = fields.get(field) {
                    let below = if let Some(g) = gte {
                        compare_values(field_val, g) == Some(std::cmp::Ordering::Less)
                    } else {
                        false
                    };
                    let above = if let Some(l) = lte {
                        compare_values(field_val, l) == Some(std::cmp::Ordering::Greater)
                    } else {
                        false
                    };
                    below || above
                } else {
                    false
                }
            }
            Filter::Prefix { field, prefix } => {
                if let Some(field_val) = fields.get(field) {
                    if let Some(s) = field_val.as_str() {
                        s.starts_with(prefix.as_str())
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Filter::Contains { field, substring } => {
                if let Some(field_val) = fields.get(field) {
                    if let Some(s) = field_val.as_str() {
                        s.contains(substring.as_str())
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Filter::Regex { field, pattern } => {
                if let Some(field_val) = fields.get(field) {
                    if let Some(s) = field_val.as_str() {
                        // Simple regex matching - we avoid pulling in the regex crate
                        // by doing basic pattern matching for common cases
                        simple_regex_match(s, pattern)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Filter::And(filters) => filters.iter().all(|f| f.matches(fields)),
            Filter::Or(filters) => filters.iter().any(|f| f.matches(fields)),
        }
    }
}

fn values_match(a: &Value, b: &Value) -> bool {
    // Numeric comparison: treat i64 and f64 as comparable
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            if let (Some(ia), Some(ib)) = (na.as_i64(), nb.as_i64()) {
                return ia == ib;
            }
            if let (Some(fa), Some(fb)) = (na.as_f64(), nb.as_f64()) {
                return (fa - fb).abs() < 1e-9;
            }
            false
        }
        _ => a == b,
    }
}

fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            let fa = na.as_f64()?;
            let fb = nb.as_f64()?;
            fa.partial_cmp(&fb)
        }
        (Value::String(sa), Value::String(sb)) => Some(sa.cmp(sb)),
        _ => None,
    }
}

fn range_check(val: &Value, gt: Option<&Value>, gte: Option<&Value>, lt: Option<&Value>, lte: Option<&Value>) -> bool {
    if let Some(g) = gt {
        if compare_values(val, g) != Some(std::cmp::Ordering::Greater) {
            return false;
        }
    }
    if let Some(g) = gte {
        match compare_values(val, g) {
            Some(std::cmp::Ordering::Less) => return false,
            None => return false,
            _ => {}
        }
    }
    if let Some(l) = lt {
        if compare_values(val, l) != Some(std::cmp::Ordering::Less) {
            return false;
        }
    }
    if let Some(l) = lte {
        match compare_values(val, l) {
            Some(std::cmp::Ordering::Greater) => return false,
            None => return false,
            _ => {}
        }
    }
    true
}

/// Simple regex-like matching for common patterns.
/// Supports: ^prefix, suffix$, .*substring.*, literal
fn simple_regex_match(s: &str, pattern: &str) -> bool {
    // Handle ^...$ anchored patterns
    if pattern.starts_with('^') && pattern.ends_with('$') {
        let inner = &pattern[1..pattern.len()-1];
        // Handle alternation (a|b)
        if inner.contains('|') {
            return inner.split('|').any(|alt| {
                let alt = alt.trim_matches('(').trim_matches(')');
                s == alt
            });
        }
        return s == inner;
    }
    // Handle ^prefix
    if pattern.starts_with('^') {
        let prefix = &pattern[1..];
        // Handle alternation like ^(a|d)
        if prefix.starts_with('(') && prefix.ends_with(')') {
            let inner = &prefix[1..prefix.len()-1];
            return inner.split('|').any(|alt| s.starts_with(alt));
        }
        return s.starts_with(prefix);
    }
    // Handle suffix$
    if pattern.ends_with('$') {
        let suffix = &pattern[..pattern.len()-1];
        return s.ends_with(suffix);
    }
    // Fallback: contains
    s.contains(pattern)
}
