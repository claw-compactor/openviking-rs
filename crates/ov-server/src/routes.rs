use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

pub fn context_routes() -> Router {
    Router::new()
        .route("/api/v1/context", get(list_contexts))
        .route("/api/v1/context/search", get(search_contexts))
}

pub fn session_routes() -> Router {
    Router::new()
        .route("/api/v1/session", get(list_sessions))
}

async fn list_contexts() -> Json<Value> {
    Json(json!({ "contexts": [] }))
}

async fn search_contexts() -> Json<Value> {
    Json(json!({ "results": [] }))
}

async fn list_sessions() -> Json<Value> {
    Json(json!({ "sessions": [] }))
}
