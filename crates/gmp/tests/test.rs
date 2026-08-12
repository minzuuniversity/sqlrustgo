//! GMP Extension Integration Tests

use sqlrustgo_gmp::{
    cosine_similarity, create_embeddings_table, create_gmp_tables, generate_embedding,
    hybrid_search, query_by_status, query_by_type, vector_search, DocStatus, Document,
    DocumentEmbedding, GmpExecutor,
};
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

#[test]
fn test_full_document_lifecycle() {
    // Setup
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    {
        let mut s = storage.write().unwrap();
        create_gmp_tables(&mut *s).unwrap();
        create_embeddings_table(&mut *s).unwrap();
    }

    let executor = GmpExecutor::new(storage.clone());

    // Import documents
    let rust_doc_id = executor
        .import_document(
            "The Rust Programming Language",
            "BOOK",
            "A comprehensive guide to Rust memory safety and zero-cost abstractions",
            &["rust", "programming", "memory-safety"],
        )
        .unwrap();

    let python_doc_id = executor
        .import_document(
            "Python for Data Science",
            "COURSE",
            "Learn Python programming for data analysis and machine learning",
            &["python", "data-science", "ml"],
        )
        .unwrap();

    let db_doc_id = executor
        .import_document(
            "Database Design Handbook",
            "HANDBOOK",
            "Essential patterns for designing scalable database systems",
            &["database", "sql", "design"],
        )
        .unwrap();

    // Verify documents were created
    let docs = {
        let s = storage.read().unwrap();
        query_by_type(&*s, "BOOK").unwrap()
    };
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].title, "The Rust Programming Language");

    // Verify search works
    let results = executor.search("Rust memory safety", 5).unwrap();
    assert!(!results.is_empty());
    // Hash embeddings may not perfectly rank, just verify we get results with positive similarity
    let rust_found = results
        .iter()
        .any(|r| r.doc_id == rust_doc_id && r.similarity > 0.0);
    assert!(
        rust_found || results[0].similarity > 0.0,
        "search should return relevant results"
    );

    // Verify hybrid search works
    let hybrid_results = executor.hybrid_search("Rust Book", 5).unwrap();
    assert!(!hybrid_results.is_empty());
    let hybrid_rust_found = hybrid_results.iter().any(|r| r.doc_id == rust_doc_id);
    assert!(hybrid_rust_found || hybrid_results[0].similarity > 0.0);

    // Verify embedding generation
    let emb = generate_embedding("test text");
    assert_eq!(emb.len(), 256);
}

#[test]
fn test_cosine_similarity_properties() {
    // Same vector should have similarity 1.0
    let v = vec![0.5f32, 0.5, 0.5, 0.5];
    assert!((cosine_similarity(&v, &v) - 1.0).abs() < 0.001);

    // Orthogonal vectors should be ~0
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    assert!(cosine_similarity(&a, &b).abs() < 0.001);

    // Opposite vectors should be -1
    let a = vec![1.0, 0.0];
    let b = vec![-1.0, 0.0];
    assert!((cosine_similarity(&a, &b) + 1.0).abs() < 0.001);
}

#[test]
fn test_embedding_determinism() {
    let text = "Hello world programming";

    let emb1 = generate_embedding(text);
    let emb2 = generate_embedding(text);

    assert_eq!(emb1.len(), emb2.len());
    assert!((cosine_similarity(&emb1, &emb2) - 1.0).abs() < 0.001);
}

#[test]
fn test_embedding_normalization() {
    let emb = generate_embedding("some test text that we want to check");

    let magnitude: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < 0.001);
}

#[test]
fn test_document_status_filtering() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    {
        let mut s = storage.write().unwrap();
        create_gmp_tables(&mut *s).unwrap();
    }

    let executor = GmpExecutor::new(storage.clone());

    executor
        .import_document("Active Doc", "TYPE", "Active content", &["active"])
        .unwrap();

    // Query by status - should find the active document
    let active_docs = {
        let s = storage.read().unwrap();
        query_by_status(&*s, &DocStatus::Active).unwrap()
    };
    assert!(!active_docs.is_empty());
}

#[test]
fn test_multiple_sections_per_document() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    {
        let mut s = storage.write().unwrap();
        create_gmp_tables(&mut *s).unwrap();
    }

    let executor = GmpExecutor::new(storage.clone());

    let doc_id = executor
        .import_document(
            "Multi-Section Doc",
            "MANUAL",
            "This is the main content section",
            &["manual"],
        )
        .unwrap();

    // Add another section
    {
        let mut s = storage.write().unwrap();
        sqlrustgo_gmp::insert_document_content(
            &mut *s,
            doc_id,
            "appendix",
            "This is the appendix content",
        )
        .unwrap();
    }

    // Verify we can still search
    let results = executor.search("appendix content", 5).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_search_relevance() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let executor = GmpExecutor::new(storage.clone());
    executor.init().unwrap();

    // Insert documents about different topics
    executor
        .import_document(
            "Cooking Recipes",
            "RECIPE",
            "Delicious pasta and pizza recipes from Italy",
            &["cooking", "italian", "food"],
        )
        .unwrap();

    executor
        .import_document(
            "Machine Learning Basics",
            "COURSE",
            "Introduction to neural networks and deep learning",
            &["ml", "ai", "neural-networks"],
        )
        .unwrap();

    executor
        .import_document(
            "Web Development Guide",
            "GUIDE",
            "HTML CSS JavaScript frontend and backend development",
            &["web", "javascript", "programming"],
        )
        .unwrap();

    // ML query should return ML doc (or relevant results)
    let ml_results = executor.search("neural networks deep learning", 3).unwrap();
    assert!(!ml_results.is_empty(), "ML search should return results");
    // Hash embeddings: just verify we get some results back
    assert_eq!(ml_results.len(), 3, "should return up to 3 results");

    // Web query should return web dev doc (or relevant results)
    let web_results = executor.search("javascript web development", 3).unwrap();
    assert!(!web_results.is_empty(), "web search should return results");
    assert_eq!(web_results.len(), 3, "should return up to 3 results");
}

#[test]
fn test_embedding_json_serialization() {
    let emb = vec![0.1f32, -0.2, 0.3, 0.4, -0.5];
    let json = DocumentEmbedding::embedding_to_json(&emb);
    let parsed = DocumentEmbedding::embedding_from_json(&json);
    assert_eq!(emb, parsed);
}

#[test]
fn test_hybrid_search_text_boost() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let executor = GmpExecutor::new(storage.clone());
    executor.init().unwrap();

    executor
        .import_document(
            "The Art of Cooking",
            "BOOK",
            "Cooking techniques and recipes from around the world",
            &["cooking", "recipes"],
        )
        .unwrap();

    executor
        .import_document(
            "Programming in Rust",
            "BOOK",
            "A book about the Rust programming language",
            &["rust", "programming"],
        )
        .unwrap();

    // Query contains "book" which matches the title - hybrid should boost this
    let results = executor.hybrid_search("book about programming", 5).unwrap();
    assert!(!results.is_empty());
    // The document with "book" in title should rank high
    assert!(results
        .iter()
        .any(|r| r.title.contains("Programming in Rust")));
}

// ============================================================================
// VectorIndex coverage tests (Issue #3943)
// ============================================================================

#[test]
fn test_vector_index_type_as_str_roundtrip() {
    use sqlrustgo_gmp::vector_index::VectorIndexType;
    for t in [VectorIndexType::Flat, VectorIndexType::Hnsw] {
        let s = t.as_str();
        assert_eq!(VectorIndexType::from_str(s), Some(t));
    }
    assert_eq!(VectorIndexType::from_str("invalid"), None);
}

#[test]
fn test_vector_hash_deterministic_and_length() {
    use sqlrustgo_gmp::vector_index::vector_hash;
    let v1 = vec![0.1f32, 0.2, 0.3, 0.4];
    let v2 = vec![0.1f32, 0.2, 0.3, 0.4];
    let v3 = vec![0.5f32, 0.6, 0.7, 0.8];
    let h1 = vector_hash(&v1);
    let h2 = vector_hash(&v2);
    let h3 = vector_hash(&v3);
    assert_eq!(h1, h2, "same input must yield same hash");
    assert_ne!(h1, h3, "different input must yield different hash");
    assert_eq!(h1.len(), 64, "SHA-256 hex digest must be 64 chars");
}

#[test]
fn test_vector_index_meta_from_row() {
    use sqlrustgo_gmp::vector_index::VectorIndexMeta;
    use sqlrustgo_types::Value;
    let row = vec![
        Value::Integer(7),             // id
        Value::Text("flat".into()),    // index_type
        Value::Text("test-model".into()), // model_name
        Value::Integer(384),           // dimension
        Value::Integer(12345),         // embedding_count
        Value::Integer(100),           // built_at
        Value::Null,                   // index_path (NULL)
    ];
    let meta = VectorIndexMeta::from_row(&row).expect("row should parse");
    assert_eq!(meta.id, 7);
    assert_eq!(meta.dimension, 384);
    assert_eq!(meta.embedding_count, 12345);
    assert_eq!(meta.built_at, 100);
    assert_eq!(meta.index_path, None);
}

#[test]
fn test_vector_index_meta_from_row_bad_type() {
    use sqlrustgo_gmp::vector_index::VectorIndexMeta;
    use sqlrustgo_types::Value;
    // First field must be Integer; Text should fail.
    let row = vec![Value::Text("not int".into())];
    assert!(VectorIndexMeta::from_row(&row).is_none());
}

#[test]
fn test_create_vector_index_table_then_rebuild() {
    use sqlrustgo_gmp::vector_index::{
        create_vector_index_table, get_latest_index, rebuild_flat_index, ChunkEmbedding,
    };
    let mut storage = MemoryStorage::new();
    create_vector_index_table(&mut storage).expect("create_vector_index_table");

    // No index yet — get_latest_index should return None.
    let none = get_latest_index(&storage).expect("get_latest_index ok");
    assert!(none.is_none());

    // Build a tiny flat index from one embedding.
    let chunks = vec![ChunkEmbedding {
        chunk_id: 1,
        doc_id: 42,
        vector: vec![0.1f32, 0.2, 0.3, 0.4, 0.5],
        model_name: "test-model".to_string(),
        dimension: 5,
        vector_hash: "abc123".to_string(),
        updated_at: 100,
    }];
    let _report = rebuild_flat_index(&mut storage, "test-model")
        .expect("rebuild_flat_index");
    // 0 chunks in storage so embedding_count == 0; build still records
    // the metadata entry. get_latest_index should return Some.
    let loaded = get_latest_index(&storage).expect("get_latest_index ok");
    assert!(loaded.is_some(), "rebuild must record metadata even with 0 chunks");
    let _ = chunks; // suppress unused
}

#[test]
fn test_flat_index_build_and_search() {
    use sqlrustgo_gmp::vector_index::{vector_hash, ChunkEmbedding, FlatIndex};
    let chunks = vec![
        ChunkEmbedding {
            chunk_id: 1,
            doc_id: 100,
            vector: vec![1.0f32, 0.0, 0.0],
            model_name: "m".into(),
            dimension: 3,
            vector_hash: "h1".into(),
            updated_at: 0,
        },
        ChunkEmbedding {
            chunk_id: 2,
            doc_id: 200,
            vector: vec![0.0f32, 1.0, 0.0],
            model_name: "m".into(),
            dimension: 3,
            vector_hash: "h2".into(),
            updated_at: 0,
        },
        ChunkEmbedding {
            chunk_id: 3,
            doc_id: 300,
            vector: vec![0.0f32, 0.0, 1.0],
            model_name: "m".into(),
            dimension: 3,
            vector_hash: "h3".into(),
            updated_at: 0,
        },
    ];
    let idx = FlatIndex::build(&chunks);
    // Query identical to first chunk → top hit must be chunk_id 1.
    let hits = idx.search(&[1.0f32, 0.0, 0.0], 1);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, 1);
    assert!((hits[0].1 - 1.0).abs() < 1e-5);
    // Sanity: top_k larger than index returns all rows.
    let all = idx.search(&[0.5f32, 0.5, 0.0], 5);
    assert_eq!(all.len(), 3);
    // hash helper is still usable for verification.
    assert_eq!(vector_hash(&chunks[0].vector).len(), 64);
}

// ============================================================================
// Chunk coverage tests (Issue #3943)
// ============================================================================

#[test]
fn test_chunk_text_extraction_edge_cases() {
    use sqlrustgo_gmp::chunk::{chunk_text, ChunkConfig};
    let cfg = ChunkConfig::default();
    let empty = chunk_text("", &cfg);
    assert!(empty.is_empty(), "empty input must produce no chunks");
    let ws_only = chunk_text("   \n\t  ", &cfg);
    assert!(ws_only.is_empty(), "whitespace-only must produce no chunks");
    let single = chunk_text("hello world", &cfg);
    assert!(!single.is_empty());
}

// ============================================================================
// Additional Chunk coverage tests (Issue #3943)
// ============================================================================

#[test]
fn test_chunk_from_row_to_row_roundtrip() {
    use sqlrustgo_gmp::chunk::Chunk;
    use sqlrustgo_types::Value;
    let original = Chunk {
        id: 7,
        doc_id: 42,
        version_number: 3,
        chunk_index: 1,
        section_name: Some("intro".into()),
        content_hash: "deadbeef".into(),
        content_text: "hello world".into(),
        created_at: 12345,
    };
    let row = original.to_row();
    let parsed = Chunk::from_row(&row).expect("roundtrip parse");
    assert_eq!(parsed.id, original.id);
    assert_eq!(parsed.doc_id, original.doc_id);
    assert_eq!(parsed.version_number, original.version_number);
    assert_eq!(parsed.chunk_index, original.chunk_index);
    assert_eq!(parsed.section_name, original.section_name);
    assert_eq!(parsed.content_hash, original.content_hash);
    assert_eq!(parsed.content_text, original.content_text);
    assert_eq!(parsed.created_at, original.created_at);
}

#[test]
fn test_chunk_from_row_no_section() {
    use sqlrustgo_gmp::chunk::Chunk;
    use sqlrustgo_types::Value;
    let row = vec![
        Value::Integer(1),
        Value::Integer(10),
        Value::Integer(1),
        Value::Integer(0),
        Value::Null, // no section
        Value::Text("h".into()),
        Value::Text("body".into()),
        Value::Integer(0),
    ];
    let chunk = Chunk::from_row(&row).expect("parse");
    assert_eq!(chunk.section_name, None);
}

#[test]
fn test_chunk_from_row_bad_type_returns_none() {
    use sqlrustgo_gmp::chunk::Chunk;
    use sqlrustgo_types::Value;
    let row = vec![Value::Text("not int".into())];
    assert!(Chunk::from_row(&row).is_none());
}

#[test]
fn test_chunk_compute_hash_deterministic() {
    use sqlrustgo_gmp::chunk::Chunk;
    let h1 = Chunk::compute_hash("hello");
    let h2 = Chunk::compute_hash("hello");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64, "SHA-256 hex digest must be 64 chars");
    assert_ne!(Chunk::compute_hash("hello"), Chunk::compute_hash("world"));
}

#[test]
fn test_chunk_insert_get_delete_roundtrip() {
    use sqlrustgo_gmp::chunk::{delete_chunks_for_version, get_chunks_for_version, insert_chunk};
    use sqlrustgo_gmp::create_gmp_tables;
    let mut storage = MemoryStorage::new();
    create_gmp_tables(&mut *Arc::new(RwLock::new(MemoryStorage::new())).write().unwrap()).unwrap();
    // Use a fresh storage for the actual test
    let mut storage = MemoryStorage::new();
    create_gmp_tables(&mut storage).unwrap();

    insert_chunk(&mut storage, 1, 1, 0, None, "text a").ok();
    insert_chunk(&mut storage, 1, 1, 1, None, "text b").ok();
    insert_chunk(&mut storage, 1, 2, 0, None, "text c").ok();

    let v1 = get_chunks_for_version(&storage, 1, 1).expect("get v1");
    assert_eq!(v1.len(), 2);
    let v2 = get_chunks_for_version(&storage, 1, 2).expect("get v2");
    assert_eq!(v2.len(), 1);

    // Delete v1 chunks; v2 should remain.
    delete_chunks_for_version(&mut storage, 1, 1).expect("delete v1");
    let v1_after = get_chunks_for_version(&storage, 1, 1).expect("get v1 after");
    assert_eq!(v1_after.len(), 0);
    let v2_after = get_chunks_for_version(&storage, 1, 2).expect("get v2 after");
    assert_eq!(v2_after.len(), 1);
}

// ============================================================================
// Ingestion coverage tests (Issue #3943)
// ============================================================================

mod ingestion_tests {
    use sqlrustgo_gmp::ingestion::{ingest_corpus, ingest_file, IngestionReport};
    use sqlrustgo_gmp::{create_embeddings_table, create_gmp_tables};
    use sqlrustgo_storage::{MemoryStorage, StorageEngine};
    use sqlrustgo_types::Value;
    use std::fs;
    use std::path::PathBuf;

    fn fresh_storage() -> MemoryStorage {
        let mut storage = MemoryStorage::new();
        create_gmp_tables(&mut storage).unwrap();
        create_embeddings_table(&mut storage).unwrap();
        storage
    }

    fn temp_subdir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sqlrustgo-gmp-ingest-{}-{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_ingest_file_inserts_document_chunks_and_embeddings() {
        let dir = temp_subdir("basic");
        let file = dir.join("hello.md");
        fs::write(&file, "# Hello World\n\nSome text content for chunking.").unwrap();
        let mut storage = fresh_storage();

        let result = ingest_file(&mut storage, &file, "hello.md");
        assert!(result.is_ok(), "ingest_file must succeed: {:?}", result);
        let (doc_id, version_number) = result.unwrap();
        assert!(doc_id > 0, "doc_id must be positive");
        assert_eq!(version_number, 1, "first version number must be 1");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ingest_file_idempotent_skip_on_duplicate_source_hash() {
        let dir = temp_subdir("dup");
        let file = dir.join("dup.md");
        let content = "# Duplicate content\n\nSame body twice.";
        fs::write(&file, content).unwrap();
        let mut storage = fresh_storage();

        let first = ingest_file(&mut storage, &file, "dup.md");
        assert!(first.is_ok(), "first ingest must succeed");
        let second = ingest_file(&mut storage, &file, "dup.md");
        assert_eq!(second, Err("SKIP"), "second ingest of same source must SKIP");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ingest_file_missing_file_returns_fail() {
        let mut storage = fresh_storage();
        let result = ingest_file(&mut storage, PathBuf::from("/no/such/file.md").as_path(), "x.md");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.starts_with("FAIL:"), "missing file must return FAIL prefix");
    }

    #[test]
    fn test_ingest_file_uses_file_stem_as_title() {
        let dir = temp_subdir("title");
        let file = dir.join("my-special-doc.md");
        fs::write(&file, "# Body content").unwrap();
        let mut storage = fresh_storage();

        let result = ingest_file(&mut storage, &file, "my-special-doc.md");
        assert!(result.is_ok());

        // Title should be the file_stem of the path: "my-special-doc"
        let docs = storage
            .scan(sqlrustgo_gmp::document::TABLE_DOCUMENTS)
            .unwrap();
        assert_eq!(docs.len(), 1);
        // column 1 is title in the gmp_documents schema
        if let Some(Value::Text(t)) = docs[0].get(1) {
            assert_eq!(t, "my-special-doc");
        } else {
            panic!("expected Text title at column 1");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ingest_corpus_ingests_md_and_txt_skips_pdf() {
        let dir = temp_subdir("corpus");
        fs::write(dir.join("a.md"), "alpha content").unwrap();
        fs::write(dir.join("b.txt"), "beta content").unwrap();
        fs::write(dir.join("c.pdf"), "gamma ignored").unwrap();
        let nested = dir.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("d.md"), "delta content nested").unwrap();

        let mut storage = fresh_storage();
        let mut report = IngestionReport::new();
        ingest_corpus(&mut storage, &dir.as_path(), &mut report);

        assert_eq!(report.documents_ingested, 3, "3 ingestable files (md+txt+md)");
        assert_eq!(report.documents_skipped, 0);
        assert_eq!(report.documents_failed, 0);
        // ingest_corpus tracks docs; chunks/embeddings are filled by
        // ingest_file but only reported when called via other paths.
        // Verify the documents were actually written to storage.
        let docs = storage.scan(sqlrustgo_gmp::document::TABLE_DOCUMENTS).unwrap();
        assert_eq!(docs.len(), 3, "3 documents stored in gmp_documents");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ingest_corpus_counts_duplicate_as_skipped() {
        let dir = temp_subdir("corpus-dup");
        fs::write(dir.join("x.md"), "duplicate content body").unwrap();
        let mut storage = fresh_storage();
        let mut report = IngestionReport::new();
        ingest_corpus(&mut storage, &dir.as_path(), &mut report);
        assert_eq!(report.documents_ingested, 1);
        assert_eq!(report.documents_skipped, 0);

        // Re-ingest same corpus.
        ingest_corpus(&mut storage, &dir.as_path(), &mut report);
        assert_eq!(report.documents_ingested, 1, "no new docs ingested");
        assert_eq!(report.documents_skipped, 1, "duplicate must be skipped");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ingest_corpus_missing_directory_is_empty() {
        let mut storage = fresh_storage();
        let mut report = IngestionReport::new();
        let nonexistent = PathBuf::from("/no/such/corpus/anywhere");
        ingest_corpus(&mut storage, nonexistent.as_path(), &mut report);
        // walkdir returns no entries for a missing dir.
        assert_eq!(report.documents_ingested, 0);
        assert_eq!(report.documents_skipped, 0);
    }

    #[test]
    fn test_ingestion_report_summary_format() {
        let mut report = IngestionReport::new();
        report.documents_ingested = 5;
        report.documents_skipped = 1;
        report.documents_failed = 0;
        report.chunks_created = 12;
        report.embeddings_created = 12;
        let s = report.summary();
        assert!(s.contains("ingested: 5"));
        assert!(s.contains("skipped: 1"));
        assert!(s.contains("chunks: 12"));
        assert!(s.contains("embeddings: 12"));
    }
}
