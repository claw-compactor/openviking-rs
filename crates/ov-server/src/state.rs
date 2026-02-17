use ov_session::manager::SessionManager;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
}
