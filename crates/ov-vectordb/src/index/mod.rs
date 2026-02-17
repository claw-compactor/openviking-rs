//! Vector index implementations: Flat (brute-force) and HNSW.

mod flat;
mod hnsw;
mod traits;

pub use flat::FlatIndex;
pub use hnsw::HnswIndex;
pub use traits::VectorIndex;

/// Search result: (id, score) pairs sorted by descending score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub ids: Vec<u64>,
    pub scores: Vec<f32>,
}

impl SearchResult {
    pub fn empty() -> Self {
        Self { ids: vec![], scores: vec![] }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}
