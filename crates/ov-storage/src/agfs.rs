//! Agent FileSystem — virtual filesystem backed by vector store

use ov_core::context::Context;
use std::sync::Arc;
use ov_vectordb::VectorIndex;

#[allow(dead_code)]
pub struct AgFs {
    index: Arc<dyn VectorIndex>,
}

impl AgFs {
    pub fn new(index: Arc<dyn VectorIndex>) -> Self {
        Self { index }
    }

    pub async fn read(&self, _uri: &str) -> anyhow::Result<Option<Context>> {
        todo!("agfs read")
    }

    pub async fn write(&self, _ctx: &Context) -> anyhow::Result<()> {
        todo!("agfs write")
    }

    pub async fn list(&self, _parent_uri: &str) -> anyhow::Result<Vec<Context>> {
        todo!("agfs list")
    }

    pub async fn delete(&self, _uri: &str) -> anyhow::Result<()> {
        todo!("agfs delete")
    }
}
