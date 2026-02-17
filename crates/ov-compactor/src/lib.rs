//! Claw Compactor — 5-layer context compression engine.
//!
//! Layers:
//! 1. JSONL cleanup (strip redundant metadata)
//! 2. CCP (Context Compression Protocol) — abbreviate technical terms
//! 3. Dictionary encoding — high-frequency phrase mapping
//! 4. Dedup — near-duplicate removal via shingle hashing
//! 5. Format cleanup — whitespace/markdown optimization

pub mod layer1_jsonl;
pub mod layer2_ccp;
pub mod layer3_dictionary;
pub mod layer4_dedup;
pub mod layer5_format;
pub mod pipeline;

pub use pipeline::{CompactorPipeline, CompressionLevel, CompressionResult};

#[cfg(test)]
mod tests;
