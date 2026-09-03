//! V312-64f / Issue #4699 — Recursive CTE integration tests.
//!
//! Tests cover:
//! - Hierarchy traversal (issue body case)
//! - Count 1..N (terminating recursion)
//! - UNION (non-ALL) dedup
//! - Empty anchor
//! - Non-UNION body rejection
//! - Multiple CTEs mixed (recursive + non-recursive)
//! - Cleanup after recursive CTE
//! - MAX_RECURSION_ROWS exceeded
//! - Aggregation inside step

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

// ============================================================================
// Issue #4699 — hierarchy/org-tree traversal (issue body case)
// ============================================================================

#[test]
fn rec_cte_hierarchy_issue_body() {
    // V312-64f / Issue #4699: org hierarchy traversal.
    // Expected: 5 rows (CEO + 2 directs + 2 indirects).
    let mut e = fresh_mem();
    e.execute("CREATE TABLE emp(id INT, mgr_id INT, name TEXT)")
        .unwrap();
    e.execute("INSERT INTO emp VALUES (1, NULL, 'CEO'), (2, 1, 'VP'), (3, 1, 'CTO'), (4, 2, 'Dev1'), (5, 2, 'Dev2')")
        .unwrap();

    let r = e.execute(
        "WITH RECURSIVE tree AS ( \
            SELECT id, name FROM emp WHERE mgr_id IS NULL \
            UNION ALL \
            SELECT e.id, e.name FROM emp e JOIN tree ON e.mgr_id = tree.id \
         ) SELECT id, name FROM tree ORDER BY id",
    )
    .unwrap();

    assert_eq!(r.rows.len(), 5, "expected 5 rows, got {}", r.rows.len());
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[0][1], Value::Text("CEO".into()));
    assert_eq!(r.rows[1][0], Value::Integer(2));
    assert_eq!(r.rows[2][0], Value::Integer(3));
    assert_eq!(r.rows[3][0], Value::Integer(4));
    assert_eq!(r.rows[4][0], Value::Integer(5));
}
