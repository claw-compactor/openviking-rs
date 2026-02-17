use super::Session;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct SessionManager {
    sessions: Mutex<HashMap<String, Session>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { sessions: Mutex::new(HashMap::new()) }
    }

    pub fn create(&self, user_id: Option<String>) -> Session {
        let now = Utc::now();
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            updated_at: now,
            metadata: serde_json::Value::Null,
        };
        self.sessions.lock().unwrap().insert(session.id.clone(), session.clone());
        session
    }

    pub fn get(&self, id: &str) -> Option<Session> {
        self.sessions.lock().unwrap().get(id).cloned()
    }
}
