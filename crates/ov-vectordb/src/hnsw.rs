use async_trait::async_trait;
use crate::traits::VectorIndex;
use ov_core::context::Context;
use ov_core::types::EmbedResult;

pub struct HnswIndex {
    _dimension: usize,
}

impl HnswIndex {
    pub fn new(dimension: usize) -> Self {
        Self { _dimension: dimension }
    }
}

#[async_trait]
impl VectorIndex for HnswIndex {
    async fn insert(&self, _ctx: &Context, _embedding: &EmbedResult) -> anyhow::Result<String> {
        todo!("HNSW insert")
    }
    async fn search(&self, _vector: &[f32], _top_k: usize) -> anyhow::Result<Vec<(String, f32)>> {
        todo!("HNSW search")
    }
    async fn delete(&self, _id: &str) -> anyhow::Result<()> {
        todo!("HNSW delete")
    }
    async fn ensure_collection(&self, _name: &str, _dimension: usize) -> anyhow::Result<()> {
        Ok(())
    }
}
