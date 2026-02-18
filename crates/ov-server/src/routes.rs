//! HTTP route handlers for OpenViking API.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use ov_core::context::{Context, ContextType};
use ov_session::session::{Part, Role};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::ApiError;
use crate::state::AppState;

type Result<T> = std::result::Result<T, ApiError>;

// ==================== Health / Status ====================

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let uptime = state.start_time.elapsed().as_secs();
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime,
        "contexts": state.context_store.count(),
        "sessions": state.session_manager.count(),
    }))
}

// ==================== Context / Memory CRUD ====================

pub fn context_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/contexts", get(list_contexts).post(create_context))
        .route("/api/v1/contexts/search", get(search_contexts))
        .route(
            "/api/v1/contexts/{*uri_path}",
            get(get_context).put(update_context).delete(delete_context),
        )
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(rename = "type")]
    context_type: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

async fn list_contexts(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Json<Value> {
    let mut contexts = if let Some(ref ct) = q.context_type {
        state.context_store.list_by_type(ct)
    } else {
        state.context_store.list()
    };
    contexts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let total = contexts.len();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(100).min(1000);
    let page: Vec<_> = contexts.into_iter().skip(offset).take(limit).collect();
    Json(json!({
        "contexts": page,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    #[serde(rename = "type")]
    context_type: Option<String>,
    limit: Option<usize>,
}

async fn search_contexts(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>> {
    let query = q.q.unwrap_or_default();
    if query.is_empty() {
        return Err(ApiError::bad_request("query parameter \"q\" is required"));
    }
    let mut results = state.context_store.search(&query);
    if let Some(ref ct) = q.context_type {
        results.retain(|c| c.context_type.as_str() == ct.as_str());
    }
    let limit = q.limit.unwrap_or(20).min(200);
    results.truncate(limit);
    Ok(Json(json!({
        "results": results,
        "query": query,
        "count": results.len(),
    })))
}

#[derive(Deserialize)]
pub struct CreateContextBody {
    pub uri: String,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub context_type: Option<String>,
    pub category: Option<String>,
    pub parent_uri: Option<String>,
    pub is_leaf: Option<bool>,
    pub meta: Option<std::collections::HashMap<String, Value>>,
}

async fn create_context(
    State(state): State<AppState>,
    Json(body): Json<CreateContextBody>,
) -> Result<(StatusCode, Json<Value>)> {
    if body.uri.is_empty() {
        return Err(ApiError::bad_request("uri is required"));
    }
    // Path traversal guard
    if body.uri.contains("..") || body.uri.contains("\0") {
        return Err(ApiError::bad_request("uri contains illegal characters"));
    }
    if state.context_store.get(&body.uri).is_some() {
        return Err(ApiError::conflict(format!("context already exists: {}", body.uri)));
    }
    let abs = body.abstract_text.unwrap_or_default();
    let mut builder = Context::builder(&body.uri)
        .abstract_text(&abs)
        .is_leaf(body.is_leaf.unwrap_or(false));
    if let Some(ref p) = body.parent_uri {
        builder = builder.parent_uri(p);
    }
    if let Some(ref ct) = body.context_type {
        if let Ok(parsed) = ct.parse::<ContextType>() {
            builder = builder.context_type(parsed);
        }
    }
    if let Some(ref cat) = body.category {
        builder = builder.category(cat);
    }
    let mut ctx = builder.build();
    if let Some(meta) = body.meta {
        ctx.meta = meta;
    }
    state.context_store.insert(ctx.clone());
    Ok((StatusCode::CREATED, Json(json!({ "context": ctx }))))
}

async fn get_context(
    State(state): State<AppState>,
    Path(uri_path): Path<String>,
) -> Result<Json<Value>> {
    let uri = format!("viking://{uri_path}");
    let ctx = state.context_store.get(&uri)
        .ok_or_else(|| ApiError::not_found(format!("context not found: {uri}")))?;
    Ok(Json(json!({ "context": ctx })))
}

#[derive(Deserialize)]
pub struct UpdateContextBody {
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub category: Option<String>,
    pub meta: Option<std::collections::HashMap<String, Value>>,
    pub is_leaf: Option<bool>,
}

async fn update_context(
    State(state): State<AppState>,
    Path(uri_path): Path<String>,
    Json(body): Json<UpdateContextBody>,
) -> Result<Json<Value>> {
    let uri = format!("viking://{uri_path}");
    let updated = state.context_store.update(&uri, |ctx| {
        if let Some(ref abs) = body.abstract_text {
            ctx.abstract_text = abs.clone();
        }
        if let Some(ref cat) = body.category {
            ctx.category = cat.clone();
        }
        if let Some(leaf) = body.is_leaf {
            ctx.is_leaf = leaf;
        }
        if let Some(ref meta) = body.meta {
            ctx.meta = meta.clone();
        }
        ctx.updated_at = chrono::Utc::now();
    }).ok_or_else(|| ApiError::not_found(format!("context not found: {uri}")))?;
    Ok(Json(json!({ "context": updated })))
}

async fn delete_context(
    State(state): State<AppState>,
    Path(uri_path): Path<String>,
) -> Result<StatusCode> {
    let uri = format!("viking://{uri_path}");
    state.context_store.remove(&uri)
        .ok_or_else(|| ApiError::not_found(format!("context not found: {uri}")))?;
    Ok(StatusCode::NO_CONTENT)
}

// ==================== Session Routes ====================

pub fn session_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/sessions", get(list_sessions).post(create_session))
        .route(
            "/api/v1/sessions/{id}",
            get(get_session).delete(close_session),
        )
        .route("/api/v1/sessions/{id}/messages", post(add_message))
        .route("/api/v1/sessions/{id}/commit", post(commit_session))
}

#[derive(Deserialize)]
pub struct CreateSessionBody {
    pub user_id: String,
    pub id: Option<String>,
}

async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionBody>,
) -> Result<(StatusCode, Json<Value>)> {
    if body.user_id.is_empty() {
        return Err(ApiError::bad_request("user_id is required"));
    }
    let session = if let Some(id) = body.id {
        if state.session_manager.get(&id).is_some() {
            return Err(ApiError::conflict(format!("session already exists: {id}")));
        }
        state.session_manager.create_with_id(id, &body.user_id)
    } else {
        state.session_manager.create(&body.user_id)
    };
    Ok((StatusCode::CREATED, Json(json!({ "session": session }))))
}

#[derive(Deserialize)]
pub struct SessionListQuery {
    user_id: Option<String>,
    active_only: Option<bool>,
}

async fn list_sessions(
    State(state): State<AppState>,
    Query(q): Query<SessionListQuery>,
) -> Json<Value> {
    let sessions = if q.active_only.unwrap_or(false) {
        state.session_manager.list_active()
    } else if let Some(ref uid) = q.user_id {
        state.session_manager.list_by_user(uid)
    } else {
        state.session_manager.list_active()
    };
    Json(json!({
        "sessions": sessions,
        "count": sessions.len(),
    }))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let session = state.session_manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("session not found: {id}")))?;
    Ok(Json(json!({ "session": session })))
}

async fn close_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    if !state.session_manager.close(&id) {
        return Err(ApiError::not_found(format!("session not found: {id}")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct AddMessageBody {
    pub role: String,
    pub content: String,
}

async fn add_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AddMessageBody>,
) -> Result<(StatusCode, Json<Value>)> {
    let mut session = state.session_manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("session not found: {id}")))?;
    let role = match body.role.as_str() {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        "tool" => Role::Tool,
        _ => return Err(ApiError::bad_request(format!("invalid role: {}", body.role))),
    };
    let msg = session.add_message(role, vec![Part::text(&body.content)]).clone();
    state.session_manager.update(&session);
    Ok((StatusCode::CREATED, Json(json!({ "message": msg }))))
}

async fn commit_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let mut session = state.session_manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("session not found: {id}")))?;
    let messages = session.commit();
    state.session_manager.update(&session);
    Ok(Json(json!({
        "committed_messages": messages.len(),
        "session": session,
    })))
}
