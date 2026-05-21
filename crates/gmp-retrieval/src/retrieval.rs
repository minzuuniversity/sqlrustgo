use crate::bm25::{Bm25Result, Bm25Search};
use crate::rrf::{RrfFusion, RrfResult};

pub struct RetrievalEngine {
    bm25: Option<Bm25Search>,
}

impl RetrievalEngine {
    pub fn new() -> Self {
        Self { bm25: None }
    }

    pub fn with_bm25(documents: Vec<String>) -> Self {
        Self {
            bm25: Some(Bm25Search::new(documents)),
        }
    }

    pub fn search_bm25(&self, query: &str, top_k: usize) -> Vec<Bm25Result> {
        match &self.bm25 {
            Some(search) => search.search(query, top_k),
            None => Vec::new(),
        }
    }

    pub fn fuse_rrf(&self, results: Vec<Vec<RrfResult>>, k: usize) -> Vec<RrfResult> {
        let mut fusion = RrfFusion::new(k);
        for result_set in results {
            for rrf_result in result_set {
                fusion.add_result(rrf_result.doc_id, rrf_result.score);
            }
        }
        fusion.rank()
    }
}

impl Default for RetrievalEngine {
    fn default() -> Self {
        Self::new()
    }
}
