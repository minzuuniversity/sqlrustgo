//! V312-64 / Issue #4645 regression integration test:
//! `CREATE FULLTEXT INDEX` must parse without error and the executor
//! must return a clear runtime error pointing to the SQLite-FTS5
//! alternative.
//!
//! Uses the public `ExecutionEngine::execute` path so the parser,
//! executor, and storage layers are exercised end-to-end — matching
//! the `v312_63_parser_issues_test.rs` convention.
//!
//! Companion to the unit tests in `crates/parser/src/parser.rs`
//! (test_create_fulltext_index_basic / _multi_column / _if_not_exists /
//! _lowercase_keyword / _missing_name_errors / _missing_columns_errors)
//! and `crates/sqlrustgo-cli/src/sqlite_mode.rs::tests`
//! (fulltext_index_parses_and_executor_rejects_with_clear_error).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Issue #4645: parser must accept `CREATE FULLTEXT INDEX <name> ON
/// <table>(<col>)` and the executor must return a clear runtime error
/// referencing the issue number and the SQLite-FTS5 alternative.
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

/// Multi-column FULLTEXT INDEX must parse (single fragment).
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
    // The error must list the column set in the fts5 suggestion.
    assert!(
        msg.contains("title") && msg.contains("body") && msg.contains("tags"),
        "error must list all 3 columns in the fts5 suggestion, got: {}",
        msg
    );
}

/// `IF NOT EXISTS` variant must parse (single fragment).
#[test]
fn v312_64_create_fulltext_index_if_not_exists_parses() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER, body TEXT)").unwrap();
    let err = x
        .execute("CREATE FULLTEXT INDEX IF NOT EXISTS ft_idx ON t(body)")
        .expect_err("FULLTEXT INDEX IF NOT EXISTS must be rejected at runtime");
    assert!(err.to_string().contains("FULLTEXT INDEX"));
}

/// Original parse error from the issue report (`Parse error: Expected
/// TABLE, ... after CREATE, got Fulltext`) must NO LONGER occur.
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

/// Malformed `CREATE FULLTEXT INDEX` (missing name) must still be
/// rejected at parse time — no silent acceptance of garbage.
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
