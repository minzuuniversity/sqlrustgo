//! PR-A / #5305 — EXPLAIN `pages_visited` suffix integration tests.
//!
//! Verifies the PR-A contract:
//! - Every EXPLAIN output row ends with ` | pages_visited=<u64>` when the
//!   underlying storage has at least one page visit (i.e. when a
//!   BufferPool-backed `FileStorage` was the target of an INSERT or
//!   SCAN before EXPLAIN ran).
//! - When `pages_visited == 0` (e.g. MemoryStorage, which returns the
//!   synthetic-zero stats), the suffix is omitted entirely — this is
//!   the pre-PR-A behaviour and we don't want to break it.
//! - The value is snapshotted at EXPLAIN entry so all rows are equal.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::BufferPool;
use std::sync::Arc;

/// Extract the `pages_visited=<u64>` token if present at row-end.
fn last_pages_visited_token(line: &str) -> Option<u64> {
    line.split(" | pages_visited=")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u64>().ok())
}

fn first_cell_text(row: &[sqlrustgo_types::Value], i: usize) -> &str {
    row.first()
        .and_then(|v| match v {
            sqlrustgo_types::Value::Text(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("row {i} first cell must be Value::Text"))
}

#[test]
fn test_explain_appends_pages_visited_suffix_after_insert_via_bufferpool() {
    // PR-A.5: a BufferPool-backed FileStorage is required to drive
    // `pages_visited` > 0. `with_memory()` always returns the
    // synthetic-zero stats and would skip the suffix.
    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(64));
    let storage = Arc::new(parking_lot::RwLock::new(
        sqlrustgo_storage::FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool)
            .expect("FileStorage::new_with_bufferpool"),
    ));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE t (id INTEGER);")
        .expect("create");
    engine.execute("INSERT INTO t VALUES (1);").expect("insert");

    let result = engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");

    assert!(!result.rows.is_empty(), "EXPLAIN must emit ≥ 1 row");
    for (i, row) in result.rows.iter().enumerate() {
        let line = first_cell_text(row, i);
        let n = last_pages_visited_token(line).unwrap_or_else(|| {
            panic!("row {i} must end with ' | pages_visited=<u64>'; got: {line}")
        });
        assert!(
            n >= 1,
            "row {i}: pages_visited must be >= 1 after INSERT; got {n}"
        );
    }
}

#[test]
fn test_explain_omits_pages_visited_suffix_when_zero() {
    // MemoryStorage's `buffer_pool_stats()` returns `synthetic_zero()`
    // and `pages_visited` stays 0 → suffix must be omitted entirely.
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER);")
        .expect("create");

    let result = engine
        .execute("EXPLAIN SELECT id FROM t;")
        .expect("EXPLAIN must succeed");

    assert!(!result.rows.is_empty(), "EXPLAIN must emit ≥ 1 row");
    for (i, row) in result.rows.iter().enumerate() {
        let line = first_cell_text(row, i);
        assert!(
            !line.contains(" | pages_visited="),
            "row {i}: pages_visited=0 must be omitted; got: {line}"
        );
    }
}
