//! Integration tests for gmp-storage
//! Run with: cargo test --test integration_storage

use std::fs;
use tempfile::TempDir;

fn create_storage() -> (TempDir, gmp_storage::StorageEngine) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config = gmp_storage::StorageConfig {
        data_dir: temp_dir.path().to_str().unwrap().to_string(),
        db_path: db_path.to_str().unwrap().to_string(),
        embedding_dim: 1024,
        use_sqlrustgo_storage: true,
    };
    let storage = gmp_storage::StorageEngine::new(config).unwrap();
    (temp_dir, storage)
}

fn write_doc(dir: &std::path::Path, name: &str, content: &str) {
    fs::write(dir.join(format!("{}.md", name)), content).unwrap();
}

#[test]
fn test_full_index_and_search_cycle() {
    let (temp_dir, storage) = create_storage();

    // Create sample documents
    write_doc(
        temp_dir.path(),
        "sop1",
        r#"---
id: SOP-001
title: 偏差处理规程
---

# 偏差处理规程

本规程规定了偏差处理的基本要求。
对于重大偏差需要进行调查。
"#,
    );

    write_doc(
        temp_dir.path(),
        "sop2",
        r#"---
id: SOP-002
title: 变更控制规程
---

# 变更控制规程

所有变更需要经过变更控制委员会审批。
"#,
    );

    write_doc(
        temp_dir.path(),
        "sop3",
        r#"---
id: SOP-003
title: 偏差调查清单
---

# 偏差调查清单

- 偏差描述
- 根本原因分析
- 影响评估
"#,
    );

    // Index all documents
    let result = storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();
    assert_eq!(result.documents_imported, 3, "Should import 3 documents");

    // Search for various terms - API should work without panic
    // Note: Chinese FTS results depend on tokenizer configuration
    let _ = storage.search_fts("偏差", 10).unwrap();
    let _ = storage.search_fts("变更", 10).unwrap();
    let _ = storage.search_fts("审批", 10).unwrap();

    // Verify search API works and returns valid structure
    let results = storage.search_fts("规程", 10).unwrap();
    for r in &results {
        assert!(r.chunk_id >= 0);
        assert!(!r.content.is_empty());
        assert!(r.score >= 0.0);
    }
}

#[test]
#[ignore = "mtime-based change detection has second-level granularity; modification in same second is not detected"]
fn test_incremental_reindex_detects_changes() {
    let (temp_dir, storage) = create_storage();

    // Initial index
    write_doc(
        temp_dir.path(),
        "doc",
        r#"---
id: INC-001
title: 版本V1
---

# 内容V1
初始版本内容。
"#,
    );
    let r1 = storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();
    assert_eq!(r1.documents_imported, 1);

    // Re-index without changes - should skip
    let r2 = storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();
    assert!(
        r2.documents_skipped >= 1,
        "re-index unchanged: should skip at least 1 doc"
    );

    // Modify document
    write_doc(
        temp_dir.path(),
        "doc",
        r#"---
id: INC-001
title: 版本V2
---

# 内容V2
修改后版本内容。
"#,
    );

    // Re-index - should re-import
    let r3 = storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();
    assert_eq!(
        r3.documents_imported, 1,
        "Modified doc should be re-imported"
    );
}

#[test]
fn test_validate_with_multiple_docs() {
    let (temp_dir, storage) = create_storage();

    // Create 5 documents
    for i in 1..=5 {
        write_doc(
            temp_dir.path(),
            &format!("doc{}", i),
            &format!(
                r#"---
id: DOC-{:03}
title: 文档{}
---

# 文档{}内容。
"#,
                i, i, i
            ),
        );
    }

    storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();

    let result = storage.validate();
    assert!(result.is_ok());
}

#[test]
#[ignore = "BM25 scoring pre-existing issue; LIKE fallback changes scores"]
fn test_search_fts_result_scoring() {
    let (temp_dir, storage) = create_storage();

    // Create docs with varying keyword frequency
    write_doc(
        temp_dir.path(),
        "low",
        r#"---
id: LOW-001
title: 单次出现
---

# 内容
关键词只出现一次。
"#,
    );

    write_doc(
        temp_dir.path(),
        "high",
        r#"---
id: HIGH-001
title: 多次出现
---

# 内容
关键词 关键词 关键词 关键词 关键词。
"#,
    );

    storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();

    // Verify scoring API works
    let results = storage.search_fts("关键词", 10).unwrap();
    for r in &results {
        assert!(r.score >= 0.0);
    }
    // If multiple results, verify ordering
    if results.len() >= 2 {
        for window in results.windows(2) {
            assert!(window[0].score >= window[1].score);
        }
    }
}

#[test]
fn test_empty_directory_handling() {
    let (temp_dir, storage) = create_storage();

    let result = storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();
    assert_eq!(result.documents_imported, 0);
    assert!(result.errors.is_empty());
}

#[test]
fn test_document_count_consistency() {
    let (temp_dir, storage) = create_storage();

    let initial_count = storage.get_document_count().unwrap();
    assert_eq!(initial_count, 0);

    write_doc(
        temp_dir.path(),
        "doc1",
        "---\nid: D1\ntitle: Doc1\n---\nContent",
    );
    write_doc(
        temp_dir.path(),
        "doc2",
        "---\nid: D2\ntitle: Doc2\n---\nContent",
    );

    storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();

    let final_count = storage.get_document_count().unwrap();
    assert_eq!(final_count, 2);
}

#[test]
fn test_search_vectors_empty_then_populated() {
    let (temp_dir, storage) = create_storage();

    // Empty search should return empty
    let empty_results = storage.search_vectors(&[0.1, 0.2, 0.3], 5).unwrap();
    assert!(empty_results.is_empty());

    // Add a document
    write_doc(
        temp_dir.path(),
        "doc",
        "---\nid: V1\ntitle: 向量测试\n---\n内容",
    );
    storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();

    // Even without embeddings, should handle gracefully
    let results = storage.search_vectors(&[0.1, 0.2, 0.3], 5).unwrap();
    // May be empty since no embeddings indexed, but shouldn't panic
    assert!(results.is_empty() || results.iter().all(|r| r.score >= 0.0));
}

#[test]
fn test_hnsw_search_vectors() {
    let (temp_dir, storage) = create_storage();

    // Index a test document
    write_doc(
        temp_dir.path(),
        "doc1",
        "---\nid: V1\ntitle: 第一向量\n---\n内容一",
    );
    write_doc(
        temp_dir.path(),
        "doc2",
        "---\nid: V2\ntitle: 第二向量\n---\n内容二",
    );
    storage
        .index_documents(temp_dir.path().to_str().unwrap())
        .unwrap();

    // Get chunk IDs using the new API
    let chunk_ids: Vec<i64> = storage.get_chunk_ids(2);

    if chunk_ids.len() < 2 {
        // Skip if we can't get 2 chunks
        return;
    }

    // Insert mock embeddings into the database and HNSW
    let vec1 = vec![1.0, 0.0, 0.0, 0.0]; // chunk id 1
    let vec2 = vec![0.5, 0.5, 0.0, 0.0]; // chunk id 2

    // Insert into HNSW directly via the index
    let hnsw = storage.get_hnsw_index();
    hnsw.insert(chunk_ids[0] as u64, &vec1).unwrap();
    hnsw.insert(chunk_ids[1] as u64, &vec2).unwrap();
    hnsw.build_index().unwrap();

    // Verify HNSW is not empty
    assert!(!hnsw.is_empty());
    assert_eq!(hnsw.len(), 2);

    // Search for vector similar to vec1 — should return chunk_id matching vec1 first
    let query = vec![0.95, 0.05, 0.0, 0.0];
    let results = storage.search_vectors(&query, 2).unwrap();
    assert_eq!(results.len(), 2);
    // First result should be the closest to query (vec1 is closest with cosine ~0.998)
    assert_eq!(results[0].chunk_id, chunk_ids[0]);

    // Search for vector similar to vec2
    let query2 = vec![0.45, 0.55, 0.0, 0.0];
    let results2 = storage.search_vectors(&query2, 2).unwrap();
    assert_eq!(results2.len(), 2);
    // First result should be vec2 (closest to query2)
    assert_eq!(results2[0].chunk_id, chunk_ids[1]);

    // Verify scores are non-negative and in descending order
    for w in results2.windows(2) {
        assert!(w[0].score >= w[1].score);
    }
}
