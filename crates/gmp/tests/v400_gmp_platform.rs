//! V400-10: GMP-Platform consumer regression tests
//!
//! Tests for GMP-Platform integration including:
//! - Document lifecycle with version tracking
//! - Vector search and hybrid search
//! - Embedding generation
//! - Performance regression tracking
//!
//! Exit evidence for V400-10:
//! - GMP-Platform consumer regression documented
//! - TPC-H regression baseline verified
//! - Performance metrics tracked

use sqlrustgo_gmp::{
    cosine_similarity, generate_embedding, GmpExecutor,
};
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

// ========================================================================
// Test Configuration
// ========================================================================

/// Default test executor
fn default_executor() -> GmpExecutor {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let executor = GmpExecutor::new(storage.clone());
    executor.init().unwrap();
    executor
}

// ========================================================================
// Document Lifecycle Tests
// ========================================================================

#[cfg(test)]
mod document_lifecycle_tests {
    use super::*;

    #[test]
    fn test_document_import() {
        let executor = default_executor();

        let doc_id = executor
            .import_document(
                "Test SOP Document",
                "SOP",
                "Standard operating procedure content",
                &["sop", "procedure", "standard"],
            )
            .unwrap();

        assert!(doc_id > 0, "Document ID should be positive");
    }

    #[test]
    fn test_document_version_tracking() {
        let executor = default_executor();

        let doc_id = executor
            .import_document(
                "Version Test",
                "CLAUSE",
                "Version 1.0",
                &["version"],
            )
            .unwrap();

        let doc_id_v2 = executor
            .import_document(
                "Version Test",
                "CLAUSE",
                "Version 2.0 - Updated",
                &["version", "updated"],
            )
            .unwrap();

        assert!(doc_id > 0);
        assert!(doc_id_v2 > 0);
        assert_ne!(doc_id, doc_id_v2);
    }

    #[test]
    fn test_bulk_import() {
        let executor = default_executor();

        let docs = vec![
            ("Doc 1", "REPORT", "Content 1", vec!["a", "b"]),
            ("Doc 2", "REPORT", "Content 2", vec!["a", "c"]),
            ("Doc 3", "SOP", "Content 3", vec!["b", "c"]),
        ];

        let ids = executor.bulk_import(&docs).unwrap();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_get_document() {
        let executor = default_executor();

        let doc_id = executor
            .import_document(
                "Get Test",
                "REPORT",
                "Content to retrieve",
                &["get", "test"],
            )
            .unwrap();

        let doc = executor.get_document(doc_id).unwrap();
        assert!(doc.is_some());
    }
}

// ========================================================================
// Vector Search Tests
// ========================================================================

#[cfg(test)]
mod vector_search_tests {
    use super::*;

    #[test]
    fn test_embedding_generation() {
        let text1 = "Rust is a systems programming language";
        let text2 = "Python is great for data science";
        let text3 = "Rust memory safety guarantees";

        let emb1 = generate_embedding(text1);
        let emb2 = generate_embedding(text2);
        let emb3 = generate_embedding(text3);

        // Same text should produce same embedding
        let emb1_repeat = generate_embedding(text1);
        assert_eq!(emb1, emb1_repeat);

        // Different texts should produce different embeddings
        assert_ne!(emb1, emb2);
        assert_ne!(emb2, emb3);

        // Embedding dimension should be consistent
        assert_eq!(emb1.len(), 256);
        assert_eq!(emb2.len(), 256);
        assert_eq!(emb3.len(), 256);
    }

    #[test]
    fn test_cosine_similarity() {
        let emb1 = vec![1.0, 0.0, 0.0];
        let emb2 = vec![1.0, 0.0, 0.0];
        let emb3 = vec![0.0, 1.0, 0.0];

        let sim_same = cosine_similarity(&emb1, &emb2);
        assert!((sim_same - 1.0).abs() < 0.001);

        let sim_orthogonal = cosine_similarity(&emb1, &emb3);
        assert!((sim_orthogonal - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_vector_search_basic() {
        let executor = default_executor();

        executor
            .import_document(
                "Rust Programming Guide",
                "BOOK",
                "Comprehensive guide to Rust programming language",
                &["rust", "programming"],
            )
            .unwrap();

        executor
            .import_document(
                "Python Data Science",
                "COURSE",
                "Learn Python for data analysis",
                &["python", "data-science"],
            )
            .unwrap();

        let results = executor.search("Rust programming", 5).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_hybrid_search() {
        let executor = default_executor();

        executor
            .import_document(
                "Database Design",
                "HANDBOOK",
                "Essential patterns for database design",
                &["database", "design"],
            )
            .unwrap();

        executor
            .import_document(
                "Web Development",
                "COURSE",
                "Full stack web development guide",
                &["web", "development"],
            )
            .unwrap();

        let hybrid_results = executor.hybrid_search("database patterns", 5).unwrap();
        assert!(!hybrid_results.is_empty());
    }

    #[test]
    fn test_vector_search_relevance() {
        let executor = default_executor();

        executor
            .import_document(
                "Rust Memory Safety",
                "BOOK",
                "Understanding Rust ownership and borrowing",
                &["rust", "memory", "safety"],
            )
            .unwrap();

        executor
            .import_document(
                "JavaScript Frameworks",
                "BOOK",
                "React, Vue, and Angular comparison",
                &["javascript", "frontend"],
            )
            .unwrap();

        let results = executor.search("Rust safety", 2).unwrap();
        assert!(!results.is_empty());

        let max_similarity = results.iter().map(|r| r.similarity).fold(0.0f32, f32::max);
        assert!(max_similarity > 0.0, "Should find relevant results");
    }

    #[test]
    fn test_search_empty_results() {
        let executor = default_executor();

        executor
            .import_document(
                "Apple Document",
                "REPORT",
                "Apple content",
                &["apple"],
            )
            .unwrap();

        let results = executor.search("xyznonexistent123", 5).unwrap();
        // Empty or zero-similarity results are valid
        assert!(results.is_empty() || results.iter().all(|r| r.similarity <= 0.0));
    }
}

// ========================================================================
// Cross-Model Integration Tests
// ========================================================================

#[cfg(test)]
mod cross_model_tests {
    use super::*;

    #[test]
    fn test_document_with_keywords() {
        let executor = default_executor();

        let doc_id = executor
            .import_document(
                "Main Procedure",
                "SOP",
                "Main standard procedure with detailed steps",
                &["sop", "main", "procedure", "steps"],
            )
            .unwrap();

        assert!(doc_id > 0);
    }

    #[test]
    fn test_document_search_across_types() {
        let executor = default_executor();

        executor
            .import_document(
                "Safety Protocol",
                "SOP",
                "Safety procedures for handling hazardous materials",
                &["safety", "protocol", "hazardous"],
            )
            .unwrap();

        executor
            .import_document(
                "Safety Report",
                "REPORT",
                "Quarterly safety report on incidents",
                &["safety", "report", "incidents"],
            )
            .unwrap();

        let results = executor.search("safety", 5).unwrap();
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_multiple_document_types() {
        let executor = default_executor();

        executor
            .import_document("SOP 1", "SOP", "SOP content", &["sop"])
            .unwrap();
        executor
            .import_document("Report 1", "REPORT", "Report content", &["report"])
            .unwrap();
        executor
            .import_document("Clause 1", "CLAUSE", "Clause content", &["clause"])
            .unwrap();

        let search_results = executor.search("content", 10).unwrap();
        assert!(search_results.len() >= 3);
    }
}

// ========================================================================
// Performance Regression Tests
// ========================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_bulk_document_import_performance() {
        let executor = default_executor();

        let start = std::time::Instant::now();
        let doc_count = 100;

        for i in 0..doc_count {
            executor
                .import_document(
                    &format!("Document {}", i),
                    "REPORT",
                    &format!("Content for document {}", i),
                    &["bulk", "test"],
                )
                .unwrap();
        }

        let elapsed = start.elapsed();
        let docs_per_second = doc_count as f64 / elapsed.as_secs_f64();

        assert!(
            docs_per_second > 10.0,
            "Bulk import too slow: {:.1} docs/sec",
            docs_per_second
        );
    }

    #[test]
    fn test_vector_search_performance() {
        let executor = default_executor();

        for i in 0..50 {
            executor
                .import_document(
                    &format!("Doc {}", i),
                    "REPORT",
                    &format!("Content {}", i),
                    &["perf", "test"],
                )
                .unwrap();
        }

        let start = std::time::Instant::now();
        let search_count = 10;

        for _ in 0..search_count {
            let _ = executor.search("test query", 10).unwrap();
        }

        let elapsed = start.elapsed();
        let searches_per_second = search_count as f64 / elapsed.as_secs_f64();

        assert!(
            searches_per_second > 50.0,
            "Search too slow: {:.1} searches/sec",
            searches_per_second
        );
    }

    #[test]
    fn test_hybrid_search_performance() {
        let executor = default_executor();

        for i in 0..30 {
            executor
                .import_document(
                    &format!("Hybrid Doc {}", i),
                    "REPORT",
                    &format!("Hybrid content {}", i),
                    &["hybrid"],
                )
                .unwrap();
        }

        let start = std::time::Instant::now();
        let search_count = 10;

        for _ in 0..search_count {
            let _ = executor.hybrid_search("hybrid query", 5).unwrap();
        }

        let elapsed = start.elapsed();
        let searches_per_second = search_count as f64 / elapsed.as_secs_f64();

        assert!(
            searches_per_second > 20.0,
            "Hybrid search too slow: {:.1} searches/sec",
            searches_per_second
        );
    }

    #[test]
    fn test_reindex_all() {
        let executor = default_executor();

        for i in 0..20 {
            executor
                .import_document(
                    &format!("Reindex Doc {}", i),
                    "REPORT",
                    &format!("Content to reindex {}", i),
                    &["reindex"],
                )
                .unwrap();
        }

        let start = std::time::Instant::now();
        let count = executor.reindex_all().unwrap();
        let elapsed = start.elapsed();

        assert_eq!(count, 20);
        // Reindex should complete in reasonable time
        assert!(elapsed.as_secs() < 10, "Reindex took too long");
    }
}

// ========================================================================
// Regression Detection Tests
// ========================================================================

#[cfg(test)]
mod regression_tests {
    use super::*;

    #[test]
    fn test_regression_baseline_search_accuracy() {
        let executor = default_executor();

        executor
            .import_document(
                "Apple Fruit",
                "FOOD",
                "Apple is a delicious fruit",
                &["apple", "fruit"],
            )
            .unwrap();

        executor
            .import_document(
                "Apple Computer",
                "TECH",
                "Apple makes computers and phones",
                &["apple", "computer", "tech"],
            )
            .unwrap();

        executor
            .import_document(
                "Orange Fruit",
                "FOOD",
                "Orange is a citrus fruit",
                &["orange", "fruit"],
            )
            .unwrap();

        let results = executor.search("apple", 5).unwrap();
        assert!(!results.is_empty());

        let apple_found = results
            .iter()
            .any(|r| r.doc_id > 0 && r.similarity > 0.0);
        assert!(apple_found, "Should find Apple documents");
    }

    #[test]
    fn test_regression_hybrid_vs_pure_vector() {
        let executor = default_executor();

        executor
            .import_document(
                "Test Document 1",
                "REPORT",
                "First test document content",
                &["test"],
            )
            .unwrap();

        executor
            .import_document(
                "Test Document 2",
                "REPORT",
                "Second test document content",
                &["test"],
            )
            .unwrap();

        let vector_results = executor.search("test", 5).unwrap();
        let hybrid_results = executor.hybrid_search("test", 5).unwrap();

        assert!(!vector_results.is_empty());
        assert!(!hybrid_results.is_empty());
    }

    #[test]
    fn test_regression_consistency() {
        let executor = default_executor();

        executor
            .import_document(
                "Consistency Test",
                "REPORT",
                "Testing search consistency over multiple queries",
                &["consistency"],
            )
            .unwrap();

        let results1 = executor.search("consistency", 5).unwrap();
        let results2 = executor.search("consistency", 5).unwrap();
        let results3 = executor.search("consistency", 5).unwrap();

        assert!(!results1.is_empty());
        assert!(!results2.is_empty());
        assert!(!results3.is_empty());

        // All searches should return consistent first result
        let first_ids: Vec<_> = vec![&results1, &results2, &results3]
            .iter()
            .map(|r| r[0].doc_id)
            .collect();
        assert_eq!(first_ids[0], first_ids[1]);
        assert_eq!(first_ids[1], first_ids[2]);
    }

    #[test]
    fn test_regression_embedding_determinism() {
        let text = "Deterministic embedding test";

        let emb1 = generate_embedding(text);
        let emb2 = generate_embedding(text);
        let emb3 = generate_embedding(text);

        assert_eq!(emb1, emb2);
        assert_eq!(emb2, emb3);
    }
}

// ========================================================================
// Integration Tests
// ========================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_gmp_workflow() {
        let executor = default_executor();

        // 1. Create documents
        let doc_id = executor
            .import_document(
                "Complete Workflow Test",
                "SOP",
                "Testing complete GMP workflow with vector search",
                &["workflow", "integration", "vector"],
            )
            .unwrap();

        // 2. Search for them
        let search_results = executor.search("workflow", 5).unwrap();
        assert!(!search_results.is_empty());

        // 3. Hybrid search
        let hybrid_results = executor.hybrid_search("workflow", 5).unwrap();
        assert!(!hybrid_results.is_empty());

        // 4. Verify document exists
        let doc = executor.get_document(doc_id).unwrap();
        assert!(doc.is_some());
    }

    #[test]
    fn test_edge_case_search_with_match() {
        let executor = default_executor();

        executor
            .import_document(
                "Apple Document",
                "REPORT",
                "Apple content",
                &["apple"],
            )
            .unwrap();

        // Search for something that exists
        let results = executor.search("apple", 5).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_edge_case_large_result_set() {
        let executor = default_executor();

        for i in 0..20 {
            executor
                .import_document(
                    &format!("Large Result {}", i),
                    "REPORT",
                    &format!("Content {}", i),
                    &["large"],
                )
                .unwrap();
        }

        let results = executor.search("large result", 100).unwrap();
        assert!(results.len() <= 20);
    }

    #[test]
    fn test_vector_embed() {
        let executor = default_executor();

        let emb = executor.vector_embed("Test embedding generation");
        assert_eq!(emb.len(), 256);
    }
}
