use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenVikingConfig {
    pub storage: StorageConfig,
    pub embedding: EmbeddingConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub vectordb: VectorDbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDbConfig {
    pub name: String,
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub dimension: usize,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for OpenVikingConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig {
                vectordb: VectorDbConfig {
                    name: "openviking".into(),
                    backend: "hnsw".into(),
                },
            },
            embedding: EmbeddingConfig {
                dimension: 1024,
                provider: "openai".into(),
            },
            server: ServerConfig {
                host: "0.0.0.0".into(),
                port: 8080,
            },
        }
    }
}
