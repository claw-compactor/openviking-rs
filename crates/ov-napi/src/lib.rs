use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::{Arc, RwLock, OnceLock};

// ========== Error Mapping ==========

fn to_napi_err(e: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(format!("{}", e))
}

fn ov_err_to_napi(e: ov_core::OvError) -> napi::Error {
    let (code, msg) = match &e {
        ov_core::OvError::ContextNotFound { uri } => ("ERR_NOT_FOUND", format!("Context not found: {}", uri)),
        ov_core::OvError::CollectionNotFound { name } => ("ERR_NOT_FOUND", format!("Collection not found: {}", name)),
        ov_core::OvError::InvalidUri(u) => ("ERR_INVALID_ARG", format!("Invalid URI: {}", u)),
        ov_core::OvError::Storage(s) => ("ERR_STORAGE", format!("Storage error: {}", s)),
        ov_core::OvError::Embedding(s) => ("ERR_EMBEDDING", format!("Embedding error: {}", s)),
        ov_core::OvError::Transaction(s) => ("ERR_TRANSACTION", format!("Transaction error: {}", s)),
        ov_core::OvError::Serialization(e) => ("ERR_SERIALIZATION", format!("Serialization error: {}", e)),
        ov_core::OvError::Other(e) => ("ERR_INTERNAL", format!("{}", e)),
    };
    napi::Error::new(Status::GenericFailure, format!("[{}] {}", code, msg))
}

// ========== Global Session Manager ==========

fn global_session_manager() -> &'static Arc<RwLock<ov_session::SessionManager>> {
    static INSTANCE: OnceLock<Arc<RwLock<ov_session::SessionManager>>> = OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(ov_session::SessionManager::new())))
}

// ========== Global Memory Store ==========

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredMemory {
    id: String,
    session_id: String,
    user_id: String,
    category: String,
    content: String,
    overview: String,
    language: String,
    created_at: String,
}

fn global_memory_store() -> &'static Arc<RwLock<Vec<StoredMemory>>> {
    static INSTANCE: OnceLock<Arc<RwLock<Vec<StoredMemory>>>> = OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(Vec::new())))
}

// ========== Ping ==========

#[napi]
pub fn ping() -> String {
    "openviking-rs v0.1.0".to_string()
}

// ========== Memory Operations ==========

#[napi(object)]
#[derive(Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub category: String,
    pub content: String,
    pub overview: String,
    pub language: String,
    pub created_at: String,
}

#[napi(object)]
pub struct AddMemoryResult {
    pub id: String,
    pub category: String,
    pub stored: bool,
}

/// Add a memory entry. Extracts category heuristically if not provided.
#[napi]
pub fn add_memory(
    content: String,
    user_id: String,
    session_id: Option<String>,
    category: Option<String>,
) -> Result<AddMemoryResult> {
    if content.trim().is_empty() {
        return Err(napi::Error::new(Status::InvalidArg, "Content cannot be empty"));
    }

    let cat = category.unwrap_or_else(|| {
        let lower = content.to_lowercase();
        if lower.contains("prefer") || lower.contains("like") { "preferences".into() }
        else if lower.contains("my name") || lower.contains("i am") { "profile".into() }
        else if lower.contains("project") { "entities".into() }
        else if lower.contains("error") || lower.contains("bug") || lower.contains("fix") { "cases".into() }
        else if lower.contains("decided") || lower.contains("event") { "events".into() }
        else { "patterns".into() }
    });

    let sid = session_id.unwrap_or_else(|| "none".into());
    let id = format!("mem_{}", uuid::Uuid::new_v4().simple());

    let overview = if content.len() > 80 {
        format!("{}...", &content[..80])
    } else {
        content.clone()
    };

    let entry = StoredMemory {
        id: id.clone(),
        session_id: sid,
        user_id,
        category: cat.clone(),
        content,
        overview,
        language: "en".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    global_memory_store().write().map_err(to_napi_err)?.push(entry);

    Ok(AddMemoryResult {
        id,
        category: cat,
        stored: true,
    })
}

/// Search memories by query string (simple text matching).
#[napi]
pub fn search_memory(
    query: String,
    user_id: Option<String>,
    category: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<MemoryEntry>> {
    let store = global_memory_store().read().map_err(to_napi_err)?;
    let max = limit.unwrap_or(10) as usize;
    let query_lower = query.to_lowercase();

    let results: Vec<MemoryEntry> = store
        .iter()
        .filter(|m| {
            let matches_query = m.content.to_lowercase().contains(&query_lower)
                || m.overview.to_lowercase().contains(&query_lower);
            let matches_user = user_id.as_ref().map_or(true, |u| &m.user_id == u);
            let matches_cat = category.as_ref().map_or(true, |c| &m.category == c);
            matches_query && matches_user && matches_cat
        })
        .take(max)
        .map(|m| MemoryEntry {
            id: m.id.clone(),
            session_id: m.session_id.clone(),
            user_id: m.user_id.clone(),
            category: m.category.clone(),
            content: m.content.clone(),
            overview: m.overview.clone(),
            language: m.language.clone(),
            created_at: m.created_at.clone(),
        })
        .collect();

    Ok(results)
}

// ========== Session Operations ==========

#[napi(object)]
#[derive(Clone)]
pub struct SessionInfo {
    pub id: String,
    pub user_id: String,
    pub state: String,
    pub message_count: u32,
    pub total_turns: u32,
    pub created_at: String,
    pub updated_at: String,
}

fn session_to_info(s: &ov_session::Session) -> SessionInfo {
    SessionInfo {
        id: s.id.clone(),
        user_id: s.user_id.clone(),
        state: format!("{:?}", s.state),
        message_count: s.message_count() as u32,
        total_turns: s.stats.total_turns as u32,
        created_at: s.created_at.to_rfc3339(),
        updated_at: s.updated_at.to_rfc3339(),
    }
}

#[napi]
pub fn create_session(user_id: Option<String>) -> Result<SessionInfo> {
    let mgr = global_session_manager().read().map_err(to_napi_err)?;
    let session = mgr.create(user_id.unwrap_or_default());
    Ok(session_to_info(&session))
}

#[napi]
pub fn get_session(session_id: String) -> Result<SessionInfo> {
    let mgr = global_session_manager().read().map_err(to_napi_err)?;
    match mgr.get(&session_id) {
        Some(s) => Ok(session_to_info(&s)),
        None => Err(napi::Error::new(
            Status::GenericFailure,
            format!("[ERR_NOT_FOUND] Session not found: {}", session_id),
        )),
    }
}

#[napi]
pub fn list_sessions(user_id: Option<String>) -> Result<Vec<SessionInfo>> {
    let mgr = global_session_manager().read().map_err(to_napi_err)?;
    let sessions = match user_id {
        Some(uid) => mgr.list_by_user(&uid),
        None => mgr.list_active(),
    };
    Ok(sessions.iter().map(session_to_info).collect())
}

/// Add a message to a session.
#[napi]
pub fn add_session_message(
    session_id: String,
    role: String,
    content: String,
) -> Result<bool> {
    let mgr = global_session_manager().read().map_err(to_napi_err)?;
    let mut session = mgr.get(&session_id).ok_or_else(|| {
        napi::Error::new(Status::GenericFailure, format!("[ERR_NOT_FOUND] Session not found: {}", session_id))
    })?;
    let r = match role.as_str() {
        "user" => ov_session::Role::User,
        "assistant" => ov_session::Role::Assistant,
        "system" => ov_session::Role::System,
        "tool" => ov_session::Role::Tool,
        _ => return Err(napi::Error::new(Status::InvalidArg, format!("Invalid role: {}", role))),
    };
    session.add_message(r, vec![ov_session::Part::text(content)]);
    mgr.update(&session);
    Ok(true)
}

/// Close a session.
#[napi]
pub fn close_session(session_id: String) -> Result<bool> {
    let mgr = global_session_manager().read().map_err(to_napi_err)?;
    Ok(mgr.close(&session_id))
}

// ========== Compactor ==========

fn parse_level(level: &str) -> ov_compactor::pipeline::CompressionLevel {
    match level {
        "lossless" => ov_compactor::pipeline::CompressionLevel::Lossless,
        "minimal" => ov_compactor::pipeline::CompressionLevel::Minimal,
        _ => ov_compactor::pipeline::CompressionLevel::Balanced,
    }
}

#[napi]
pub fn compress(text: String, level: String) -> String {
    let pipeline = ov_compactor::pipeline::CompactorPipeline::new(parse_level(&level));
    pipeline.compress(&text).output
}

#[napi(object)]
pub struct CompressionInfo {
    pub compressed: String,
    pub original_len: u32,
    pub compressed_len: u32,
    pub ratio: f64,
}

#[napi]
pub fn compress_detailed(text: String, level: String) -> CompressionInfo {
    let pipeline = ov_compactor::pipeline::CompactorPipeline::new(parse_level(&level));
    let r = pipeline.compress(&text);
    CompressionInfo {
        original_len: r.original_len as u32,
        compressed_len: r.compressed_len as u32,
        ratio: r.ratio(),
        compressed: r.output,
    }
}

// ========== Router ==========

#[napi(object)]
pub struct RoutingResult {
    pub model: String,
    pub tier: String,
    pub confidence: f64,
    pub reasoning: String,
}

#[napi]
pub fn route(prompt: String, profile: String) -> RoutingResult {
    let prof = match profile.as_str() {
        "eco" => ov_router::RoutingProfile::Eco,
        "premium" => ov_router::RoutingProfile::Premium,
        _ => ov_router::RoutingProfile::Auto,
    };
    let config = ov_router::config::default_routing_config();
    let d = ov_router::route(&prompt, None, 4096, &config, prof);
    RoutingResult {
        model: d.model,
        tier: format!("{:?}", d.tier),
        confidence: d.confidence,
        reasoning: d.reasoning,
    }
}

// ========== Vector Search ==========

#[napi(object)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f64,
}

#[napi]
pub fn vector_search(query: Vec<f64>, vectors_json: String, top_k: Option<u32>) -> Vec<VectorSearchResult> {
    let k = top_k.unwrap_or(10) as usize;
    let vectors: Vec<(String, Vec<f64>)> = serde_json::from_str(&vectors_json).unwrap_or_default();
    let q32: Vec<f32> = query.iter().map(|&x| x as f32).collect();

    let mut scores: Vec<(String, f64)> = vectors
        .iter()
        .map(|(id, v)| {
            let v32: Vec<f32> = v.iter().map(|&x| x as f32).collect();
            let score = ov_vectordb::distance::cosine_similarity(&q32, &v32);
            (id.clone(), score as f64)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(k);
    scores.into_iter().map(|(id, score)| VectorSearchResult { id, score }).collect()
}

// ========== Memory Extraction ==========

#[napi(object)]
pub struct ExtractedMemory {
    pub category: String,
    pub content: String,
    pub overview: String,
    pub language: String,
}

/// Extract memory candidates from session messages.
#[napi]
pub fn extract_memories(session_id: String) -> Result<Vec<ExtractedMemory>> {
    let mgr = global_session_manager().read().map_err(to_napi_err)?;
    let session = mgr.get(&session_id).ok_or_else(|| {
        napi::Error::new(Status::GenericFailure, format!("[ERR_NOT_FOUND] Session not found: {}", session_id))
    })?;
    let candidates = ov_session::memory::extract_candidates(&session.messages, &session.id, &session.user_id);
    Ok(candidates.into_iter().map(|c| ExtractedMemory {
        category: c.category.as_str().to_string(),
        content: c.content,
        overview: c.overview,
        language: c.language,
    }).collect())
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping() {
        assert_eq!(ping(), "openviking-rs v0.1.0");
    }

    // -- Memory tests --

    #[test]
    fn test_add_memory_basic() {
        let r = add_memory("I prefer dark mode".into(), "user1".into(), None, None).unwrap();
        assert!(r.id.starts_with("mem_"));
        assert_eq!(r.category, "preferences");
        assert!(r.stored);
    }

    #[test]
    fn test_add_memory_explicit_category() {
        let r = add_memory("some content here".into(), "user2".into(), Some("sess1".into()), Some("events".into())).unwrap();
        assert_eq!(r.category, "events");
    }

    #[test]
    fn test_add_memory_empty_content_fails() {
        let r = add_memory("".into(), "user1".into(), None, None);
        assert!(r.is_err());
    }

    #[test]
    fn test_add_memory_whitespace_only_fails() {
        let r = add_memory("   ".into(), "user1".into(), None, None);
        assert!(r.is_err());
    }

    #[test]
    fn test_add_memory_auto_category_profile() {
        let r = add_memory("my name is Duke Nukem".into(), "u".into(), None, None).unwrap();
        assert_eq!(r.category, "profile");
    }

    #[test]
    fn test_add_memory_auto_category_cases() {
        let r = add_memory("there was an error in the build".into(), "u".into(), None, None).unwrap();
        assert_eq!(r.category, "cases");
    }

    #[test]
    fn test_add_memory_auto_category_entities() {
        let r = add_memory("the project is called OpenViking".into(), "u".into(), None, None).unwrap();
        assert_eq!(r.category, "entities");
    }

    #[test]
    fn test_search_memory_finds_match() {
        // Clear and add
        add_memory("I like Rust programming very much".into(), "searcher1".into(), None, None).unwrap();
        let results = search_memory("Rust".into(), Some("searcher1".into()), None, None).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].content.contains("Rust"));
    }

    #[test]
    fn test_search_memory_no_match() {
        let results = search_memory("zzz_nonexistent_zzz".into(), Some("nobody999".into()), None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_memory_filter_by_category() {
        add_memory("I prefer tabs over spaces".into(), "cat_test".into(), None, Some("preferences".into())).unwrap();
        add_memory("the project deadline is friday".into(), "cat_test".into(), None, Some("events".into())).unwrap();
        let results = search_memory("the".into(), Some("cat_test".into()), Some("events".into()), None).unwrap();
        for r in &results {
            assert_eq!(r.category, "events");
        }
    }

    #[test]
    fn test_search_memory_limit() {
        for i in 0..5 {
            add_memory(format!("limit test item number {}", i), "limiter".into(), None, Some("patterns".into())).unwrap();
        }
        let results = search_memory("limit test".into(), Some("limiter".into()), None, Some(2)).unwrap();
        assert!(results.len() <= 2);
    }

    // -- Session tests --

    #[test]
    fn test_create_session() {
        let s = create_session(Some("testuser".into())).unwrap();
        assert!(!s.id.is_empty());
        assert_eq!(s.user_id, "testuser");
        assert_eq!(s.state, "Active");
    }

    #[test]
    fn test_get_session_exists() {
        let s = create_session(Some("getter".into())).unwrap();
        let found = get_session(s.id.clone()).unwrap();
        assert_eq!(found.id, s.id);
    }

    #[test]
    fn test_get_session_not_found() {
        let r = get_session("nonexistent-session-id".into());
        assert!(r.is_err());
    }

    #[test]
    fn test_list_sessions_empty_user() {
        let sessions = list_sessions(Some("unknown_user_xyz".into())).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_list_sessions_finds_user() {
        let _s = create_session(Some("lister42".into())).unwrap();
        let sessions = list_sessions(Some("lister42".into())).unwrap();
        assert!(!sessions.is_empty());
        assert!(sessions.iter().all(|s| s.user_id == "lister42"));
    }

    #[test]
    fn test_add_session_message() {
        let s = create_session(Some("msguser".into())).unwrap();
        let ok = add_session_message(s.id.clone(), "user".into(), "hello world".into()).unwrap();
        assert!(ok);
        let updated = get_session(s.id).unwrap();
        assert_eq!(updated.message_count, 1);
        assert_eq!(updated.total_turns, 1);
    }

    #[test]
    fn test_add_session_message_invalid_role() {
        let s = create_session(Some("roletest".into())).unwrap();
        let r = add_session_message(s.id, "invalid_role".into(), "hello".into());
        assert!(r.is_err());
    }

    #[test]
    fn test_close_session() {
        let s = create_session(Some("closer".into())).unwrap();
        let closed = close_session(s.id.clone()).unwrap();
        assert!(closed);
    }

    #[test]
    fn test_close_nonexistent_session() {
        let closed = close_session("nonexistent-close".into()).unwrap();
        assert!(!closed);
    }

    // -- Compactor tests --

    #[test]
    fn test_compress_basic() {
        let r = compress("Hello world, this is a test.".into(), "balanced".into());
        assert!(!r.is_empty());
    }

    #[test]
    fn test_compress_detailed_ratio() {
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(20);
        let r = compress_detailed(text, "balanced".into());
        assert!(r.original_len > 0);
        assert!(r.ratio > 0.0);
        assert!(r.ratio <= 1.0 || r.compressed_len <= r.original_len + 10); // allow minor overhead
    }

    // -- Router tests --

    #[test]
    fn test_route_auto() {
        let r = route("What is 2+2?".into(), "auto".into());
        assert!(!r.model.is_empty());
        assert!(r.confidence > 0.0);
    }

    #[test]
    fn test_route_eco() {
        let r = route("simple question".into(), "eco".into());
        assert!(!r.model.is_empty());
    }

    #[test]
    fn test_route_premium() {
        let r = route("complex analysis needed".into(), "premium".into());
        assert!(!r.model.is_empty());
    }

    // -- Error mapping test --

    #[test]
    fn test_ov_err_to_napi_not_found() {
        let e = ov_core::OvError::ContextNotFound { uri: "test://foo".into() };
        let ne = ov_err_to_napi(e);
        let msg = format!("{}", ne);
        assert!(msg.contains("ERR_NOT_FOUND"));
    }

    #[test]
    fn test_ov_err_to_napi_storage() {
        let e = ov_core::OvError::Storage("disk full".into());
        let ne = ov_err_to_napi(e);
        let msg = format!("{}", ne);
        assert!(msg.contains("ERR_STORAGE"));
    }

    #[test]
    fn test_ov_err_to_napi_invalid_uri() {
        let e = ov_core::OvError::InvalidUri("bad://uri".into());
        let ne = ov_err_to_napi(e);
        let msg = format!("{}", ne);
        assert!(msg.contains("ERR_INVALID_ARG"));
    }
}
