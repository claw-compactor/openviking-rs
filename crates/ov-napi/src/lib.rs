use napi_derive::napi;

#[napi]
pub fn ping() -> String {
    "openviking-rs".to_string()
}

#[napi]
pub async fn create_session(user_id: Option<String>) -> napi::Result<String> {
    let mgr = ov_session::manager::SessionManager::new();
    let session = mgr.create(user_id);
    Ok(session.id)
}

#[napi(object)]
pub struct SearchResult {
    pub id: String,
    pub score: f64,
    pub uri: String,
    pub abstract_text: String,
}

#[napi]
pub async fn search_context(_query: String, _top_k: Option<u32>) -> napi::Result<Vec<SearchResult>> {
    // Placeholder
    Ok(vec![])
}
