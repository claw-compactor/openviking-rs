use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Message role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A single message part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    #[serde(rename = "type")]
    pub part_type: String,
    pub text: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<String>,
    pub tool_output: Option<String>,
    pub tool_status: Option<String>,
    pub uri: Option<String>,
}

impl Part {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            part_type: "text".into(),
            text: Some(content.into()),
            tool_name: None,
            tool_input: None,
            tool_output: None,
            tool_status: None,
            uri: None,
        }
    }

    pub fn tool(name: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            part_type: "tool".into(),
            text: None,
            tool_name: Some(name.into()),
            tool_input: Some(input.into()),
            tool_output: None,
            tool_status: Some("pending".into()),
            uri: None,
        }
    }
}

/// A conversation message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub parts: Vec<Part>,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn new(role: Role, parts: Vec<Part>) -> Self {
        Self {
            id: format!("msg_{}", Uuid::new_v4().simple()),
            role,
            parts,
            created_at: Utc::now(),
        }
    }

    /// Get concatenated text content.
    pub fn content(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| p.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Find a tool part by name.
    pub fn find_tool_part(&mut self, tool_name: &str) -> Option<&mut Part> {
        self.parts
            .iter_mut()
            .find(|p| p.tool_name.as_deref() == Some(tool_name))
    }

    /// Serialize to JSONL line.
    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSONL line.
    pub fn from_jsonl(line: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(line)?)
    }
}

/// Session compression info.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCompression {
    pub summary: String,
    pub original_count: usize,
    pub compressed_count: usize,
    pub compression_index: usize,
}

/// Session statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_turns: usize,
    pub total_tokens: usize,
    pub compression_count: usize,
    pub contexts_used: usize,
    pub skills_used: usize,
    pub memories_extracted: usize,
}

/// Usage record for context/skill tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub uri: String,
    pub usage_type: String, // "context" | "skill"
    pub contribution: f64,
    pub input: String,
    pub output: String,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

impl Usage {
    pub fn context(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            usage_type: "context".into(),
            contribution: 0.0,
            input: String::new(),
            output: String::new(),
            success: true,
            timestamp: Utc::now(),
        }
    }

    pub fn skill(uri: impl Into<String>, input: impl Into<String>, output: impl Into<String>, success: bool) -> Self {
        Self {
            uri: uri.into(),
            usage_type: "skill".into(),
            contribution: 0.0,
            input: input.into(),
            output: output.into(),
            success,
            timestamp: Utc::now(),
        }
    }
}

/// Session state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Active,
    Committed,
    Closed,
}

/// Core session struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    pub compression: SessionCompression,
    pub stats: SessionStats,
    pub usage_records: Vec<Usage>,
    pub auto_commit_threshold: usize,
    pub metadata: serde_json::Value,
}

impl Session {
    pub fn new(user_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            state: SessionState::Active,
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            compression: SessionCompression::default(),
            stats: SessionStats::default(),
            usage_records: Vec::new(),
            auto_commit_threshold: 8000,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_id(id: impl Into<String>, user_id: impl Into<String>) -> Self {
        let mut s = Self::new(user_id);
        s.id = id.into();
        s
    }

    /// Add a message.
    pub fn add_message(&mut self, role: Role, parts: Vec<Part>) -> &Message {
        let msg = Message::new(role.clone(), parts);
        if role == Role::User {
            self.stats.total_turns += 1;
        }
        self.stats.total_tokens += msg.content().len() / 4;
        self.messages.push(msg);
        self.updated_at = Utc::now();
        self.messages.last().unwrap()
    }

    /// Update a tool part's output.
    pub fn update_tool(&mut self, message_id: &str, tool_name: &str, output: &str, status: &str) -> bool {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            if let Some(part) = msg.find_tool_part(tool_name) {
                part.tool_output = Some(output.to_string());
                part.tool_status = Some(status.to_string());
                self.updated_at = Utc::now();
                return true;
            }
        }
        false
    }

    /// Record context/skill usage.
    pub fn track_usage(&mut self, usage: Usage) {
        match usage.usage_type.as_str() {
            "context" => self.stats.contexts_used += 1,
            "skill" => self.stats.skills_used += 1,
            _ => {}
        }
        self.usage_records.push(usage);
    }

    /// Commit session: archive messages, return them for memory extraction.
    pub fn commit(&mut self) -> Vec<Message> {
        if self.messages.is_empty() {
            return Vec::new();
        }
        self.compression.compression_index += 1;
        self.compression.original_count += self.messages.len();
        self.stats.compression_count = self.compression.compression_index;
        self.state = SessionState::Committed;
        self.updated_at = Utc::now();
        std::mem::take(&mut self.messages)
    }

    /// Close session.
    pub fn close(&mut self) {
        self.state = SessionState::Closed;
        self.updated_at = Utc::now();
    }

    /// Check if session needs compression.
    pub fn needs_compression(&self) -> bool {
        self.stats.total_tokens >= self.auto_commit_threshold
    }

    /// Get total message count.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get session URI.
    pub fn uri(&self) -> String {
        format!("viking://session/{}", self.id)
    }

    /// Generate archive summary.
    pub fn generate_summary(&self) -> String {
        let turn_count = self.messages.iter().filter(|m| m.role == Role::User).count();
        format!("# Session Summary\n\n**Overview**: {} turns, {} messages", turn_count, self.messages.len())
    }

    /// Serialize all messages to JSONL.
    pub fn messages_to_jsonl(&self) -> String {
        self.messages
            .iter()
            .map(|m| m.to_jsonl())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Load messages from JSONL.
    pub fn load_messages_from_jsonl(&mut self, content: &str) -> anyhow::Result<usize> {
        let mut count = 0;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            let msg = Message::from_jsonl(line)?;
            if msg.role == Role::User {
                self.stats.total_turns += 1;
            }
            self.stats.total_tokens += msg.content().len() / 4;
            self.messages.push(msg);
            count += 1;
        }
        Ok(count)
    }
}

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Session(user={}, id={}, state={:?})", self.user_id, self.id, self.state)
    }
}
