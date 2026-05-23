//! Unit tests for gmp-storage

use std::fs;
use tempfile::TempDir;

use crate::storage::{
    cosine_sim, IndexStats, SearchResult, StorageConfig, StorageEngine, ValidationResult,
    VectorSearchResult,
};

fn create_test_engine(tmpdir: &TempDir) -> StorageEngine {
    let config = StorageConfig {
        data_dir: tmpdir.path().to_string_lossy().to_string(),
        db_path: tmpdir.path().join("test.db").to_string_lossy().to_string(),
        embedding_dim: 768,
        use_sqlrustgo_storage: true,
    };
    StorageEngine::new(config).expect("Failed to create storage engine")
}

fn create_test_markdown(tmpdir: &TempDir, filename: &str, content: &str) -> std::path::PathBuf {
    let path = tmpdir.path().join(filename);
    fs::write(&path, content).expect("Failed to write test file");
    path
}

// =============================================================================
// StorageConfig tests
// =============================================================================

#[test]
fn test_storage_config_default() {
    let config = StorageConfig::default();
    assert_eq!(config.data_dir, "./data");
    assert_eq!(config.db_path, "./data/gmp.db");
}

#[test]
fn test_storage_config_custom() {
    let config = StorageConfig {
        data_dir: "/tmp/custom_data".to_string(),
        db_path: "/tmp/custom_data/custom.db".to_string(),
        ..Default::default()
    };
    assert_eq!(config.data_dir, "/tmp/custom_data");
    assert_eq!(config.db_path, "/tmp/custom_data/custom.db");
}

#[test]
fn test_storage_config_clone() {
    let config = StorageConfig::default();
    let cloned = config.clone();
    assert_eq!(config.data_dir, cloned.data_dir);
    assert_eq!(config.db_path, cloned.db_path);
}

// =============================================================================
// StorageEngine new/init_schema tests
// =============================================================================

#[test]
fn test_storage_engine_new_creates_db() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let db_path = tmpdir.path().join("test.db");
    let config = StorageConfig {
        data_dir: tmpdir.path().to_string_lossy().to_string(),
        db_path: db_path.to_string_lossy().to_string(),
        ..Default::default()
    };
    let engine = StorageEngine::new(config).expect("Failed to create engine");
    // FileStorage stores data in data_dir/gmp_data (a directory), not db_path
    let storage_path = tmpdir.path().join("gmp_data");
    assert!(storage_path.exists(), "FileStorage should create data_dir/gmp_data");
    let _ = engine; // suppress unused warning
}

#[test]
fn test_storage_engine_new_creates_parent_dirs() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let nested_path = tmpdir.path().join("nested").join("deep");
    let db_path = nested_path.join("test.db");
    let config = StorageConfig {
        data_dir: nested_path.to_string_lossy().to_string(),
        db_path: db_path.to_string_lossy().to_string(),
        ..Default::default()
    };
    let engine = StorageEngine::new(config).expect("Failed to create engine with nested path");
    assert!(nested_path.exists());
    let _ = engine;
}

#[test]
fn test_storage_engine_init_schema_creates_tables() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let engine = create_test_engine(&tmpdir);

    // Verify by inserting and querying
    let count = engine
        .get_document_count()
        .expect("get_document_count failed");
    assert_eq!(count, 0);
}

// =============================================================================
// index_documents tests
// =============================================================================

#[test]
fn test_index_documents_nonexistent_dir() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let engine = create_test_engine(&tmpdir);
    let result = engine.index_documents("/nonexistent/path");
    assert!(result.is_err());
}

#[test]
fn test_index_documents_empty_dir() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let empty_dir = tmpdir.path().join("empty");
    fs::create_dir(&empty_dir).expect("Failed to create empty dir");

    let engine = create_test_engine(&tmpdir);
    let stats = engine
        .index_documents(empty_dir.to_str().unwrap())
        .expect("index_documents failed");
    assert_eq!(stats.documents_imported, 0);
    assert!(stats.errors.is_empty());
}

#[test]
fn test_index_documents_single_markdown() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    let content = r#"---
code: DOC001
title: Test Document
document_type: policy
department: QA
version: "1.0"
effective_date: "2024-01-01"
---

# Introduction

This is the introduction section with enough content to be chunked properly by the system.

## Details

This is the details section with additional content that ensures proper chunking occurs during the indexing process.
"#;
    create_test_markdown(&tmpdir, "docs/test.md", content);

    let engine = create_test_engine(&tmpdir);
    let stats = engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    assert_eq!(stats.documents_imported, 1);
    assert!(stats.errors.is_empty());
}

#[test]
fn test_index_documents_multiple_markdowns() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    create_test_markdown(
        &tmpdir,
        "docs/doc1.md",
        r#"---
code: DOC001
title: Document One
---

# Content One
Some content here with enough text to be properly chunked.
"#,
    );

    create_test_markdown(
        &tmpdir,
        "docs/doc2.md",
        r#"---
code: DOC002
title: Document Two
---

# Content Two
Different content with sufficient length.
"#,
    );

    let engine = create_test_engine(&tmpdir);
    let stats = engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    assert_eq!(stats.documents_imported, 2);
    assert!(stats.errors.is_empty());
}

#[test]
fn test_index_documents_no_frontmatter() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    // Markdown without frontmatter - uses filename as doc_id
    create_test_markdown(
        &tmpdir,
        "docs/note.md",
        "# Simple Note\n\nJust some text with enough content to be chunked.",
    );

    let engine = create_test_engine(&tmpdir);
    let stats = engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    assert_eq!(stats.documents_imported, 1);
    assert!(stats.errors.is_empty());
}

// =============================================================================
// search_fts tests
// =============================================================================

#[test]
fn test_search_fts_empty_index() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let engine = create_test_engine(&tmpdir);

    let results = engine.search_fts("test", 10).expect("search_fts failed");
    assert!(results.is_empty());
}

#[test]
fn test_search_fts_with_results() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    // Use longer content that will actually produce chunks
    let content = r#"---
code: FTS001
title: FTS Test
---

# Introduction to Quality Assurance Testing

This document provides comprehensive information about testing procedures in the quality assurance department. It covers various aspects of software testing methodology and best practices.

## Unit Testing Overview

Unit testing is a fundamental practice in software development that involves testing individual components or functions in isolation to ensure they work correctly.

## Integration Testing

Integration testing combines individual software modules and tests them as a group to verify that they work together as expected.

## System Testing

System testing evaluates the complete and integrated software system to ensure it meets specified requirements.
"#;
    create_test_markdown(&tmpdir, "docs/fts.md", content);

    let engine = create_test_engine(&tmpdir);
    engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    let results = engine.search_fts("testing", 10).expect("search_fts failed");
    assert!(!results.is_empty());
}

#[test]
fn test_search_fts_no_match() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    create_test_markdown(
        &tmpdir,
        "docs/test.md",
        r#"---
code: TEST001
title: Test
---

# Hello World
Content about something specific with enough length for proper chunking.
"#,
    );

    let engine = create_test_engine(&tmpdir);
    engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    let results = engine
        .search_fts("xyznonexistent", 10)
        .expect("search_fts failed");
    assert!(results.is_empty());
}

#[test]
fn test_search_fts_limit() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    // Create multiple docs with longer content
    for i in 0..5 {
        create_test_markdown(
            &tmpdir,
            &format!("docs/doc{i}.md"),
            &format!(
                r#"---
code: DOC{i}
title: Document {i}
---

# Testing Section One

This is the first section of document {i} with enough content to be chunked properly during the indexing process.

# Testing Section Two

This is the second section with additional content that ensures proper chunking and search functionality.
"#
            ),
        );
    }

    let engine = create_test_engine(&tmpdir);
    engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    let results = engine.search_fts("section", 2).expect("search_fts failed");
    assert_eq!(results.len(), 2);
}

// =============================================================================
// get_document_count tests
// =============================================================================

#[test]
fn test_get_document_count_empty() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let engine = create_test_engine(&tmpdir);

    let count = engine
        .get_document_count()
        .expect("get_document_count failed");
    assert_eq!(count, 0);
}

#[test]
fn test_get_document_count_after_indexing() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    for i in 0..3 {
        create_test_markdown(
            &tmpdir,
            &format!("docs/doc{i}.md"),
            &format!(
                r#"---
code: DOC{i}
title: Document {i}
---

# Content {i}
Sufficient content for proper chunking.
"#
            ),
        );
    }

    let engine = create_test_engine(&tmpdir);
    engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    let count = engine
        .get_document_count()
        .expect("get_document_count failed");
    assert_eq!(count, 3);
}

// =============================================================================
// search_vectors tests
// =============================================================================

#[test]
fn test_search_vectors_empty_index() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let engine = create_test_engine(&tmpdir);

    // Query with a random embedding
    let query = vec![0.1f32; 128];
    let results = engine
        .search_vectors(&query, 10)
        .expect("search_vectors failed");
    assert!(results.is_empty());
}

// =============================================================================
// cosine_sim tests
// =============================================================================

#[test]
fn test_cosine_sim_identical_vectors() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![1.0f32, 0.0, 0.0];
    let sim = cosine_sim(&a, &b);
    assert!((sim - 1.0).abs() < 1e-6);
}

#[test]
fn test_cosine_sim_orthogonal_vectors() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.0f32, 1.0, 0.0];
    let sim = cosine_sim(&a, &b);
    assert!((sim - 0.0).abs() < 1e-6);
}

#[test]
fn test_cosine_sim_opposite_vectors() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![-1.0f32, 0.0, 0.0];
    let sim = cosine_sim(&a, &b);
    assert!((sim - (-1.0)).abs() < 1e-6);
}

#[test]
fn test_cosine_sim_zero_vector() {
    let a = vec![0.0f32, 0.0, 0.0];
    let b = vec![1.0f32, 0.0, 0.0];
    let sim = cosine_sim(&a, &b);
    assert_eq!(sim, 0.0);
}

#[test]
fn test_cosine_sim_zero_both() {
    let a = vec![0.0f32, 0.0, 0.0];
    let b = vec![0.0f32, 0.0, 0.0];
    let sim = cosine_sim(&a, &b);
    assert_eq!(sim, 0.0);
}

#[test]
fn test_cosine_sim_mismatched_length() {
    let a = vec![1.0f32, 0.0];
    let b = vec![1.0f32, 0.0, 0.0];
    let sim = cosine_sim(&a, &b);
    assert_eq!(sim, 0.0);
}

#[test]
fn test_cosine_sim_normalized() {
    // Unit vectors should give dot product as cosine similarity
    let a = vec![0.57735027f32, 0.57735027, 0.57735027]; // 1/sqrt(3) each
    let b = vec![0.70710678f32, 0.70710678, 0.0]; // 1/sqrt(2) each
    let sim = cosine_sim(&a, &b);
    // dot = 0.577*0.707 + 0.577*0.707 + 0 = 0.816
    assert!((sim - 0.816).abs() < 0.01);
}

// =============================================================================
// ValidationResult tests
// =============================================================================

#[test]
fn test_validation_result_is_valid() {
    let result = ValidationResult {
        is_valid: true,
        tables_present: true,
        chunks_table_present: true,
        documents: 2000,
        chunks: 100,
        embedding_count: 50,
        fts_token_count: 5000,
        errors: vec![],
        warnings: vec![],
    };
    assert!(result.is_valid);
    assert_eq!(result.documents, 2000);
    assert_eq!(result.chunks, 100);
}

#[test]
fn test_validation_result_is_invalid() {
    let result = ValidationResult {
        is_valid: false,
        tables_present: true,
        chunks_table_present: true,
        documents: 100,
        chunks: 50,
        embedding_count: 0,
        fts_token_count: 0,
        errors: vec!["Missing index".to_string()],
        warnings: vec![],
    };
    assert!(!result.is_valid);
}

#[test]
fn test_validation_result_debug() {
    let result = ValidationResult {
        is_valid: true,
        tables_present: true,
        chunks_table_present: true,
        documents: 1600,
        chunks: 10,
        embedding_count: 5,
        fts_token_count: 100,
        errors: vec![],
        warnings: vec![],
    };
    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("ValidationResult"));
    assert!(debug_str.contains("1600"));
    assert!(debug_str.contains("10"));
}

// =============================================================================
// IndexStats tests
// =============================================================================

#[test]
fn test_index_stats_default() {
    let stats = IndexStats::default();
    assert_eq!(stats.documents_imported, 0);
    assert_eq!(stats.chunks_indexed, 0);
    assert!(stats.errors.is_empty());
}

#[test]
fn test_index_stats_with_errors() {
    let stats = IndexStats {
        documents_imported: 5,
        documents_skipped: 2,
        chunks_indexed: 20,
        errors: vec!["error1".to_string(), "error2".to_string()],
    };
    assert_eq!(stats.documents_imported, 5);
    assert_eq!(stats.chunks_indexed, 20);
    assert_eq!(stats.errors.len(), 2);
}

// =============================================================================
// SearchResult and VectorSearchResult tests
// =============================================================================

#[test]
fn test_search_result_clone() {
    let result = SearchResult {
        chunk_id: 1,
        doc_id: "DOC001".to_string(),
        content: "test content".to_string(),
        token_count: 10,
        title: "Test".to_string(),
        doc_type: Some("policy".to_string()),
        chapter: Some("intro".to_string()),
        score: 0.95,
    };
    let cloned = result.clone();
    assert_eq!(result.chunk_id, cloned.chunk_id);
    assert_eq!(result.doc_id, cloned.doc_id);
    assert_eq!(result.score, cloned.score);
}

#[test]
fn test_vector_search_result_clone() {
    let result = VectorSearchResult {
        chunk_id: 1,
        doc_id: "DOC001".to_string(),
        chunk_index: 0,
        content: "test".to_string(),
        score: 0.88,
    };
    let cloned = result.clone();
    assert_eq!(result.chunk_id, cloned.chunk_id);
    assert_eq!(result.score, cloned.score);
}

// =============================================================================
// validate tests
// =============================================================================

#[test]
fn test_validate_empty_index() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let engine = create_test_engine(&tmpdir);

    let result = engine.validate().expect("validate failed");
    assert!(!result.is_valid);
    assert_eq!(result.documents, 0);
    assert_eq!(result.chunks, 0);
}

#[test]
fn test_validate_with_documents() {
    let tmpdir = TempDir::new().expect("Failed to create temp dir");
    let docs_dir = tmpdir.path().join("docs");
    fs::create_dir(&docs_dir).expect("Failed to create docs dir");

    // Create 1600+ documents to pass validation (1600+ documents AND chunks > 0)
    for i in 0..1605 {
        create_test_markdown(
            &tmpdir,
            &format!("docs/doc{i}.md"),
            &format!(
                r#"---
code: DOC{i}
title: Document {i}
---

# Content {i}
Sufficient content for proper chunking.
"#
            ),
        );
    }

    let engine = create_test_engine(&tmpdir);
    engine
        .index_documents(docs_dir.to_str().unwrap())
        .expect("index_documents failed");

    let result = engine.validate().expect("validate failed");
    assert!(result.is_valid);
    assert_eq!(result.documents, 1605);
}
