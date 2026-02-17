use napi_derive::napi;

#[napi]
pub fn ping() -> String {
    "openviking-rs v0.1.0".to_string()
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

// ========== Session ==========

#[napi]
pub fn create_session(user_id: Option<String>) -> String {
    let mgr = ov_session::manager::SessionManager::new();
    let session = mgr.create(user_id.unwrap_or_default());
    session.id
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

#[napi]
pub fn search_context(_query: String, _top_k: Option<u32>) -> Vec<VectorSearchResult> {
    vec![]
}
