//! V400-01 / Issue #4877: vector SQL syntax parser tests.
//!
//! Covers `CREATE VECTOR INDEX ... USING HNSW|IVF (column) WITH (...)`
//! per docs/releases/v4.0.0/DEV_PLAN.md §V400-01.

use sqlrustgo_parser::{
    parse, parse_statements, CreateVectorIndexStatement, Statement, VectorIndexAlgorithm,
};

fn parse_create_vector(sql: &str) -> CreateVectorIndexStatement {
    match parse(sql).expect("parse should succeed") {
        Statement::CreateVectorIndex(v) => v,
        other => panic!("expected CreateVectorIndex, got {:?}", other),
    }
}

fn parse_err(sql: &str) -> String {
    parse(sql).err().expect("parse should fail")
}

// --- Algorithm variants ----------------------------------------------------

#[test]
fn v400_create_vector_index_hnsw_basic() {
    let v = parse_create_vector("CREATE VECTOR INDEX idx_emb ON t USING HNSW (embedding);");
    assert_eq!(v.name, "idx_emb");
    assert_eq!(v.table, "t");
    assert_eq!(v.column, "embedding");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
    assert!(v.options.is_empty());
}

#[test]
fn v400_create_vector_index_ivf_basic() {
    let v = parse_create_vector("CREATE VECTOR INDEX idx_ivf ON items USING IVF (vec);");
    assert_eq!(v.name, "idx_ivf");
    assert_eq!(v.table, "items");
    assert_eq!(v.column, "vec");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Ivf);
    assert!(v.options.is_empty());
}

#[test]
fn v400_create_vector_index_column_without_parens() {
    let v = parse_create_vector("CREATE VECTOR INDEX idx ON t USING HNSW embedding;");
    assert_eq!(v.column, "embedding");
}

// --- WITH clause options ---------------------------------------------------

#[test]
fn v400_create_vector_index_hnsw_with_options() {
    let v = parse_create_vector(
        "CREATE VECTOR INDEX idx_emb ON t USING HNSW (embedding) WITH (m=16, ef_construction=200);",
    );
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
    assert_eq!(
        v.options,
        vec![
            ("m".to_string(), "16".to_string()),
            ("ef_construction".to_string(), "200".to_string()),
        ]
    );
}

#[test]
fn v400_create_vector_index_ivf_with_options() {
    let v = parse_create_vector("CREATE VECTOR INDEX idx_ivf ON t USING IVF (vec) WITH (nlist=100);");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Ivf);
    assert_eq!(v.options, vec![("nlist".to_string(), "100".to_string())]);
}

#[test]
fn v400_create_vector_index_with_multiple_options() {
    let v = parse_create_vector(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (m=16, ef_construction=200, ef_search=64);",
    );
    assert_eq!(v.options.len(), 3);
    assert_eq!(v.options[0], ("m".to_string(), "16".to_string()));
    assert_eq!(
        v.options[1],
        ("ef_construction".to_string(), "200".to_string())
    );
    assert_eq!(v.options[2], ("ef_search".to_string(), "64".to_string()));
}

#[test]
fn v400_create_vector_index_with_string_option_value() {
    let v = parse_create_vector(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (metric='cosine');",
    );
    assert_eq!(
        v.options,
        vec![("metric".to_string(), "cosine".to_string())]
    );
}

#[test]
fn v400_create_vector_index_with_identifier_option_value() {
    let v = parse_create_vector(
        "CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (metric=cosine);",
    );
    assert_eq!(
        v.options,
        vec![("metric".to_string(), "cosine".to_string())]
    );
}

// --- IF NOT EXISTS ---------------------------------------------------------

#[test]
fn v400_create_vector_index_if_not_exists() {
    let v = parse_create_vector(
        "CREATE VECTOR INDEX IF NOT EXISTS idx_emb ON t USING HNSW (embedding);",
    );
    assert_eq!(v.name, "idx_emb");
    assert_eq!(v.column, "embedding");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
}

// --- Case insensitivity ----------------------------------------------------

#[test]
fn v400_create_vector_index_lowercase_keywords() {
    let v = parse_create_vector("create vector index idx_emb on t using hnsw (embedding);");
    assert_eq!(v.name, "idx_emb");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
}

#[test]
fn v400_create_vector_index_mixed_case() {
    let v = parse_create_vector("Create Vector Index Idx On T Using Hnsw (Col);");
    assert_eq!(v.name, "Idx");
    assert_eq!(v.table, "T");
    assert_eq!(v.column, "Col");
    assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
}

// --- Error paths -----------------------------------------------------------

#[test]
fn v400_create_vector_index_wrong_algorithm_errors() {
    let e = parse_err("CREATE VECTOR INDEX idx ON t USING FLAT (embedding);");
    assert!(e.contains("HNSW") || e.contains("IVF"), "got: {}", e);
}

#[test]
fn v400_create_vector_index_missing_column_errors() {
    let e = parse_err("CREATE VECTOR INDEX idx ON t USING HNSW;");
    assert!(!e.is_empty(), "expected error, got empty");
}

#[test]
fn v400_create_vector_index_with_missing_eq_errors() {
    let e = parse_err("CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (m 16);");
    assert!(!e.is_empty(), "expected error, got empty");
}

// --- VECTOR as column type (data_type is String, no AST change needed) -----

#[test]
fn v400_create_table_with_vector_column_parses() {
    let stmts = parse_statements(
        "CREATE TABLE t(id INT PRIMARY KEY, embedding VECTOR(384, FLOAT32));",
    )
    .expect("parse should succeed");
    match &stmts[0] {
        Statement::CreateTable(ct) => {
            assert_eq!(ct.columns.len(), 2);
            assert_eq!(ct.columns[1].data_type, "VECTOR");
        }
        other => panic!("expected CreateTable, got {:?}", other),
    }
}

#[test]
fn v400_create_table_with_vector_and_meta_parses() {
    let stmts = parse_statements(
        "CREATE TABLE docs(id INT PRIMARY KEY, embedding VECTOR(384), meta JSON);",
    )
    .expect("parse should succeed");
    match &stmts[0] {
        Statement::CreateTable(ct) => {
            assert_eq!(ct.columns.len(), 3);
            assert_eq!(ct.columns[1].data_type, "VECTOR");
            assert_eq!(ct.columns[2].data_type, "JSON");
        }
        other => panic!("expected CreateTable, got {:?}", other),
    }
}

// --- Algorithm keyword preservation ---------------------------------------

#[test]
fn v400_vector_index_algorithm_as_keyword_hnsw() {
    let v = parse_create_vector("CREATE VECTOR INDEX idx ON t USING HNSW (emb);");
    assert_eq!(v.index_type.as_keyword(), "HNSW");
}

#[test]
fn v400_vector_index_algorithm_as_keyword_ivf() {
    let v = parse_create_vector("CREATE VECTOR INDEX idx ON t USING IVF (emb);");
    assert_eq!(v.index_type.as_keyword(), "IVF");
}

// --- Realistic composite: CREATE TABLE + CREATE VECTOR INDEX --------------

#[test]
fn v400_realistic_create_table_then_vector_index() {
    let stmts = parse_statements(
        "CREATE TABLE docs(id INT PRIMARY KEY, embedding VECTOR(384, FLOAT32), meta JSON);\
         CREATE VECTOR INDEX idx_emb ON docs USING HNSW (embedding) WITH (m=16, ef_construction=200);",
    )
    .expect("parse should succeed");
    assert_eq!(stmts.len(), 2);

    match &stmts[0] {
        Statement::CreateTable(_) => {}
        other => panic!("expected CreateTable first, got {:?}", other),
    }
    match &stmts[1] {
        Statement::CreateVectorIndex(v) => {
            assert_eq!(v.name, "idx_emb");
            assert_eq!(v.table, "docs");
            assert_eq!(v.column, "embedding");
            assert_eq!(v.index_type, VectorIndexAlgorithm::Hnsw);
            assert_eq!(v.options.len(), 2);
        }
        other => panic!("expected CreateVectorIndex second, got {:?}", other),
    }
}

#[test]
fn v400_realistic_create_table_with_ivf_index() {
    let stmts = parse_statements(
        "CREATE TABLE items(id INT PRIMARY KEY, vec VECTOR(128, FLOAT16));\
         CREATE VECTOR INDEX idx_ivf ON items USING IVF (vec) WITH (nlist=256);",
    )
    .expect("parse should succeed");
    assert_eq!(stmts.len(), 2);

    match &stmts[1] {
        Statement::CreateVectorIndex(v) => {
            assert_eq!(v.index_type, VectorIndexAlgorithm::Ivf);
            assert_eq!(v.options, vec![("nlist".to_string(), "256".to_string())]);
        }
        other => panic!("expected CreateVectorIndex, got {:?}", other),
    }
}

// --- Numeric option variants ----------------------------------------------

#[test]
fn v400_hnsw_with_large_m_value() {
    let v = parse_create_vector("CREATE VECTOR INDEX idx ON t USING HNSW (emb) WITH (m=128);");
    assert_eq!(v.options[0], ("m".to_string(), "128".to_string()));
}

#[test]
fn v400_ivf_with_nprobe_option() {
    let v = parse_create_vector(
        "CREATE VECTOR INDEX idx ON t USING IVF (emb) WITH (nlist=100, nprobe=8);",
    );
    assert_eq!(v.options.len(), 2);
    assert_eq!(v.options[1], ("nprobe".to_string(), "8".to_string()));
}
