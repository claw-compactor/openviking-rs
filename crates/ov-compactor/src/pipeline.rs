//! Compression pipeline — orchestrates all 5 layers.

use crate::{layer1_jsonl, layer2_ccp, layer3_dictionary, layer4_dedup, layer5_format};
use std::collections::HashMap;

/// Compression level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// ~99% fidelity. Only format cleanup + dedup.
    Lossless,
    /// ~93% fidelity. + CCP abbreviations.
    Minimal,
    /// ~87% fidelity. All layers including dictionary encoding.
    Balanced,
}

impl CompressionLevel {
    pub fn target_fidelity(&self) -> f64 {
        match self {
            Self::Lossless => 0.99,
            Self::Minimal => 0.93,
            Self::Balanced => 0.87,
        }
    }
}

/// Compression result with statistics.
#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub output: String,
    pub original_len: usize,
    pub compressed_len: usize,
    pub reduction_pct: f64,
    pub level: CompressionLevel,
    pub layers_applied: Vec<String>,
    pub codebook: Option<HashMap<String, String>>,
}

impl CompressionResult {
    pub fn ratio(&self) -> f64 {
        if self.original_len == 0 { return 1.0; }
        self.compressed_len as f64 / self.original_len as f64
    }
}

/// The main compactor pipeline.
pub struct CompactorPipeline {
    pub level: CompressionLevel,
}

impl CompactorPipeline {
    pub fn new(level: CompressionLevel) -> Self {
        Self { level }
    }

    pub fn lossless() -> Self { Self::new(CompressionLevel::Lossless) }
    pub fn minimal() -> Self { Self::new(CompressionLevel::Minimal) }
    pub fn balanced() -> Self { Self::new(CompressionLevel::Balanced) }

    /// Compress text through the pipeline.
    pub fn compress(&self, text: &str) -> CompressionResult {
        let original_len = text.len();
        let mut result = text.to_string();
        let mut layers = Vec::new();
        let mut codebook = None;

        // Layer 1: JSONL cleanup (always applied if content looks like JSONL)
        if result.lines().any(|l| l.trim_start().starts_with('{')) {
            result = layer1_jsonl::compress(&result);
            layers.push("jsonl".into());
        }

        // Layer 5: Format cleanup (always applied)
        result = layer5_format::compress(&result);
        layers.push("format".into());

        // Layer 4: Dedup (always applied)
        result = layer4_dedup::compress(&result);
        layers.push("dedup".into());

        // Layer 2: CCP (Minimal and above)
        if matches!(self.level, CompressionLevel::Minimal | CompressionLevel::Balanced) {
            result = layer2_ccp::compress(&result);
            layers.push("ccp".into());
        }

        // Layer 3: Dictionary encoding (Balanced only)
        if matches!(self.level, CompressionLevel::Balanced) {
            let texts = vec![result.as_str()];
            let cb = layer3_dictionary::build_codebook(&texts);
            if !cb.is_empty() {
                result = layer3_dictionary::compress(&result, &cb);
                layers.push("dictionary".into());
                codebook = Some(cb);
            }
        }

        let compressed_len = result.len();
        let reduction = if original_len > 0 {
            ((original_len - compressed_len) as f64 / original_len as f64) * 100.0
        } else {
            0.0
        };

        CompressionResult {
            output: result,
            original_len,
            compressed_len,
            reduction_pct: reduction,
            level: self.level,
            layers_applied: layers,
            codebook,
        }
    }

    /// Decompress (best-effort, requires codebook for dictionary layer).
    pub fn decompress(&self, text: &str, codebook: Option<&HashMap<String, String>>) -> String {
        let mut result = text.to_string();

        // Reverse dictionary
        if let Some(cb) = codebook {
            result = layer3_dictionary::decompress(&result, cb);
        }

        // Reverse CCP
        if matches!(self.level, CompressionLevel::Minimal | CompressionLevel::Balanced) {
            result = layer2_ccp::decompress(&result);
        }

        result
    }
}

impl Default for CompactorPipeline {
    fn default() -> Self {
        Self::new(CompressionLevel::Lossless)
    }
}
