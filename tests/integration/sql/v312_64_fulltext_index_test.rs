//! V312-64 / Issue #4645 regression integration test:
//! `CREATE FULLTEXT INDEX` must parse without error and the executor
//! must return a clear runtime error pointing to the SQLite-FTS5
//! alternative.
//!
//! Uses the public `ExecutionEngine::execute` path so the parser,
//! executor, and storage layers are exercised end-to-end — matching
//! the `v312_63_parser_issues_test.rs` convention.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_64_create_fulltext_index_parses_and_executor_rejects() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER, body TEXT)").unwrap();
    let err = x
        .execute("CREATE FULLTEXT INDEX ft_idx ON t(body)")
        .expect_err("FULLTEXT INDEX must be rejected at runtime (issue #4645)");
    let msg = err.to_string();
    assert!(
        msg.contains("FULLTEXT INDEX is not yet implemented"),
        "error must mention 'FULLTEXT INDEX is not yet implemented', got: {}",
        msg
    );
    assert!(
        msg.contains("4645"),
        "error must reference issue #4645 for traceability, got: {}",
        msg
    );
    assert!(
        msg.contains("fts5"),
        "error must suggest the SQLite-FTS5 alternative, got: {}",
        msg
    );
}

#[test]
fn v312_64_create_fulltext_index_multi_column_parses() {
    let mut x = fresh();
    x.execute("CREATE TABLE articles (id INTEGER, title TEXT, body TEXT, tags TEXT)")
        .unwrap();
    let err = x
        .execute("CREATE FULLTEXT INDEX ft_articles ON articles(title, body, tags)")
        .expect_err("FULLTEXT INDEX must be rejected at runtime");
    let msg = err.to_string();
    assert!(msg.contains("articles"), "error must name the table");
    assert!(
        msg.contains("title") && msg.contains("body") && msg.contains("tags"),
        "error must list all 3 columns in the fts5 suggestion, got: {}",
        msg
    );
}

#[test]
fn v312_64_create_fulltext_index_if_not_exists_parses() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER, body TEXT)").unwrap();
    let err = x
        .execute("CREATE FULLTEXT INDEX IF NOT EXISTS ft_idx ON t(body)")
        .expect_err("FULLTEXT INDEX IF NOT EXISTS must be rejected at runtime");
    assert!(err.to_string().contains("FULLTEXT INDEX"));
}

#[test]
fn v312_64_fulltext_index_no_longer_crashes_parser() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (body TEXT)").unwrap();
    let result = x.execute("CREATE FULLTEXT INDEX ft_idx ON t(body)");
    match result {
        Err(e) => {
            let msg = e.to_string();
            assert!(
                !msg.to_lowercase().contains("parse error"),
                "FULLTEXT INDEX must not produce a parse error; got: {}",
                msg
            );
        }
        Ok(_) => panic!("FULLTEXT INDEX must reject at runtime, but execute returned Ok"),
    }
}

#[test]
fn v312_64_fulltext_index_missing_name_rejected_at_parse() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (body TEXT)").unwrap();
    let err = x
        .execute("CREATE FULLTEXT INDEX ON t(body)")
        .expect_err("missing index name must error");
    assert!(
        err.to_string().to_lowercase().contains("parse error"),
        "missing-name FULLTEXT INDEX must produce a parse error, got: {}",
        err
    );
}
