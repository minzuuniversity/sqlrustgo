//! PR-B / #5305 — EXPLAIN `pages_visited` suffix integration test for the
//! default SQL-path engine (`ExecutionEngine<MemoryStorage>`).
//!
//! Verifies the PR-B contract:
//! - `with_memory_and_bufferpool` engine + INSERT + SELECT → EXPLAIN emits
//!   ` | pages_visited=<N>` with `N >= 1`.
//! - `with_memory()` engine (legacy) + INSERT → EXPLAIN omits the suffix
//!   (`pages_visited == 0`).

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{BufferPool, MemoryStorage};
use std::sync::Arc;

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
fn test_explain_appends_pages_visited_suffix_via_with_memory_and_bufferpool() {
    let pool = Arc::new(BufferPool::new(64));
    let mut engine = ExecutionEngine::<MemoryStorage>::with_memory_and_bufferpool(pool);

    engine
        .execute("CREATE TABLE t (id INTEGER);")
        .expect("create");
    engine
        .execute("INSERT INTO t VALUES (1), (2);")
        .expect("insert 2 rows");

    let result = engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");
    assert!(!result.rows.is_empty(), "EXPLAIN must emit >= 1 row");
    for (i, row) in result.rows.iter().enumerate() {
        let line = first_cell_text(row, i);
        let n = last_pages_visited_token(line).unwrap_or_else(|| {
            panic!("row {i} must end with ' | pages_visited=<u64>'; got: {line}")
        });
        assert!(
            n >= 1,
            "row {i}: pages_visited must be >= 1 after INSERT+SELECT; got {n}"
        );
    }
}

#[test]
fn test_explain_omits_pages_visited_suffix_for_legacy_with_memory() {
    // Back-compat: legacy `with_memory()` continues to use synthetic_zero
    // stats; `pages_visited` stays 0; suffix is omitted.
    let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER);")
        .expect("create");
    engine.execute("INSERT INTO t VALUES (1);").expect("insert");
    let result = engine
        .execute("EXPLAIN SELECT id FROM t;")
        .expect("EXPLAIN must succeed");
    assert!(!result.rows.is_empty());
    for (i, row) in result.rows.iter().enumerate() {
        let line = first_cell_text(row, i);
        assert!(
            !line.contains(" | pages_visited="),
            "row {i}: legacy with_memory() must omit pages_visited suffix; got: {line}"
        );
    }
}
