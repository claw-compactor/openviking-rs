//! Preset directory structure definitions for OpenViking.
//!
//! Port of `openviking/core/directories.py`.

use crate::context::ContextType;
use crate::types::DirectoryDefinition;
use std::collections::HashMap;

/// Build the preset directory tree.
///
/// Returns a map from scope name to root [`DirectoryDefinition`].
pub fn preset_directories() -> HashMap<&'static str, DirectoryDefinition> {
    let mut map = HashMap::new();

    map.insert(
        "session",
        DirectoryDefinition {
            path: String::new(),
            abstract_text: "Session scope. Stores complete context for a single conversation, including original messages and compressed summaries.".into(),
            overview: "Session-level temporary data storage, can be archived or cleaned after session ends.".into(),
            children: vec![],
        },
    );

    map.insert(
        "user",
        DirectoryDefinition {
            path: String::new(),
            abstract_text: "User scope. Stores user's long-term memory, persisted across sessions.".into(),
            overview: "User-level persistent data storage for building user profiles and managing private memories.".into(),
            children: vec![
                DirectoryDefinition {
                    path: "memories".into(),
                    abstract_text: "User's long-term memory storage.".into(),
                    overview: "Use this directory to access user's personalized memories.".into(),
                    children: vec![
                        DirectoryDefinition {
                            path: "preferences".into(),
                            abstract_text: "User's personalized preference memories.".into(),
                            overview: "Access when adjusting output style, following user habits.".into(),
                            children: vec![],
                        },
                        DirectoryDefinition {
                            path: "entities".into(),
                            abstract_text: "Entity memories from user's world.".into(),
                            overview: "Access when referencing user-related projects, people, concepts.".into(),
                            children: vec![],
                        },
                        DirectoryDefinition {
                            path: "events".into(),
                            abstract_text: "User's event records.".into(),
                            overview: "Access when reviewing user history.".into(),
                            children: vec![],
                        },
                    ],
                },
            ],
        },
    );

    map.insert(
        "agent",
        DirectoryDefinition {
            path: String::new(),
            abstract_text: "Agent scope. Stores Agent's learning memories, instructions, and skills.".into(),
            overview: "Agent-level global data storage.".into(),
            children: vec![
                DirectoryDefinition {
                    path: "memories".into(),
                    abstract_text: "Agent's long-term memory storage.".into(),
                    overview: "Use this directory to access Agent's learning memories.".into(),
                    children: vec![
                        DirectoryDefinition {
                            path: "cases".into(),
                            abstract_text: "Agent's case records.".into(),
                            overview: "Access cases when encountering similar problems.".into(),
                            children: vec![],
                        },
                        DirectoryDefinition {
                            path: "patterns".into(),
                            abstract_text: "Agent's effective patterns.".into(),
                            overview: "Access patterns when executing tasks requiring strategy.".into(),
                            children: vec![],
                        },
                    ],
                },
                DirectoryDefinition {
                    path: "instructions".into(),
                    abstract_text: "Agent instruction set.".into(),
                    overview: "Access when Agent needs to follow specific rules.".into(),
                    children: vec![],
                },
                DirectoryDefinition {
                    path: "skills".into(),
                    abstract_text: "Agent's skill registry.".into(),
                    overview: "Access when Agent needs to execute specific tasks.".into(),
                    children: vec![],
                },
            ],
        },
    );

    map.insert(
        "resources",
        DirectoryDefinition {
            path: String::new(),
            abstract_text: "Resources scope. Independent knowledge and resource storage.".into(),
            overview: "Globally shared resource storage, organized by project/topic.".into(),
            children: vec![],
        },
    );

    map.insert(
        "transactions",
        DirectoryDefinition {
            path: String::new(),
            abstract_text: "Transaction scope. Stores transaction records.".into(),
            overview: "Per-account transaction storage.".into(),
            children: vec![],
        },
    );

    map
}

/// Determine [`ContextType`] based on URI.
pub fn get_context_type_for_uri(uri: &str) -> ContextType {
    let prefix = &uri[..uri.len().min(20)];
    if prefix.contains("/memories") {
        ContextType::Memory
    } else if prefix.contains("/resources") {
        ContextType::Resource
    } else if prefix.contains("/skills") {
        ContextType::Skill
    } else if uri.starts_with("viking://session") {
        ContextType::Memory
    } else {
        ContextType::Resource
    }
}

/// Collect all URIs from the preset directory tree in depth-first order.
pub fn collect_all_uris(scope: &str, defn: &DirectoryDefinition) -> Vec<String> {
    let root = format!("viking://{scope}");
    let mut result = vec![root.clone()];
    collect_children_uris(&root, &defn.children, &mut result);
    result
}

fn collect_children_uris(parent: &str, children: &[DirectoryDefinition], out: &mut Vec<String>) {
    for child in children {
        let uri = format!("{parent}/{}", child.path);
        out.push(uri.clone());
        collect_children_uris(&uri, &child.children, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_has_all_scopes() {
        let dirs = preset_directories();
        assert!(dirs.contains_key("session"));
        assert!(dirs.contains_key("user"));
        assert!(dirs.contains_key("agent"));
        assert!(dirs.contains_key("resources"));
        assert!(dirs.contains_key("transactions"));
    }

    #[test]
    fn test_user_has_memories_children() {
        let dirs = preset_directories();
        let user = &dirs["user"];
        assert_eq!(user.children.len(), 1);
        let mem = &user.children[0];
        assert_eq!(mem.path, "memories");
        assert_eq!(mem.children.len(), 3);
    }

    #[test]
    fn test_agent_has_three_top_children() {
        let dirs = preset_directories();
        let agent = &dirs["agent"];
        assert_eq!(agent.children.len(), 3);
        let names: Vec<&str> = agent.children.iter().map(|c| c.path.as_str()).collect();
        assert!(names.contains(&"memories"));
        assert!(names.contains(&"instructions"));
        assert!(names.contains(&"skills"));
    }

    #[test]
    fn test_agent_memories_has_cases_patterns() {
        let dirs = preset_directories();
        let agent_mem = &dirs["agent"].children[0];
        assert_eq!(agent_mem.path, "memories");
        let names: Vec<&str> = agent_mem.children.iter().map(|c| c.path.as_str()).collect();
        assert!(names.contains(&"cases"));
        assert!(names.contains(&"patterns"));
    }

    #[test]
    fn test_get_context_type_memories() {
        assert_eq!(get_context_type_for_uri("viking://memories/x"), ContextType::Memory);
    }

    #[test]
    fn test_get_context_type_skills() {
        assert_eq!(get_context_type_for_uri("viking://skills/s"), ContextType::Skill);
    }

    #[test]
    fn test_get_context_type_resources() {
        assert_eq!(get_context_type_for_uri("viking://resources/d"), ContextType::Resource);
    }

    #[test]
    fn test_get_context_type_session() {
        assert_eq!(get_context_type_for_uri("viking://session/123"), ContextType::Memory);
    }

    #[test]
    fn test_collect_all_uris_agent() {
        let dirs = preset_directories();
        let uris = collect_all_uris("agent", &dirs["agent"]);
        assert!(uris.contains(&"viking://agent".to_string()));
        assert!(uris.contains(&"viking://agent/memories".to_string()));
        assert!(uris.contains(&"viking://agent/memories/cases".to_string()));
        assert!(uris.contains(&"viking://agent/skills".to_string()));
    }

    #[test]
    fn test_collect_all_uris_resources() {
        let dirs = preset_directories();
        let uris = collect_all_uris("resources", &dirs["resources"]);
        assert_eq!(uris, vec!["viking://resources"]);
    }

    #[test]
    fn test_session_no_children() {
        let dirs = preset_directories();
        assert!(dirs["session"].children.is_empty());
    }

    #[test]
    fn test_transactions_no_children() {
        let dirs = preset_directories();
        assert!(dirs["transactions"].children.is_empty());
    }

    #[test]
    fn test_resources_no_children() {
        let dirs = preset_directories();
        assert!(dirs["resources"].children.is_empty());
    }
}
