//! V312-89 / Issue #4759 — CTE column aliases on first anchor.
//!
//! Tests cover:
//! - `walk(n)` column alias + `SELECT 1 AS n` first anchor — exact issue body
//! - `walk(n)` with column count match (issue body case)
//! - Multi-column alias `t(a, b)` with explicit AS in anchor
//! - Original counter pattern (no alias) still works (regression guard)

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

// ============================================================================
// Issue #4759 — exact issue body case
// ============================================================================

#[test]
fn rec_cte_walk_with_column_alias_4759() {
    // V312-89 / Issue #4759: CTE column alias `walk(n)` + `SELECT 1 AS n`
    // first anchor. The original report claims this is unsupported.
    let mut e = fresh_mem();
    let r = e.execute(
        "WITH RECURSIVE walk(n) AS ( \
            SELECT 1 AS n \
            UNION ALL \
            SELECT n + 1 FROM walk WHERE n < 5 \
         ) SELECT n FROM walk ORDER BY n",
    );
    assert!(
        r.is_ok(),
        "walk(n) CTE with `SELECT 1 AS n` anchor failed: {:?}",
        r.err()
    );
    let rows = r.unwrap().rows;
    assert_eq!(rows.len(), 5, "expected 5 rows, got {}", rows.len());
    for i in 0..5 {
        assert_eq!(rows[i][0], Value::Integer((i + 1) as i64));
    }
}

#[test]
fn rec_cte_walk_select_star_4759() {
    // Issue body uses `SELECT * FROM walk` — verify that path too.
    let mut e = fresh_mem();
    let r = e.execute(
        "WITH RECURSIVE walk(n) AS ( \
            SELECT 1 AS n \
            UNION ALL \
            SELECT n + 1 FROM walk WHERE n < 5 \
         ) SELECT * FROM walk ORDER BY n",
    );
    assert!(
        r.is_ok(),
        "walk(n) with SELECT * failed: {:?}",
        r.err()
    );
    let rows = r.unwrap().rows;
    assert_eq!(rows.len(), 5, "expected 5 rows, got {}", rows.len());
    for i in 0..5 {
        assert_eq!(rows[i][0], Value::Integer((i + 1) as i64));
    }
}

// ============================================================================
// Multi-column alias variant — broader regression coverage
// ============================================================================

#[test]
fn rec_cte_multi_column_alias_4759() {
    // Two-column alias `t(a, b)` with first anchor SELECT 1, 'x'.
    let mut e = fresh_mem();
    let r = e.execute(
        "WITH RECURSIVE t(a, b) AS ( \
            SELECT 1, 'x' \
            UNION ALL \
            SELECT a + 1, b FROM t WHERE a < 3 \
         ) SELECT a, b FROM t ORDER BY a",
    );
    assert!(
        r.is_ok(),
        "t(a, b) multi-column alias failed: {:?}",
        r.err()
    );
    let rows = r.unwrap().rows;
    assert_eq!(rows.len(), 3, "expected 3 rows, got {}", rows.len());
    assert_eq!(rows[0][0], Value::Integer(1));
    assert_eq!(rows[0][1], Value::Text("x".into()));
    assert_eq!(rows[2][0], Value::Integer(3));
}

// ============================================================================
// Regression guard — pre-existing pattern without alias still works
// ============================================================================

#[test]
fn rec_cte_no_alias_still_works_4759() {
    // The pattern from the existing `rec_cte_count_1_to_n` test (no column
    // alias, no AS in anchor). Must still work post-fix.
    let mut e = fresh_mem();
    let r = e
        .execute(
            "WITH RECURSIVE cnt AS ( \
            SELECT 1 AS n UNION ALL \
            SELECT n + 1 FROM cnt WHERE n < 5 \
         ) SELECT n FROM cnt ORDER BY n",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 5, "expected 5 rows, got {}", r.rows.len());
}