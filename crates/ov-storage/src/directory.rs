//! Preset directory initializer

use ov_core::types::DirectoryDefinition;

pub fn preset_directories() -> Vec<(&'static str, DirectoryDefinition)> {
    vec![
        ("session", DirectoryDefinition {
            path: String::new(),
            abstract_text: "Session scope".into(),
            overview: "Session-level temporary data storage".into(),
            children: vec![],
        }),
        ("user", DirectoryDefinition {
            path: String::new(),
            abstract_text: "User scope".into(),
            overview: "User-level persistent data storage".into(),
            children: vec![
                DirectoryDefinition {
                    path: "memories".into(),
                    abstract_text: "User long-term memory".into(),
                    overview: "Preferences, entities, events".into(),
                    children: vec![],
                },
            ],
        }),
        ("agent", DirectoryDefinition {
            path: String::new(),
            abstract_text: "Agent scope".into(),
            overview: "Agent-level global data".into(),
            children: vec![
                DirectoryDefinition {
                    path: "memories".into(),
                    abstract_text: "Agent learning memories".into(),
                    overview: "Cases and patterns".into(),
                    children: vec![],
                },
                DirectoryDefinition {
                    path: "instructions".into(),
                    abstract_text: "Agent instruction set".into(),
                    overview: "Behavioral directives".into(),
                    children: vec![],
                },
                DirectoryDefinition {
                    path: "skills".into(),
                    abstract_text: "Agent skill registry".into(),
                    overview: "Callable skill definitions".into(),
                    children: vec![],
                },
            ],
        }),
        ("resources", DirectoryDefinition {
            path: String::new(),
            abstract_text: "Resources scope".into(),
            overview: "Globally shared resource storage".into(),
            children: vec![],
        }),
    ]
}
