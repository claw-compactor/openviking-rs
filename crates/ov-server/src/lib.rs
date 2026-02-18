//! OpenViking HTTP API server (Axum).
//!
//! Provides REST endpoints for context/memory CRUD, search, session management,
//! and health/status monitoring.

pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
use state::AppState;

/// Build the application router with all routes.
pub fn app() -> Router {
    let state = AppState::new();
    app_with_state(state)
}

/// Build the application router with a custom state.
pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .merge(routes::health_routes())
        .merge(routes::context_routes())
        .merge(routes::session_routes())
        .with_state(state)
}

#[cfg(test)]
mod tests;
