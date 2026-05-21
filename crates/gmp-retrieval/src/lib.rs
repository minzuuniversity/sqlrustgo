pub mod bm25;
pub mod rrf;
pub mod retrieval;

pub use bm25::Bm25Search;
pub use rrf::RrfFusion;
pub use retrieval::RetrievalEngine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_basic() {
        let docs = vec![
            "The quick brown fox".to_string(),
            "The lazy dog".to_string(),
            "The quick rabbit".to_string(),
        ];
        let search = Bm25Search::new(docs);
        let results = search.search("quick fox", 2);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_rrf_fusion() {
        let mut fusion = RrfFusion::new(60);
        fusion.add_result(1, 0.9);
        fusion.add_result(2, 0.8);
        fusion.add_result(1, 0.7);
        let ranked = fusion.rank();
        assert_eq!(ranked[0].doc_id, 1);
    }
}
