use anyhow::Result;
use super::Chunk;

pub trait DocumentParser: Send + Sync {
    fn parse(&self, content: &[u8], mime_type: &str) -> Result<Vec<Chunk>>;
}
