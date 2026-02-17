//! Distance metrics for vector similarity search.

use std::fmt;

/// Supported distance metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistanceMetric {
    /// Cosine similarity (implemented as normalized IP).
    Cosine,
    /// Euclidean (L2 squared) distance.
    L2,
    /// Inner product (dot product).
    Ip,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        Self::Cosine
    }
}

impl fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cosine => write!(f, "cosine"),
            Self::L2 => write!(f, "l2"),
            Self::Ip => write!(f, "ip"),
        }
    }
}

impl DistanceMetric {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cosine" | "cos" => Self::Cosine,
            "l2" | "euclidean" => Self::L2,
            "ip" | "dot" | "inner_product" => Self::Ip,
            _ => Self::Cosine,
        }
    }
}

/// Compute inner product (dot product) of two vectors.
#[inline]
pub fn inner_product(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compute L2 squared distance.
#[inline]
pub fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(x, y)| {
        let d = x - y;
        d * d
    }).sum()
}

/// Compute cosine similarity (returns value in [-1, 1]).
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = inner_product(a, b);
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Normalize a vector in-place (L2 normalization).
pub fn normalize_vector(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Compute a similarity score. Higher = more similar.
/// For L2: returns 1.0 / (1.0 + l2_dist) so it's in (0, 1].
/// For IP/Cosine: returns the raw dot product.
pub fn compute_score(metric: DistanceMetric, a: &[f32], b: &[f32]) -> f32 {
    match metric {
        DistanceMetric::L2 => {
            let dist = l2_squared(a, b);
            1.0 / (1.0 + dist)
        }
        DistanceMetric::Ip => inner_product(a, b),
        DistanceMetric::Cosine => cosine_similarity(a, b),
    }
}
