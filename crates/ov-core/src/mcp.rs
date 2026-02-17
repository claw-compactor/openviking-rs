//! MCP to Skill converter.
//!
//! Port of `openviking/core/mcp_converter.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A converted skill from MCP format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpSkill {
    /// Skill name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Full SKILL.md content (frontmatter + body).
    pub content: String,
}

/// JSON Schema style input schema.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputSchema {
    /// Property definitions.
    #[serde(default)]
    pub properties: HashMap<String, PropertyInfo>,
    /// Required field names.
    #[serde(default)]
    pub required: Vec<String>,
}

/// Property definition in an input schema.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PropertyInfo {
    /// Type name (string, number, etc.).
    #[serde(default, rename = "type")]
    pub type_name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// MCP tool config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolConfig {
    /// Tool name.
    #[serde(default)]
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Input schema.
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Option<InputSchema>,
}

/// Convert an MCP tool definition to a [`McpSkill`].
pub fn mcp_to_skill(config: &McpToolConfig) -> McpSkill {
    let name = config.name.replace('_', "-");
    let name = if name.is_empty() {
        "unnamed-tool".to_string()
    } else {
        name
    };
    let description = &config.description;

    let mut body = format!("# {name}\n\n");
    if !description.is_empty() {
        body.push_str(description);
        body.push('\n');
    }

    if let Some(schema) = &config.input_schema {
        if !schema.properties.is_empty() {
            body.push_str("\n## Parameters\n\n");
            for (param_name, info) in &schema.properties {
                let typ = if info.type_name.is_empty() {
                    "any"
                } else {
                    &info.type_name
                };
                let req = if schema.required.contains(param_name) {
                    " (required)"
                } else {
                    " (optional)"
                };
                body.push_str(&format!(
                    "- **{param_name}** ({typ}){req}: {}\n",
                    info.description
                ));
            }
        }
    }

    body.push_str(&format!(
        "\n## Usage\n\nThis tool wraps the MCP tool `{name}`. \
         Call this when the user needs functionality matching the description above.\n"
    ));

    let content = format!(
        "---\nname: {name}\ndescription: {description}\n---\n\n{body}"
    );

    McpSkill {
        name,
        description: description.clone(),
        content,
    }
}

/// Check if a JSON value looks like an MCP tool config.
pub fn is_mcp_format(data: &serde_json::Value) -> bool {
    data.is_object() && data.get("inputSchema").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mcp() -> McpToolConfig {
        McpToolConfig {
            name: "web_search".into(),
            description: "Search the web".into(),
            input_schema: Some(InputSchema {
                properties: {
                    let mut m = HashMap::new();
                    m.insert(
                        "query".into(),
                        PropertyInfo {
                            type_name: "string".into(),
                            description: "Search query".into(),
                        },
                    );
                    m
                },
                required: vec!["query".into()],
            }),
        }
    }

    #[test]
    fn test_mcp_to_skill_basic() {
        let skill = mcp_to_skill(&sample_mcp());
        assert_eq!(skill.name, "web-search");
        assert_eq!(skill.description, "Search the web");
        assert!(skill.content.contains("# web-search"));
        assert!(skill.content.contains("## Parameters"));
        assert!(skill.content.contains("(required)"));
    }

    #[test]
    fn test_mcp_to_skill_no_schema() {
        let config = McpToolConfig {
            name: "simple".into(),
            description: "Simple tool".into(),
            input_schema: None,
        };
        let skill = mcp_to_skill(&config);
        assert_eq!(skill.name, "simple");
        assert!(!skill.content.contains("## Parameters"));
    }

    #[test]
    fn test_mcp_to_skill_unnamed() {
        let config = McpToolConfig {
            name: String::new(),
            description: "No name".into(),
            input_schema: None,
        };
        let skill = mcp_to_skill(&config);
        assert_eq!(skill.name, "unnamed-tool");
    }

    #[test]
    fn test_mcp_to_skill_underscore_replace() {
        let config = McpToolConfig {
            name: "my_cool_tool".into(),
            description: "Cool".into(),
            input_schema: None,
        };
        let skill = mcp_to_skill(&config);
        assert_eq!(skill.name, "my-cool-tool");
    }

    #[test]
    fn test_mcp_to_skill_frontmatter() {
        let skill = mcp_to_skill(&sample_mcp());
        assert!(skill.content.starts_with("---"));
        assert!(skill.content.contains("name: web-search"));
    }

    #[test]
    fn test_is_mcp_format_true() {
        let val = serde_json::json!({"name": "t", "inputSchema": {}});
        assert!(is_mcp_format(&val));
    }

    #[test]
    fn test_is_mcp_format_false() {
        let val = serde_json::json!({"name": "t"});
        assert!(!is_mcp_format(&val));
    }

    #[test]
    fn test_is_mcp_format_not_object() {
        assert!(!is_mcp_format(&serde_json::json!("string")));
    }

    #[test]
    fn test_mcp_serde_roundtrip() {
        let config = sample_mcp();
        let json = serde_json::to_string(&config).unwrap();
        let config2: McpToolConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config2.name, "web_search");
    }

    #[test]
    fn test_mcp_to_skill_optional_param() {
        let config = McpToolConfig {
            name: "tool".into(),
            description: "desc".into(),
            input_schema: Some(InputSchema {
                properties: {
                    let mut m = HashMap::new();
                    m.insert(
                        "opt".into(),
                        PropertyInfo {
                            type_name: "number".into(),
                            description: "Optional param".into(),
                        },
                    );
                    m
                },
                required: vec![],
            }),
        };
        let skill = mcp_to_skill(&config);
        assert!(skill.content.contains("(optional)"));
    }
}
