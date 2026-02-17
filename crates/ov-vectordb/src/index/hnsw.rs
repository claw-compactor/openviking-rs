use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;
use std::path::Path;
use parking_lot::RwLock;
use rand::Rng;
use crate::distance::{self, DistanceMetric};
use crate::error::{Result, VectorDbError};
use super::{SearchResult, traits::VectorIndex};

/// HNSW (Hierarchical Navigable Small World) index.
///
/// Parameters:
/// - `m`: Number of connections per layer (default 16)
/// - `ef_construction`: Size of dynamic candidate list during construction (default 200)
/// - `ef_search`: Size of dynamic candidate list during search (default 50)
pub struct HnswIndex {
    dimension: usize,
    metric: DistanceMetric,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
    inner: RwLock<HnswInner>,
}

struct HnswInner {
    /// All vectors stored by internal id.
    vectors: Vec<Vec<f32>>,
    /// Map from user label to internal id.
    label_to_id: HashMap<u64, usize>,
    /// Map from internal id to user label.
    id_to_label: Vec<u64>,
    /// Adjacency lists per layer. layers[level][node_id] = neighbors.
    layers: Vec<Vec<Vec<usize>>>,
    /// Maximum level for each node.
    node_levels: Vec<usize>,
    /// Entry point (internal id).
    entry_point: Option<usize>,
    /// Maximum level in the graph.
    max_level: usize,
    /// Deleted set.
    deleted: HashSet<usize>,
    /// ML = 1/ln(M)
    ml: f64,
}

impl HnswIndex {
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        Self::with_params(dimension, metric, 16, 200, 50)
    }

    pub fn with_params(
        dimension: usize,
        metric: DistanceMetric,
        m: usize,
        ef_construction: usize,
        ef_search: usize,
    ) -> Self {
        let ml = 1.0 / (m as f64).ln();
        Self {
            dimension,
            metric,
            m,
            ef_construction,
            ef_search,
            inner: RwLock::new(HnswInner {
                vectors: Vec::new(),
                label_to_id: HashMap::new(),
                id_to_label: Vec::new(),
                layers: Vec::new(),
                node_levels: Vec::new(),
                entry_point: None,
                max_level: 0,
                deleted: HashSet::new(),
                ml,
            }),
        }
    }

    fn compute_score_inner(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.metric {
            DistanceMetric::Cosine => distance::cosine_similarity(a, b),
            DistanceMetric::Ip => distance::inner_product(a, b),
            DistanceMetric::L2 => {
                let d = distance::l2_squared(a, b);
                1.0 / (1.0 + d)
            }
        }
    }

    fn random_level(ml: f64) -> usize {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();
        (-r.ln() * ml).floor() as usize
    }
}

impl VectorIndex for HnswIndex {
    fn insert(&self, label: u64, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(VectorDbError::DimensionMismatch {
                expected: self.dimension,
                got: vector.len(),
            });
        }

        let mut vec = vector.to_vec();
        if self.metric == DistanceMetric::Cosine {
            distance::normalize_vector(&mut vec);
        }

        let mut inner = self.inner.write();

        // Check if updating existing
        if let Some(&id) = inner.label_to_id.get(&label) {
            inner.vectors[id] = vec;
            inner.deleted.remove(&id);
            return Ok(());
        }

        let new_id = inner.vectors.len();
        let level = Self::random_level(inner.ml);

        inner.vectors.push(vec);
        inner.label_to_id.insert(label, new_id);
        inner.id_to_label.push(label);
        inner.node_levels.push(level);

        // Ensure layers exist up to this level
        while inner.layers.len() <= level {
            inner.layers.push(Vec::new());
        }
        for l in 0..=level {
            while inner.layers[l].len() <= new_id {
                inner.layers[l].push(Vec::new());
            }
        }

        if inner.entry_point.is_none() {
            inner.entry_point = Some(new_id);
            inner.max_level = level;
            return Ok(());
        }

        let ep = inner.entry_point.unwrap();
        let mut curr_ep = ep;

        // Traverse from top level down to level+1 with greedy search
        let query = inner.vectors[new_id].clone();
        for lev in (level + 1..=inner.max_level).rev() {
            curr_ep = greedy_closest(&inner.vectors, &inner.layers, lev, curr_ep, &query, &inner.deleted, self.metric);
        }

        // For levels [min(level, max_level) down to 0], do ef_construction search and connect
        let top = std::cmp::min(level, inner.max_level);
        for lev in (0..=top).rev() {
            let candidates = search_layer(
                &inner.vectors,
                &inner.layers,
                lev,
                curr_ep,
                &query,
                self.ef_construction,
                &inner.deleted,
                self.metric,
            );

            // Select M best neighbors
            let max_neighbors = if lev == 0 { self.m * 2 } else { self.m };
            let neighbors: Vec<usize> = candidates.iter()
                .take(max_neighbors)
                .map(|&(id, _)| id)
                .collect();

            // Ensure adjacency lists exist
            while inner.layers[lev].len() <= new_id {
                inner.layers[lev].push(Vec::new());
            }

            // Connect new_id to neighbors
            inner.layers[lev][new_id] = neighbors.clone();

            // Bidirectional connections
            for &neighbor in &neighbors {
                while inner.layers[lev].len() <= neighbor {
                    inner.layers[lev].push(Vec::new());
                }
                inner.layers[lev][neighbor].push(new_id);
                // Prune if over-connected
                if inner.layers[lev][neighbor].len() > max_neighbors {
                    let nv = &inner.vectors[neighbor];
                    let mut scored: Vec<(usize, f32)> = inner.layers[lev][neighbor].iter()
                        .map(|&n| {
                            let s = distance::compute_score(self.metric, nv, &inner.vectors[n]);
                            (n, s)
                        })
                        .collect();
                    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    scored.truncate(max_neighbors);
                    inner.layers[lev][neighbor] = scored.into_iter().map(|(id, _)| id).collect();
                }
            }

            if !candidates.is_empty() {
                curr_ep = candidates[0].0;
            }
        }

        if level > inner.max_level {
            inner.entry_point = Some(new_id);
            inner.max_level = level;
        }

        Ok(())
    }

    fn delete(&self, label: u64) -> Result<()> {
        let mut inner = self.inner.write();
        if let Some(&id) = inner.label_to_id.get(&label) {
            inner.deleted.insert(id);
            inner.label_to_id.remove(&label);
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
        if inner.entry_point.is_none() || top_k == 0 {
            return Ok(SearchResult::empty());
        }

        let query_vec = if self.metric == DistanceMetric::Cosine {
            let mut q = query.to_vec();
            distance::normalize_vector(&mut q);
            q
        } else {
            query.to_vec()
        };

        let ep = inner.entry_point.unwrap();
        let mut curr_ep = ep;

        // Traverse from top level down to level 1
        for lev in (1..=inner.max_level).rev() {
            curr_ep = greedy_closest(&inner.vectors, &inner.layers, lev, curr_ep, &query_vec, &inner.deleted, self.metric);
        }

        // Search at level 0 with ef_search
        let ef = std::cmp::max(self.ef_search, top_k);
        let candidates = search_layer(
            &inner.vectors,
            &inner.layers,
            0,
            curr_ep,
            &query_vec,
            ef,
            &inner.deleted,
            self.metric,
        );

        let mut results: Vec<(u64, f32)> = candidates.into_iter()
            .filter(|&(id, _)| !inner.deleted.contains(&id))
            .take(top_k)
            .map(|(id, score)| (inner.id_to_label[id], score))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(SearchResult {
            ids: results.iter().map(|r| r.0).collect(),
            scores: results.iter().map(|r| r.1).collect(),
        })
    }

    fn len(&self) -> usize {
        let inner = self.inner.read();
        inner.label_to_id.len()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn metric(&self) -> DistanceMetric {
        self.metric
    }

    fn save(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        std::fs::create_dir_all(path)?;
        let inner = self.inner.read();

        let data = HnswSerData {
            dimension: self.dimension,
            m: self.m,
            ef_construction: self.ef_construction,
            ef_search: self.ef_search,
            metric: self.metric,
            vectors: &inner.vectors,
            id_to_label: &inner.id_to_label,
            node_levels: &inner.node_levels,
            layers: &inner.layers,
            entry_point: inner.entry_point,
            max_level: inner.max_level,
        };

        let json = serde_json::to_vec(&data).map_err(|e| VectorDbError::Serialization(e.to_string()))?;
        let file_path = path.join("hnsw_index.json");
        let mut f = std::fs::File::create(&file_path)?;
        f.write_all(&json)?;
        f.flush()?;
        Ok(())
    }

    fn load(&mut self, path: &Path) -> Result<()> {
        let file_path = path.join("hnsw_index.json");
        let data = std::fs::read(&file_path)?;
        let deser: HnswDeserData = serde_json::from_slice(&data)
            .map_err(|e| VectorDbError::Serialization(e.to_string()))?;

        self.dimension = deser.dimension;
        self.m = deser.m;
        self.ef_construction = deser.ef_construction;
        self.ef_search = deser.ef_search;
        self.metric = deser.metric;

        let mut inner = self.inner.write();
        inner.vectors = deser.vectors;
        inner.id_to_label = deser.id_to_label;
        inner.node_levels = deser.node_levels;
        inner.layers = deser.layers;
        inner.entry_point = deser.entry_point;
        inner.max_level = deser.max_level;
        inner.ml = 1.0 / (self.m as f64).ln();
        inner.deleted.clear();

        // Rebuild label_to_id
        inner.label_to_id.clear();
        let labels_copy: Vec<(usize, u64)> = inner.id_to_label.iter().enumerate().map(|(i, &l)| (i, l)).collect();
        for (id, label) in labels_copy {
            inner.label_to_id.insert(label, id);
        }

        Ok(())
    }

    fn needs_rebuild(&self) -> bool {
        let inner = self.inner.read();
        let total = inner.vectors.len();
        if total == 0 { return false; }
        inner.deleted.len() * 2 > total
    }
}

// -- Helper functions --

fn greedy_closest(
    vectors: &[Vec<f32>],
    layers: &[Vec<Vec<usize>>],
    level: usize,
    start: usize,
    query: &[f32],
    deleted: &HashSet<usize>,
    metric: DistanceMetric,
) -> usize {
    let mut current = start;
    let mut current_score = distance::compute_score(metric, query, &vectors[current]);

    loop {
        let mut changed = false;
        if level < layers.len() && current < layers[level].len() {
            for &neighbor in &layers[level][current] {
                if deleted.contains(&neighbor) { continue; }
                let score = distance::compute_score(metric, query, &vectors[neighbor]);
                if score > current_score {
                    current = neighbor;
                    current_score = score;
                    changed = true;
                }
            }
        }
        if !changed { break; }
    }
    current
}

/// Search a single layer, returns candidates sorted by score descending.
fn search_layer(
    vectors: &[Vec<f32>],
    layers: &[Vec<Vec<usize>>],
    level: usize,
    entry: usize,
    query: &[f32],
    ef: usize,
    deleted: &HashSet<usize>,
    metric: DistanceMetric,
) -> Vec<(usize, f32)> {
    let mut visited = HashSet::new();
    let entry_score = distance::compute_score(metric, query, &vectors[entry]);

    // Max-heap for candidates (we want highest score)
    let mut candidates: BinaryHeap<(ordered_float::OrderedFloat<f32>, usize)> = BinaryHeap::new();
    // Min-heap for results (track worst in result set)
    let mut results: BinaryHeap<Reverse<(ordered_float::OrderedFloat<f32>, usize)>> = BinaryHeap::new();

    candidates.push((ordered_float::OrderedFloat(entry_score), entry));
    if !deleted.contains(&entry) {
        results.push(Reverse((ordered_float::OrderedFloat(entry_score), entry)));
    }
    visited.insert(entry);

    while let Some((ordered_float::OrderedFloat(cand_score), cand_id)) = candidates.pop() {
        // If worst result is better than best candidate, stop
        if let Some(&Reverse((ordered_float::OrderedFloat(worst_score), _))) = results.peek() {
            if results.len() >= ef && cand_score < worst_score {
                break;
            }
        }

        if level < layers.len() && cand_id < layers[level].len() {
            for &neighbor in &layers[level][cand_id] {
                if !visited.insert(neighbor) { continue; }
                let score = distance::compute_score(metric, query, &vectors[neighbor]);

                let should_add = if results.len() < ef {
                    true
                } else if let Some(&Reverse((ordered_float::OrderedFloat(worst), _))) = results.peek() {
                    score > worst
                } else {
                    true
                };

                if should_add {
                    candidates.push((ordered_float::OrderedFloat(score), neighbor));
                    if !deleted.contains(&neighbor) {
                        results.push(Reverse((ordered_float::OrderedFloat(score), neighbor)));
                        if results.len() > ef {
                            results.pop();
                        }
                    }
                }
            }
        }
    }

    let mut result_vec: Vec<(usize, f32)> = results.into_iter()
        .map(|Reverse((ordered_float::OrderedFloat(score), id))| (id, score))
        .collect();
    result_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result_vec
}

// Serialization helpers
#[derive(serde::Serialize)]
struct HnswSerData<'a> {
    dimension: usize,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
    metric: DistanceMetric,
    vectors: &'a Vec<Vec<f32>>,
    id_to_label: &'a Vec<u64>,
    node_levels: &'a Vec<usize>,
    layers: &'a Vec<Vec<Vec<usize>>>,
    entry_point: Option<usize>,
    max_level: usize,
}

#[derive(serde::Deserialize)]
struct HnswDeserData {
    dimension: usize,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
    metric: DistanceMetric,
    vectors: Vec<Vec<f32>>,
    id_to_label: Vec<u64>,
    node_levels: Vec<usize>,
    layers: Vec<Vec<Vec<usize>>>,
    entry_point: Option<usize>,
    max_level: usize,
}
