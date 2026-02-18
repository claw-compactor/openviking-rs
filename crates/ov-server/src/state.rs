//! Application state shared across all handlers.

use ov_core::context::Context;
use ov_session::manager::SessionManager;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory context store keyed by URI.
#[derive(Debug, Clone, Default)]
pub struct ContextStore {
    inner: Arc<RwLock<HashMap<String, Context>>>,
}

impl ContextStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, ctx: Context) {
        self.inner.write().unwrap().insert(ctx.uri.clone(), ctx);
    }

    pub fn get(&self, uri: &str) -> Option<Context> {
        self.inner.read().unwrap().get(uri).cloned()
    }

    pub fn remove(&self, uri: &str) -> Option<Context> {
        self.inner.write().unwrap().remove(uri)
    }

    pub fn list(&self) -> Vec<Context> {
        self.inner.read().unwrap().values().cloned().collect()
    }

    pub fn search(&self, query: &str) -> Vec<Context> {
        let q = query.to_lowercase();
        self.inner.read().unwrap().values()
            .filter(|c| {
                c.uri.to_lowercase().contains(&q)
                    || c.abstract_text.to_lowercase().contains(&q)
                    || c.category.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    pub fn list_by_type(&self, context_type: &str) -> Vec<Context> {
        self.inner.read().unwrap().values()
            .filter(|c| c.context_type.as_str() == context_type)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    pub fn update(&self, uri: &str, f: impl FnOnce(&mut Context)) -> Option<Context> {
        let mut map = self.inner.write().unwrap();
        if let Some(ctx) = map.get_mut(uri) {
            f(ctx);
            Some(ctx.clone())
        } else {
            None
        }
    }
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub context_store: ContextStore,
    pub start_time: std::time::Instant,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
            context_store: ContextStore::new(),
            start_time: std::time::Instant::now(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
