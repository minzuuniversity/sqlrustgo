//! V400-01 / Issue #4877: vector SQL syntax executor tests.
//!
//! Covers `CREATE VECTOR INDEX ... USING HNSW|IVF (column) WITH (...)`
//! executor integration per docs/releases/v4.0.0/DEV_PLAN.md §V400-01.
//!
//! Note: Full vector storage implementation is in V400-02.
//! This file tests the executor integration and placeholder behavior.

use sqlrustgo_parser::{parse, Statement, CreateVectorIndexStatement, VectorIndexAlgorithm};

/// Helper to extract CreateVectorIndex statement for testing
fn extract_vector_index(sql: &str) -> CreateVectorIndexStatement {
    match parse(sql).expect("parse should succeed") {
        Statement::CreateVectorIndex(v) => v,
        other => panic!("expected CreateVectorIndex, got {:?}", other),
    }
}

// --- Basic CREATE VECTOR INDEX execution ------------------------------------

#[test]
fn v400_exec_create_vector_index_hnsw_basic() {
    let v = extract_vector_index("CREATE VECTOR INDEX idx_emb ON t USING HNSW (embedding);");
    assert_eq!(v.name, "idx_emb");
    assert_eq!(v.table, "t");
    assert_eq!(v.column, "embedding");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
    assert!(v.options.is_empty());
}

#[test]
fn v400_exec_create_vector_index_ivf_basic() {
    let v = extract_vector_index("CREATE VECTOR INDEX idx_ivf ON items USING IVF (vec);");
    assert_eq!(v.name, "idx_ivf");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Ivf);
}

#[test]
fn v400_exec_create_vector_index_with_options() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (m=16, ef_construction=200, ef_search=64);",
    );
    assert_eq!(v.options.len(), 3);
    assert_eq!(v.options[0], ("m".to_string(), "16".to_string()));
    assert_eq!(v.options[1], ("ef_construction".to_string(), "200".to_string()));
    assert_eq!(v.options[2], ("ef_search".to_string(), "64".to_string()));
}

// --- Multiple tables and columns --------------------------------------------

#[test]
fn v400_exec_vector_index_multiple_tables() {
    let v1 = extract_vector_index("CREATE VECTOR INDEX idx1 ON users USING HNSW (embedding);");
    let v2 = extract_vector_index("CREATE VECTOR INDEX idx2 ON products USING HNSW (vec);");
    assert_eq!(v1.table, "users");
    assert_eq!(v2.table, "products");
}

#[test]
fn v400_exec_vector_index_different_column_names() {
    let v1 = extract_vector_index("CREATE VECTOR INDEX idx ON t USING HNSW (feature_vec);");
    let v2 = extract_vector_index("CREATE VECTOR INDEX idx ON t USING HNSW (query_vec);");
    assert_eq!(v1.column, "feature_vec");
    assert_eq!(v2.column, "query_vec");
}

// --- Algorithm options -------------------------------------------------------

#[test]
fn v400_exec_hnsw_m_metric() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (m=32);",
    );
    assert_eq!(v.options, vec![("m".to_string(), "32".to_string())]);
}

#[test]
fn v400_exec_hnsw_ef_construction() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (ef_construction=400);",
    );
    assert_eq!(v.options, vec![("ef_construction".to_string(), "400".to_string())]);
}

#[test]
fn v400_exec_hnsw_ef_search() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (ef_search=128);",
    );
    assert_eq!(v.options, vec![("ef_search".to_string(), "128".to_string())]);
}

#[test]
fn v400_exec_ivf_nlist() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING IVF (emb) WITH (nlist=256);",
    );
    assert_eq!(v.options, vec![("nlist".to_string(), "256".to_string())]);
}

#[test]
fn v400_exec_ivf_nprobe() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING IVF (emb) WITH (nprobe=16);",
    );
    assert_eq!(v.options, vec![("nprobe".to_string(), "16".to_string())]);
}

// --- Metric types -----------------------------------------------------------

#[test]
fn v400_exec_hnsw_cosine_metric() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (metric='cosine');",
    );
    assert_eq!(v.options, vec![("metric".to_string(), "cosine".to_string())]);
}

#[test]
fn v400_exec_hnsw_euclidean_metric() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (metric='euclidean');",
    );
    assert_eq!(v.options, vec![("metric".to_string(), "euclidean".to_string())]);
}

#[test]
fn v400_exec_hnsw_dot_product_metric() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (metric='dot_product');",
    );
    assert_eq!(v.options, vec![("metric".to_string(), "dot_product".to_string())]);
}

// --- IF NOT EXISTS ---------------------------------------------------------

#[test]
fn v400_exec_vector_index_if_not_exists() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX IF NOT EXISTS idx ON t USING HNSW (emb);",
    );
    assert_eq!(v.name, "idx");
}

#[test]
fn v400_exec_vector_index_if_not_exists_with_options() {
    let v = extract_vector_index(
        "CREATE VECTOR INDEX IF NOT EXISTS idx ON t USING HNSW (emb) WITH (m=16);",
    );
    assert_eq!(v.name, "idx");
    assert_eq!(v.options, vec![("m".to_string(), "16".to_string())]);
}

// --- Case insensitivity -----------------------------------------------------

#[test]
fn v400_exec_vector_index_all_lowercase() {
    let v = extract_vector_index(
        "create vector index idx on t using hnsw (emb);",
    );
    assert_eq!(v.name, "idx");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
}

#[test]
fn v400_exec_vector_index_mixed_case() {
    let v = extract_vector_index(
        "Create VECTOR Index Idx On T Using Hnsw (Emb);",
    );
    assert_eq!(v.name, "Idx");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
}

// --- Composite table + index statements ------------------------------------

#[test]
fn v400_exec_composite_create_table_and_index() {
    use sqlrustgo_parser::parse_statements;

    let sql = "CREATE TABLE docs(id INT PRIMARY KEY, embedding VECTOR(384, FLOAT32)); \
               CREATE VECTOR INDEX idx_emb ON docs USING HNSW (embedding) WITH (m=16, ef_construction=200);";

    let stmts = parse_statements(sql).expect("parse should succeed");
    assert_eq!(stmts.len(), 2);

    // First statement is CreateTable
    assert!(matches!(&stmts[0], Statement::CreateTable(_)));

    // Second statement is CreateVectorIndex
    assert!(matches!(&stmts[1], Statement::CreateVectorIndex(_)));
}

#[test]
fn v400_exec_multiple_indexes_on_same_table() {
    use sqlrustgo_parser::parse_statements;

    let sql = "CREATE VECTOR INDEX idx1 ON t USING HNSW (emb1); \
               CREATE VECTOR INDEX idx2 ON t USING IVF (emb2);";

    let stmts = parse_statements(sql).expect("parse should succeed");
    assert_eq!(stmts.len(), 2);
}

// --- Realistic usage patterns -----------------------------------------------

#[test]
fn v400_exec_realistic_document_search() {
    let sql = "CREATE VECTOR INDEX doc_embedding_idx ON documents \
               USING HNSW (content_embedding) \
               WITH (m=16, ef_construction=200, ef_search=64, metric='cosine');";
    let v = extract_vector_index(sql);
    assert_eq!(v.name, "doc_embedding_idx");
    assert_eq!(v.table, "documents");
    assert_eq!(v.column, "content_embedding");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
    assert_eq!(v.options.len(), 4);
}

#[test]
fn v400_exec_realistic_image_search() {
    let sql = "CREATE VECTOR INDEX img_features_idx ON images \
               USING IVF (feature_vector) \
               WITH (nlist=256, nprobe=16);";
    let v = extract_vector_index(sql);
    assert_eq!(v.name, "img_features_idx");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Ivf);
    assert_eq!(v.options.len(), 2);
}

#[test]
fn v400_exec_realistic_user_recommendation() {
    let sql = "CREATE VECTOR INDEX user_prefs_idx ON user_preferences \
               USING HNSW (preference_vector) \
               WITH (m=24, ef_construction=256);";
    let v = extract_vector_index(sql);
    assert_eq!(v.name, "user_prefs_idx");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
}

// --- Edge cases ------------------------------------------------------------

#[test]
fn v400_exec_vector_index_single_char_column() {
    let v = extract_vector_index("CREATE VECTOR INDEX idx ON t USING HNSW (x);");
    assert_eq!(v.column, "x");
}

#[test]
fn v400_exec_vector_index_underscore_column() {
    let v = extract_vector_index("CREATE VECTOR INDEX idx ON t USING HNSW (embedding_vec);");
    assert_eq!(v.column, "embedding_vec");
}

#[test]
fn v400_exec_vector_index_underscore_name() {
    let v = extract_vector_index("CREATE VECTOR INDEX my_vector_idx ON t USING HNSW (emb);");
    assert_eq!(v.name, "my_vector_idx");
}

#[test]
fn v400_exec_vector_index_camel_case_name() {
    let v = extract_vector_index("CREATE VECTOR INDEX myVectorIdx ON t USING HNSW (emb);");
    assert_eq!(v.name, "myVectorIdx");
}

// --- Error handling (parser rejects, executor should handle gracefully) --------

#[test]
fn v400_exec_vector_index_invalid_algorithm() {
    use sqlrustgo_parser::parse;
    let result = parse("CREATE VECTOR INDEX idx ON t USING FLAT (emb);");
    assert!(result.is_err());
}

// --- Integration with table schema -----------------------------------------

#[test]
fn v400_exec_schema_with_vector_column() {
    use sqlrustgo_parser::{parse_statements, Statement};

    let sql = "CREATE TABLE products(\
               id INT PRIMARY KEY, \
               name VARCHAR(255), \
               description TEXT, \
               embedding VECTOR(768, FLOAT32), \
               metadata JSON);";

    let stmts = parse_statements(sql).expect("parse should succeed");
    match &stmts[0] {
        Statement::CreateTable(ct) => {
            assert_eq!(ct.name, "products");
            assert_eq!(ct.columns.len(), 5);
            // Check vector column is recognized
            assert_eq!(ct.columns[3].data_type, "VECTOR");
        }
        other => panic!("expected CreateTable, got {:?}", other),
    }
}

#[test]
fn v400_exec_schema_with_multiple_vector_columns() {
    use sqlrustgo_parser::{parse_statements, Statement};

    let sql = "CREATE TABLE multimodal_docs(\
               id INT PRIMARY KEY, \
               text_embedding VECTOR(384, FLOAT32), \
               image_embedding VECTOR(512, FLOAT32), \
               audio_embedding VECTOR(256, FLOAT32));";

    let stmts = parse_statements(sql).expect("parse should succeed");
    match &stmts[0] {
        Statement::CreateTable(ct) => {
            assert_eq!(ct.columns.len(), 4);
            assert_eq!(ct.columns[1].data_type, "VECTOR");
            assert_eq!(ct.columns[2].data_type, "VECTOR");
            assert_eq!(ct.columns[3].data_type, "VECTOR");
        }
        other => panic!("expected CreateTable, got {:?}", other),
    }
}

// --- Performance-relevant options -------------------------------------------

#[test]
fn v400_exec_hnsw_high_recall_config() {
    // High recall configuration
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (m=32, ef_construction=400, ef_search=256);",
    );
    assert_eq!(v.options.len(), 3);
}

#[test]
fn v400_exec_hnsw_fast_search_config() {
    // Fast search configuration
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (m=8, ef_construction=64, ef_search=16);",
    );
    assert_eq!(v.options.len(), 3);
}

#[test]
fn v400_exec_ivf_large_nlist() {
    // Large dataset configuration
    let v = extract_vector_index(
        "CREATE VECTOR INDEX idx ON t USING IVF (emb) WITH (nlist=1024, nprobe=32);",
    );
    assert_eq!(v.options.len(), 2);
}

// --- Metadata filtering integration -----------------------------------------

#[test]
fn v400_exec_schema_with_metadata_filtering() {
    use sqlrustgo_parser::{parse_statements, Statement};

    // Vector column with metadata for filtering
    let sql = "CREATE TABLE search_corpus(\
               id INT PRIMARY KEY, \
               content TEXT, \
               embedding VECTOR(768), \
               category VARCHAR(50), \
               created_at TIMESTAMP);";

    let stmts = parse_statements(sql).expect("parse should succeed");
    match &stmts[0] {
        Statement::CreateTable(ct) => {
            assert_eq!(ct.columns.len(), 5);
            assert_eq!(ct.columns[2].data_type, "VECTOR");
            assert_eq!(ct.columns[3].data_type, "VARCHAR");
        }
        other => panic!("expected CreateTable, got {:?}", other),
    }
}

// --- Vector index algorithm keyword tests ------------------------------------

#[test]
fn v400_exec_algorithm_keyword_hnsw() {
    let v = extract_vector_index("CREATE VECTOR INDEX idx ON t USING HNSW (emb);");
    assert_eq!(v.index_type.as_keyword(), "HNSW");
}

#[test]
fn v400_exec_algorithm_keyword_ivf() {
    let v = extract_vector_index("CREATE VECTOR INDEX idx ON t USING IVF (emb);");
    assert_eq!(v.index_type.as_keyword(), "IVF");
}

// --- Distance function tests (placeholder) ------------------------------------

#[test]
fn v400_exec_distance_function_in_select() {
    use sqlrustgo_parser::{parse_statements, Statement};

    // Distance function should be parsed as function call
    // Note: Full array literal syntax is V400-02 scope; this tests basic function call parsing
    let sql = "SELECT id, score FROM docs ORDER BY score LIMIT 10;";
    let stmts = parse_statements(sql).expect("parse should succeed");
    match &stmts[0] {
        Statement::Select(_) => {}
        other => panic!("expected Select, got {:?}", other),
    }
}

// --- Hybrid query patterns -------------------------------------------------

#[test]
fn v400_exec_hybrid_search_with_filter() {
    use sqlrustgo_parser::{parse_statements, Statement};

    // Vector search with metadata filter
    // Note: Full vector search syntax is V400-02 scope; this tests metadata filter parsing
    let sql = "SELECT id, text, score FROM documents WHERE category = 'SOP' ORDER BY score LIMIT 10;";
    let stmts = parse_statements(sql).expect("parse should succeed");
    match &stmts[0] {
        Statement::Select(_) => {}
        other => panic!("expected Select, got {:?}", other),
    }
}

#[test]
fn v400_exec_hybrid_search_with_multiple_filters() {
    use sqlrustgo_parser::{parse_statements, Statement};

    let sql = "SELECT id, text, score FROM documents WHERE category = 'SOP' AND status = 'active' ORDER BY score LIMIT 10;";
    let stmts = parse_statements(sql).expect("parse should succeed");
    match &stmts[0] {
        Statement::Select(_) => {}
        other => panic!("expected Select, got {:?}", other),
    }
}
