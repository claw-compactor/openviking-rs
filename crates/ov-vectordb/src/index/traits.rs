use crate::distance::DistanceMetric;
use crate::error::Result;
use super::SearchResult;
use std::path::Path;

/// Core trait for vector index implementations.
pub trait VectorIndex: Send + Sync {
    /// Insert a vector with the given label.
    fn insert(&self, label: u64, vector: &[f32]) -> Result<()>;

    /// Batch insert vectors.
    fn insert_batch(&self, labels: &[u64], vectors: &[Vec<f32>]) -> Result<()> {
        for (label, vec) in labels.iter().zip(vectors.iter()) {
            self.insert(*label, vec)?;
        }
        Ok(())
    }

    /// Delete a vector by label.
    fn delete(&self, label: u64) -> Result<()>;

    /// Search for the top-k nearest vectors.
    fn search(&self, query: &[f32], top_k: usize) -> Result<SearchResult>;

    /// Get the number of vectors in the index.
    fn len(&self) -> usize;

    /// Check if the index is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the dimension of vectors in this index.
    fn dimension(&self) -> usize;

    /// Get the distance metric used.
    fn metric(&self) -> DistanceMetric;

    /// Persist the index to disk.
    fn save(&self, path: &Path) -> Result<()>;

    /// Load the index from disk.
    fn load(&mut self, path: &Path) -> Result<()>;

    /// Check if rebuild is needed.
    fn needs_rebuild(&self) -> bool {
        false
    }
}
