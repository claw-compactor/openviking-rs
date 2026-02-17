use std::collections::HashMap;
use std::path::Path;
use parking_lot::RwLock;
use crate::distance::{self, DistanceMetric};
use crate::error::{Result, VectorDbError};
use super::{SearchResult, traits::VectorIndex};

/// Brute-force (flat) vector index.
/// Exact nearest-neighbor search by scanning all vectors.
pub struct FlatIndex {
    dimension: usize,
    metric: DistanceMetric,
    inner: RwLock<FlatInner>,
}

struct FlatInner {
    labels: Vec<u64>,
    vectors: Vec<Vec<f32>>,
    label_to_idx: HashMap<u64, usize>,
    /// Tracks deleted slots for compaction.
    deleted_count: usize,
}

impl FlatIndex {
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        Self {
            dimension,
            metric,
            inner: RwLock::new(FlatInner {
                labels: Vec::new(),
                vectors: Vec::new(),
                label_to_idx: HashMap::new(),
                deleted_count: 0,
            }),
        }
    }

    /// Create with pre-allocated capacity.
    pub fn with_capacity(dimension: usize, metric: DistanceMetric, capacity: usize) -> Self {
        Self {
            dimension,
            metric,
            inner: RwLock::new(FlatInner {
                labels: Vec::with_capacity(capacity),
                vectors: Vec::with_capacity(capacity),
                label_to_idx: HashMap::with_capacity(capacity),
                deleted_count: 0,
            }),
        }
    }
}

impl VectorIndex for FlatIndex {
    fn insert(&self, label: u64, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(VectorDbError::DimensionMismatch {
                expected: self.dimension,
                got: vector.len(),
            });
        }
        let mut inner = self.inner.write();
        let mut vec = vector.to_vec();
        // For cosine, normalize on insert
        if self.metric == DistanceMetric::Cosine {
            distance::normalize_vector(&mut vec);
        }
        if let Some(&idx) = inner.label_to_idx.get(&label) {
            // Update existing
            inner.vectors[idx] = vec;
        } else {
            let idx = inner.labels.len();
            inner.labels.push(label);
            inner.vectors.push(vec);
            inner.label_to_idx.insert(label, idx);
        }
        Ok(())
    }

    fn delete(&self, label: u64) -> Result<()> {
        let mut inner = self.inner.write();
        if let Some(idx) = inner.label_to_idx.remove(&label) {
            // Swap-remove for O(1) deletion
            let last = inner.labels.len() - 1;
            if idx != last {
                let moved_label = inner.labels[last];
                inner.labels.swap(idx, last);
                inner.vectors.swap(idx, last);
                inner.label_to_idx.insert(moved_label, idx);
            }
            inner.labels.pop();
            inner.vectors.pop();
        }
        Ok(())
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<SearchResult> {
        if query.len() != self.dimension {
            return Err(VectorDbError::DimensionMismatch {
                expected: self.dimension,
                got: query.len(),
            });
        }
        let inner = self.inner.read();
        if inner.labels.is_empty() || top_k == 0 {
            return Ok(SearchResult::empty());
        }

        let query_vec = if self.metric == DistanceMetric::Cosine {
            let mut q = query.to_vec();
            distance::normalize_vector(&mut q);
            q
        } else {
            query.to_vec()
        };

        let effective_metric = if self.metric == DistanceMetric::Cosine {
            DistanceMetric::Ip // normalized vectors: cosine = IP
        } else {
            self.metric
        };

        // Compute all scores
        let mut scored: Vec<(u64, f32)> = inner.labels.iter().zip(inner.vectors.iter())
            .map(|(&label, vec)| {
                let score = distance::compute_score(effective_metric, &query_vec, vec);
                (label, score)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(SearchResult {
            ids: scored.iter().map(|s| s.0).collect(),
            scores: scored.iter().map(|s| s.1).collect(),
        })
    }

    fn len(&self) -> usize {
        self.inner.read().labels.len()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn metric(&self) -> DistanceMetric {
        self.metric
    }

    fn save(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let inner = self.inner.read();
        let dir = path;
        std::fs::create_dir_all(dir)?;
        let file_path = dir.join("flat_index.bin");
        let mut f = std::fs::File::create(&file_path)?;

        // Format: dim(u32) | count(u64) | [label(u64) | vector(f32 * dim)] ...
        let dim = self.dimension as u32;
        let count = inner.labels.len() as u64;
        f.write_all(&dim.to_le_bytes())?;
        f.write_all(&count.to_le_bytes())?;
        for i in 0..inner.labels.len() {
            f.write_all(&inner.labels[i].to_le_bytes())?;
            for &val in &inner.vectors[i] {
                f.write_all(&val.to_le_bytes())?;
            }
        }
        // Write metric
        let metric_byte = match self.metric {
            DistanceMetric::Cosine => 0u8,
            DistanceMetric::L2 => 1u8,
            DistanceMetric::Ip => 2u8,
        };
        f.write_all(&[metric_byte])?;
        f.flush()?;
        Ok(())
    }

    fn load(&mut self, path: &Path) -> Result<()> {
        use std::io::Read;
        let file_path = path.join("flat_index.bin");
        let mut f = std::fs::File::open(&file_path)?;
        let mut buf4 = [0u8; 4];
        let mut buf8 = [0u8; 8];

        f.read_exact(&mut buf4)?;
        let dim = u32::from_le_bytes(buf4) as usize;
        f.read_exact(&mut buf8)?;
        let count = u64::from_le_bytes(buf8) as usize;

        let mut inner = self.inner.write();
        inner.labels.clear();
        inner.vectors.clear();
        inner.label_to_idx.clear();

        for i in 0..count {
            f.read_exact(&mut buf8)?;
            let label = u64::from_le_bytes(buf8);
            let mut vec = vec![0f32; dim];
            for v in vec.iter_mut() {
                f.read_exact(&mut buf4)?;
                *v = f32::from_le_bytes(buf4);
            }
            inner.label_to_idx.insert(label, i);
            inner.labels.push(label);
            inner.vectors.push(vec);
        }
        self.dimension = dim;
        Ok(())
    }

    fn needs_rebuild(&self) -> bool {
        false
    }
}
