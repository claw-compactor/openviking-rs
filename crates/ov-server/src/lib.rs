//! HTTP API server (Axum)

pub mod routes;
pub mod state;

use axum::Router;

pub fn app() -> Router {
    Router::new()
        .merge(routes::context_routes())
        .merge(routes::session_routes())
}
