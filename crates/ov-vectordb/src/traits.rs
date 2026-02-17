use async_trait::async_trait;
use ov_core::context::Context;
use ov_core::types::EmbedResult;

#[async_trait]
pub trait VectorIndex: Send + Sync {
    async fn insert(&self, ctx: &Context, embedding: &EmbedResult) -> anyhow::Result<String>;
    async fn search(&self, vector: &[f32], top_k: usize) -> anyhow::Result<Vec<(String, f32)>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
    async fn ensure_collection(&self, name: &str, dimension: usize) -> anyhow::Result<()>;
}
