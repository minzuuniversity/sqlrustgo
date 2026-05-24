//! HNSW Parameter Auto-Tuner
//!
//! Automatically selects optimal HNSW parameters (m, ef_search, efConstruction)
//! based on the number of vectors in the index.

use crate::hnsw_vector_index::HnswVectorIndex;

/// HNSW parameter presets by dataset size
#[derive(Debug, Clone, Copy)]
pub struct HnswParams {
    /// m: number of bi-directional links per layer-0 node
    pub m: u32,
    /// ef_search: search width during query
    pub ef_search: u32,
    /// ef_construction: index build width
    pub ef_construction: u32,
}

impl HnswParams {
    /// Auto-select parameters based on vector count
    pub fn for_count(count: usize) -> Self {
        match count {
            0..=1_000 => Self {
                // Small corpus: less links, faster build
                m: 8,
                ef_search: 50,
                ef_construction: 100,
            },
            1_001..=10_000 => Self {
                // Medium corpus: balanced
                m: 16,
                ef_search: 100,
                ef_construction: 200,
            },
            10_001..=100_000 => Self {
                // Large corpus: higher recall
                m: 32,
                ef_search: 200,
                ef_construction: 400,
            },
            _ => Self {
                // Very large corpus: maximum quality
                m: 64,
                ef_search: 400,
                ef_construction: 800,
            },
        }
    }
}

/// Auto-tune HNSW index parameters based on current vector count
pub fn auto_tune_hnsw(index: &HnswVectorIndex, current_count: usize) -> HnswParams {
    let params = HnswParams::for_count(current_count);

    // Log the tuning decision
    eprintln!(
        "[hnsw_tuner] vectors={}, m={}, ef_search={}, ef_construction={}",
        current_count, params.m, params.ef_search, params.ef_construction
    );

    params
}

#[cfg(test)]
mod tests {
    use super::HnswParams;

    #[test]
    fn test_small_corpus() {
        let p = HnswParams::for_count(500);
        assert_eq!(p.m, 8);
        assert_eq!(p.ef_search, 50);
    }

    #[test]
    fn test_medium_corpus() {
        let p = HnswParams::for_count(5_000);
        assert_eq!(p.m, 16);
        assert_eq!(p.ef_search, 100);
    }

    #[test]
    fn test_large_corpus() {
        let p = HnswParams::for_count(50_000);
        assert_eq!(p.m, 32);
        assert_eq!(p.ef_search, 200);
    }

    #[test]
    fn test_very_large_corpus() {
        let p = HnswParams::for_count(500_000);
        assert_eq!(p.m, 64);
        assert_eq!(p.ef_search, 400);
    }
}
