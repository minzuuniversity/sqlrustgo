//! RRF (Reciprocal Rank Fusion) implementation

use gmp_core::ScoredChunk;
use std::collections::HashMap;

/// RRF fusion for combining multiple retrieval channels
pub struct RrfFusion {
    k: f32,
}

impl RrfFusion {
    pub fn new(k: f32) -> Self {
        Self { k }
    }

    /// Fuse results from multiple channels using RRF formula
    /// score = Σ (weight / (k + rank))
    pub fn fuse(&self, channel_results: Vec<(Vec<ScoredChunk>, f32)>) -> Vec<ScoredChunk> {
        let mut scores: HashMap<String, (f32, ScoredChunk)> = HashMap::new();

        for (results, weight) in channel_results {
            for (rank, chunk) in results.iter().enumerate() {
                let rrf_score = weight / (self.k + (rank + 1) as f32);
                let key = format!("{}-{}", chunk.doc_id, chunk.chunk_id);
                if let Some((old_score, _)) = scores.get(&key) {
                    scores.insert(key, (old_score + rrf_score, chunk.clone()));
                } else {
                    scores.insert(key, (rrf_score, chunk.clone()));
                }
            }
        }

        let mut fused: Vec<_> = scores.into_values().collect();
        fused.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        fused.into_iter().map(|(_, c)| c).collect()
    }
}

impl Default for RrfFusion {
    fn default() -> Self {
        Self::new(60.0)
    }
}
