//! Session manager — in-memory session store with concurrent access.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::session::{Session, SessionState};

/// Thread-safe session manager.
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session.
    pub fn create(&self, user_id: impl Into<String>) -> Session {
        let session = Session::new(user_id);
        self.sessions.write().unwrap().insert(session.id.clone(), session.clone());
        session
    }

    /// Create with specific ID.
    pub fn create_with_id(&self, id: impl Into<String>, user_id: impl Into<String>) -> Session {
        let session = Session::with_id(id, user_id);
        self.sessions.write().unwrap().insert(session.id.clone(), session.clone());
        session
    }

    /// Get a session by ID.
    pub fn get(&self, id: &str) -> Option<Session> {
        self.sessions.read().unwrap().get(id).cloned()
    }

    /// Update a session.
    pub fn update(&self, session: &Session) {
        self.sessions.write().unwrap().insert(session.id.clone(), session.clone());
    }

    /// Close a session.
    pub fn close(&self, id: &str) -> bool {
        if let Some(session) = self.sessions.write().unwrap().get_mut(id) {
            session.close();
            true
        } else {
            false
        }
    }

    /// List all active sessions.
    pub fn list_active(&self) -> Vec<Session> {
        self.sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.state == SessionState::Active)
            .cloned()
            .collect()
    }

    /// List sessions for a user.
    pub fn list_by_user(&self, user_id: &str) -> Vec<Session> {
        self.sessions
            .read()
            .unwrap()
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    }

    /// Remove a closed session.
    pub fn remove(&self, id: &str) -> Option<Session> {
        self.sessions.write().unwrap().remove(id)
    }

    /// Count total sessions.
    pub fn count(&self) -> usize {
        self.sessions.read().unwrap().len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
