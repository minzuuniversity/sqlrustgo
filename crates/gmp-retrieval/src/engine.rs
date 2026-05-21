//! Retrieval engine - orchestrates four channels + fusion

use gmp_core::ScoredChunk;

/// Retrieval engine configuration
#[derive(Debug, Clone)]
pub struct RetrievalEngineConfig {
    pub rrf_k: f32,
    pub rule_weight: f32,
    pub vector_weight: f32,
    pub fts_weight: f32,
    pub graph_weight: f32,
}

impl Default for RetrievalEngineConfig {
    fn default() -> Self {
        Self {
            rrf_k: 60.0,
            rule_weight: 2.0,
            vector_weight: 1.0,
            fts_weight: 1.0,
            graph_weight: 1.0,
        }
    }
}

/// Main retrieval engine - placeholder for future multi-channel orchestration
pub struct RetrievalEngine {
    config: RetrievalEngineConfig,
}

impl RetrievalEngine {
    pub fn new(config: RetrievalEngineConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &RetrievalEngineConfig {
        &self.config
    }
}
