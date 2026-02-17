//! OpenViking Vector Database - Pure Rust implementation
//!
//! Provides HNSW and Flat (brute-force) vector indexes, collection management,
//! KV store, metadata management, project management, and filter support.

pub mod distance;
pub mod filter;
pub mod index;
pub mod store;
pub mod meta;
pub mod collection;
pub mod project;
pub mod error;

pub use collection::{Collection, CollectionConfig, FieldDef, FieldType};
pub use index::{VectorIndex, FlatIndex, HnswIndex};
pub use project::{Project, ProjectGroup};
pub use error::{VectorDbError, Result};
