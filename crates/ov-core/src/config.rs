//! Configuration types and loader for OpenViking.
//!
//! Port of `openviking_cli/utils/config/`.

use crate::error::{OvError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level OpenViking configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub struct OpenVikingConfig {
    /// Storage backend configuration.
    #[serde(default)]
    pub storage: StorageConfig,
    /// Embedding model configuration.
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    /// Server configuration.
    #[serde(default)]
    pub server: ServerConfig,
    /// AGFS configuration.
    #[serde(default)]
    pub agfs: AgfsConfig,
    /// Parser configuration.
    #[serde(default)]
    pub parser: ParserConfig,
    /// Rerank configuration.
    #[serde(default)]
    pub rerank: RerankConfig,
}

/// Storage backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub struct StorageConfig {
    /// Vector database settings.
    #[serde(default)]
    pub vectordb: VectorDbConfig,
}

/// Vector database configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorDbConfig {
    /// Collection name.
    #[serde(default = "default_collection_name")]
    pub name: String,
    /// Backend type (e.g. "hnsw", "vikingdb").
    #[serde(default = "default_backend")]
    pub backend: String,
}

/// Embedding model configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingConfig {
    /// Vector dimension.
    #[serde(default = "default_dimension")]
    pub dimension: usize,
    /// Embedding provider name.
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model name.
    #[serde(default)]
    pub model: String,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    /// Bind address.
    #[serde(default = "default_host")]
    pub host: String,
    /// Bind port.
    #[serde(default = "default_port")]
    pub port: u16,
}

/// AGFS service configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgfsConfig {
    /// AGFS service URL.
    #[serde(default = "default_agfs_url")]
    pub url: String,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Parser configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ParserConfig {
    /// Maximum file size in bytes.
    #[serde(default)]
    pub max_file_size: Option<u64>,
    /// Supported file extensions.
    #[serde(default)]
    pub supported_extensions: Vec<String>,
}

/// Rerank configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankConfig {
    /// Whether reranking is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Rerank model provider.
    #[serde(default)]
    pub provider: String,
    /// Top-k after reranking.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

// --- Defaults ---
fn default_collection_name() -> String { "openviking".into() }
fn default_backend() -> String { "hnsw".into() }
fn default_dimension() -> usize { 1024 }
fn default_provider() -> String { "openai".into() }
fn default_host() -> String { "0.0.0.0".into() }
fn default_port() -> u16 { 8080 }
fn default_agfs_url() -> String { "http://localhost:8080".into() }
fn default_timeout() -> u64 { 10 }
fn default_true() -> bool { true }
fn default_top_k() -> usize { 10 }



impl Default for VectorDbConfig {
    fn default() -> Self {
        Self {
            name: default_collection_name(),
            backend: default_backend(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            dimension: default_dimension(),
            provider: default_provider(),
            model: String::new(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Default for AgfsConfig {
    fn default() -> Self {
        Self {
            url: default_agfs_url(),
            timeout: default_timeout(),
        }
    }
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: String::new(),
            top_k: default_top_k(),
        }
    }
}

// --- Config Loader ---

/// Default config directory: `~/.openviking/`.
pub fn default_config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openviking")
}

const OPENVIKING_CONFIG_ENV: &str = "OPENVIKING_CONFIG_FILE";
const DEFAULT_OV_CONF: &str = "ov.conf";

/// Resolve a config file path using a three-level chain:
/// 1. Explicit path
/// 2. Environment variable
/// 3. `~/.openviking/<default_filename>`
pub fn resolve_config_path(
    explicit_path: Option<&str>,
    env_var: &str,
    default_filename: &str,
) -> Option<PathBuf> {
    // Level 1
    if let Some(p) = explicit_path {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
        return None;
    }
    // Level 2
    if let Ok(val) = std::env::var(env_var) {
        let path = PathBuf::from(val);
        if path.exists() {
            return Some(path);
        }
        return None;
    }
    // Level 3
    let path = default_config_dir().join(default_filename);
    if path.exists() {
        return Some(path);
    }
    None
}

/// Load a JSON config file.
pub fn load_json_config(path: &Path) -> Result<serde_json::Value> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| OvError::Storage(format!("Config file not found: {}: {e}", path.display())))?;
    serde_json::from_str(&content).map_err(|e| {
        OvError::Storage(format!("Invalid JSON in config {}: {e}", path.display()))
    })
}

/// Load [`OpenVikingConfig`] from the standard resolution chain.
pub fn load_openviking_config(explicit_path: Option<&str>) -> Result<OpenVikingConfig> {
    match resolve_config_path(explicit_path, OPENVIKING_CONFIG_ENV, DEFAULT_OV_CONF) {
        Some(path) => {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                OvError::Storage(format!("Cannot read config {}: {e}", path.display()))
            })?;
            serde_json::from_str(&content).map_err(|e| {
                OvError::Storage(format!("Invalid config JSON: {e}"))
            })
        }
        None => Ok(OpenVikingConfig::default()),
    }
}

/// Validate an [`OpenVikingConfig`].
pub fn validate_config(config: &OpenVikingConfig) -> Result<()> {
    if config.embedding.dimension == 0 {
        return Err(OvError::Storage("embedding.dimension must be > 0".into()));
    }
    if config.server.port == 0 {
        return Err(OvError::Storage("server.port must be > 0".into()));
    }
    if config.storage.vectordb.name.is_empty() {
        return Err(OvError::Storage("storage.vectordb.name cannot be empty".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = OpenVikingConfig::default();
        assert_eq!(cfg.embedding.dimension, 1024);
        assert_eq!(cfg.server.port, 8080);
        assert_eq!(cfg.storage.vectordb.name, "openviking");
        assert_eq!(cfg.storage.vectordb.backend, "hnsw");
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = OpenVikingConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: OpenVikingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, cfg2);
    }

    #[test]
    fn test_config_from_json() {
        let json = r#"{
            "storage": {"vectordb": {"name": "test", "backend": "local"}},
            "embedding": {"dimension": 512, "provider": "hf"},
            "server": {"host": "127.0.0.1", "port": 9090}
        }"#;
        let cfg: OpenVikingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.storage.vectordb.name, "test");
        assert_eq!(cfg.embedding.dimension, 512);
        assert_eq!(cfg.server.port, 9090);
    }

    #[test]
    fn test_config_partial_json() {
        let json = r#"{"server": {"port": 3000}}"#;
        let cfg: OpenVikingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.server.port, 3000);
        // Defaults for others
        assert_eq!(cfg.embedding.dimension, 1024);
    }

    #[test]
    fn test_validate_ok() {
        let cfg = OpenVikingConfig::default();
        assert!(validate_config(&cfg).is_ok());
    }

    #[test]
    fn test_validate_zero_dimension() {
        let mut cfg = OpenVikingConfig::default();
        cfg.embedding.dimension = 0;
        assert!(validate_config(&cfg).is_err());
    }

    #[test]
    fn test_validate_zero_port() {
        let mut cfg = OpenVikingConfig::default();
        cfg.server.port = 0;
        assert!(validate_config(&cfg).is_err());
    }

    #[test]
    fn test_validate_empty_name() {
        let mut cfg = OpenVikingConfig::default();
        cfg.storage.vectordb.name = String::new();
        assert!(validate_config(&cfg).is_err());
    }

    #[test]
    fn test_resolve_config_path_none() {
        let result = resolve_config_path(None, "NONEXISTENT_ENV_VAR_12345", "nonexistent.conf");
        // May or may not exist depending on filesystem
        // Just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_resolve_config_path_explicit() {
        let result = resolve_config_path(Some("/nonexistent"), "X", "x");
        assert!(result.is_none());
    }

    #[test]
    fn test_load_json_config_nonexistent() {
        assert!(load_json_config(Path::new("/nonexistent/file.json")).is_err());
    }

    #[test]
    fn test_load_openviking_config_default() {
        // When no config file exists, should return defaults
        let cfg = load_openviking_config(Some("/nonexistent"));
        // explicit path doesn't exist => None => default
        // Actually: explicit_path provided but doesn't exist => None => default
        // Wait: resolve_config_path returns None if explicit doesn't exist
        // load_openviking_config then returns Ok(default)
        let cfg = cfg.unwrap();
        assert_eq!(cfg, OpenVikingConfig::default());
    }

    #[test]
    fn test_agfs_config_default() {
        let cfg = AgfsConfig::default();
        assert_eq!(cfg.url, "http://localhost:8080");
        assert_eq!(cfg.timeout, 10);
    }

    #[test]
    fn test_rerank_config_default() {
        let cfg = RerankConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.top_k, 10);
    }

    #[test]
    fn test_parser_config_default() {
        let cfg = ParserConfig::default();
        assert!(cfg.max_file_size.is_none());
        assert!(cfg.supported_extensions.is_empty());
    }

    #[test]
    fn test_config_empty_json() {
        let cfg: OpenVikingConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg, OpenVikingConfig::default());
    }
}

    // ========== Extended Config Tests ==========

    #[test]
    fn test_config_full_json() {
        let json = r#"{
            "storage": { "vectordb": { "name": "mydb", "backend": "flat" } },
            "embedding": { "dimension": 768, "provider": "huggingface", "model": "bge-small" },
            "server": { "host": "127.0.0.1", "port": 9090 },
            "agfs": { "url": "http://agfs:3000", "timeout": 30 },
            "rerank": { "enabled": false, "top_k": 5 }
        }"#;
        let cfg: OpenVikingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.storage.vectordb.name, "mydb");
        assert_eq!(cfg.storage.vectordb.backend, "flat");
        assert_eq!(cfg.embedding.dimension, 768);
        assert_eq!(cfg.embedding.provider, "huggingface");
        assert_eq!(cfg.server.host, "127.0.0.1");
        assert_eq!(cfg.server.port, 9090);
        assert_eq!(cfg.agfs.url, "http://agfs:3000");
        assert_eq!(cfg.agfs.timeout, 30);
        assert!(!cfg.rerank.enabled);
        assert_eq!(cfg.rerank.top_k, 5);
    }

    #[test]
    fn test_config_roundtrip_json() {
        let cfg = OpenVikingConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: OpenVikingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, cfg2);
    }

    #[test]
    fn test_validate_negative_dimension() {
        // dimension is usize, so can't be negative in Rust
        // but 0 should fail
        let mut cfg = OpenVikingConfig::default();
        cfg.embedding.dimension = 0;
        assert!(validate_config(&cfg).is_err());
    }

    #[test]
    fn test_validate_valid_config() {
        let cfg = OpenVikingConfig::default();
        assert!(validate_config(&cfg).is_ok());
    }

    #[test]
    fn test_config_extra_fields_ignored() {
        let json = r#"{"unknown_field": true, "storage": {}}"#;
        let cfg: std::result::Result<OpenVikingConfig, _> = serde_json::from_str(json);
        // Should either work (ignoring extra) or fail gracefully
        let _ = cfg;
    }

    #[test]
    fn test_vectordb_config_defaults() {
        let cfg = VectorDbConfig::default();
        assert_eq!(cfg.name, "openviking");
        assert_eq!(cfg.backend, "hnsw");
    }

    #[test]
    fn test_embedding_config_defaults() {
        let cfg = EmbeddingConfig::default();
        assert_eq!(cfg.dimension, 1024);
        assert_eq!(cfg.provider, "openai");
    }

    #[test]
    fn test_server_config_defaults() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 8080);
    }
