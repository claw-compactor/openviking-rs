//! SKILL.md loader and parser.
//!
//! Port of `openviking/core/skill_loader.py`.

use crate::error::{OvError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Parsed skill definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    /// Skill name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Markdown body content.
    pub content: String,
    /// File the skill was loaded from.
    pub source_path: String,
    /// Allowed tool names.
    pub allowed_tools: Vec<String>,
    /// Tags for categorization.
    pub tags: Vec<String>,
}

/// Load and parse SKILL.md files.
pub struct SkillLoader;

impl SkillLoader {
    /// Load a skill from a file path.
    pub fn load(path: impl AsRef<Path>) -> Result<Skill> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            OvError::Storage(format!("Skill file not found: {}: {e}", path.display()))
        })?;
        Self::parse(&content, &path.display().to_string())
    }

    /// Parse SKILL.md content string.
    pub fn parse(content: &str, source_path: &str) -> Result<Skill> {
        let (frontmatter, body) = Self::split_frontmatter(content);

        let fm = frontmatter.ok_or_else(|| {
            OvError::Storage("SKILL.md must have YAML frontmatter".into())
        })?;

        let meta: HashMap<String, serde_json::Value> =
            serde_json::from_str(&Self::yaml_to_json(&fm)?).map_err(|e| {
                OvError::Storage(format!("Invalid YAML frontmatter: {e}"))
            })?;

        let name = meta
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OvError::Storage("Skill must have 'name' field".into()))?
            .to_string();

        let description = meta
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OvError::Storage("Skill must have 'description' field".into()))?
            .to_string();

        let allowed_tools = meta
            .get("allowed-tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let tags = meta
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Skill {
            name,
            description,
            content: body.trim().to_string(),
            source_path: source_path.to_string(),
            allowed_tools,
            tags,
        })
    }

    /// Convert a skill back to SKILL.md format string.
    pub fn to_skill_md(skill: &Skill) -> String {
        format!(
            "---\nname: {}\ndescription: {}\n---\n\n{}",
            skill.name, skill.description, skill.content
        )
    }

    /// Split frontmatter (between `---`) from body.
    fn split_frontmatter(content: &str) -> (Option<String>, String) {
        if !content.starts_with("---") {
            return (None, content.to_string());
        }
        // Find second ---
        let rest = &content[3..];
        if let Some(end) = rest.find("\n---") {
            let fm = rest[..end].trim().to_string();
            let body = rest[end + 4..].to_string();
            (Some(fm), body)
        } else {
            (None, content.to_string())
        }
    }

    /// Minimal YAML key: value to JSON converter (handles simple flat YAML).
    fn yaml_to_json(yaml: &str) -> Result<String> {
        let mut map = serde_json::Map::new();
        let mut current_key: Option<String> = None;
        let mut current_list: Option<Vec<String>> = None;

        for line in yaml.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // List item
            if trimmed.starts_with("- ") {
                if let Some(ref mut list) = current_list {
                    list.push(trimmed.strip_prefix("- ").unwrap().trim().to_string());
                }
                continue;
            }

            // Flush previous list
            if let (Some(key), Some(list)) = (current_key.take(), current_list.take()) {
                let arr: Vec<serde_json::Value> = list.into_iter().map(|s| serde_json::json!(s)).collect();
                map.insert(key, serde_json::Value::Array(arr));
            }

            // key: value
            if let Some(colon) = trimmed.find(':') {
                let key = trimmed[..colon].trim().to_string();
                let val = trimmed[colon + 1..].trim();
                if val.is_empty() {
                    // Could be a list follows
                    current_key = Some(key);
                    current_list = Some(Vec::new());
                } else {
                    map.insert(key, serde_json::Value::String(val.to_string()));
                }
            }
        }

        // Flush final list
        if let (Some(key), Some(list)) = (current_key, current_list) {
            let arr: Vec<serde_json::Value> = list.into_iter().map(|s| serde_json::json!(s)).collect();
            map.insert(key, serde_json::Value::Array(arr));
        }

        Ok(serde_json::to_string(&map).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample_skill_md() -> &'static str {
        "---\nname: web-search\ndescription: Search the web\ntags:\n- search\n- web\nallowed-tools:\n- brave_search\n---\n\n# Web Search\n\nSearch the web for information."
    }

    #[test]
    fn test_parse_basic() {
        let skill = SkillLoader::parse(sample_skill_md(), "test.md").unwrap();
        assert_eq!(skill.name, "web-search");
        assert_eq!(skill.description, "Search the web");
        assert!(skill.content.contains("# Web Search"));
        assert_eq!(skill.tags, vec!["search", "web"]);
        assert_eq!(skill.allowed_tools, vec!["brave_search"]);
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let result = SkillLoader::parse("# Just markdown", "test.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_name() {
        let content = "---\ndescription: test\n---\n\nbody";
        let result = SkillLoader::parse(content, "test.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_description() {
        let content = "---\nname: test\n---\n\nbody";
        let result = SkillLoader::parse(content, "test.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_skill_md() {
        let skill = Skill {
            name: "test".into(),
            description: "A test skill".into(),
            content: "# Test\n\nBody.".into(),
            source_path: String::new(),
            allowed_tools: vec![],
            tags: vec![],
        };
        let md = SkillLoader::to_skill_md(&skill);
        assert!(md.starts_with("---"));
        assert!(md.contains("name: test"));
        assert!(md.contains("# Test"));
    }

    #[test]
    fn test_load_from_file() {
        let dir = std::env::temp_dir().join("ov_skill_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_skill.md");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "{}", sample_skill_md()).unwrap();
        let skill = SkillLoader::load(&path).unwrap();
        assert_eq!(skill.name, "web-search");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_nonexistent() {
        let result = SkillLoader::load("/nonexistent/path/skill.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_tags() {
        let content = "---\nname: t\ndescription: d\n---\n\nbody";
        let skill = SkillLoader::parse(content, "").unwrap();
        assert!(skill.tags.is_empty());
        assert!(skill.allowed_tools.is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let skill = SkillLoader::parse(sample_skill_md(), "orig").unwrap();
        let md = SkillLoader::to_skill_md(&skill);
        let skill2 = SkillLoader::parse(&md, "round").unwrap();
        assert_eq!(skill.name, skill2.name);
        assert_eq!(skill.description, skill2.description);
    }

    #[test]
    fn test_skill_serde() {
        let skill = SkillLoader::parse(sample_skill_md(), "test").unwrap();
        let json = serde_json::to_string(&skill).unwrap();
        let skill2: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(skill, skill2);
    }


    #[test]
    fn test_skill_parse_empty_content() {
        let result = SkillLoader::parse("", "test.md");
        // Should handle empty input gracefully
        let _ = result;
    }

    #[test]
    fn test_skill_parse_no_frontmatter() {
        let result = SkillLoader::parse("Just some text", "test.md");
        let _ = result;
    }

    #[test]
    fn test_skill_parse_unicode() {
        let md = "---
name: test
description: unicode test
---
# Chinese content

some body";
        let result = SkillLoader::parse(md, "test.md");
        let _ = result;
    }

    #[test]
    fn test_skill_load_nonexistent() {
        let result = SkillLoader::load("/nonexistent/skill.md");
        assert!(result.is_err());
    }

}
