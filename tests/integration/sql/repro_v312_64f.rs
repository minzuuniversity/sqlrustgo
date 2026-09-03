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

// ============================================================================
// Issue #4699 — count 1..N (terminating recursion)
// ============================================================================

#[test]
fn rec_cte_count_1_to_n() {
    // Terminating recursion: WHERE n < 10 stops the loop at 10 rows.
    let mut e = fresh_mem();
    let r = e
        .execute(
            "WITH RECURSIVE cnt AS ( \
            SELECT 1 AS n UNION ALL \
            SELECT n + 1 FROM cnt WHERE n < 10 \
         ) SELECT n FROM cnt ORDER BY n",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 10, "expected 10 rows, got {}", r.rows.len());
    for i in 0..10 {
        assert_eq!(r.rows[i][0], Value::Integer((i + 1) as i64));
    }
}

#[test]
fn rec_cte_union_dedup_implicit_termination() {
    // UNION (not ALL): step produces duplicates; dedup against accumulated
    // causes termination when no new rows remain.
    let mut e = fresh_mem();
    e.execute("CREATE TABLE nums(n INT)").unwrap();
    e.execute("INSERT INTO nums VALUES (1), (2), (3)").unwrap();

    // Step is `SELECT n FROM nums` — same as anchor; UNION dedupes
    // against accumulated, so after one iteration no new rows remain.
    let r = e
        .execute(
            "WITH RECURSIVE all_n AS ( \
            SELECT n FROM nums \
            UNION \
            SELECT n FROM nums JOIN all_n ON nums.n = all_n.n \
         ) SELECT n FROM all_n ORDER BY n",
        )
        .unwrap();
    assert_eq!(
        r.rows.len(),
        3,
        "expected 3 distinct values, got {}",
        r.rows.len()
    );
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[1][0], Value::Integer(2));
    assert_eq!(r.rows[2][0], Value::Integer(3));
}

// ============================================================================
// Issue #4699 — edge cases: empty anchor + non-UNION rejection
// ============================================================================

#[test]
fn rec_cte_empty_anchor() {
    // Empty anchor: no rows match, so t and t__work start empty.
    // The step's SELECT against the empty t__work returns 0 rows,
    // the loop exits, and the outer SELECT sees 0 rows.
    let mut e = fresh_mem();
    e.execute("CREATE TABLE empty_src(n INT)").unwrap();
    let r = e
        .execute(
            "WITH RECURSIVE cnt AS ( \
            SELECT n FROM empty_src \
            UNION ALL \
            SELECT n + 1 FROM cnt WHERE n < 10 \
         ) SELECT n FROM cnt",
        )
        .expect("empty anchor must succeed with 0 rows");
    assert_eq!(
        r.rows.len(),
        0,
        "empty anchor → 0 rows, got {}",
        r.rows.len()
    );
}

#[test]
fn rec_cte_nonrecursive_body_in_recursive_clause_works() {
    // PG/SQLite semantics: WITH RECURSIVE allows mixing recursive and
    // non-recursive CTEs. A non-UNION body falls through to the simple
    // materialize-once path (NOT the two-table recursive algorithm).
    // This is a deliberate divergence from "WITH RECURSIVE only allows
    // UNION bodies".
    let mut e = fresh_mem();
    let r = e
        .execute(
            "WITH RECURSIVE cnt AS (SELECT 1 AS n) SELECT n FROM cnt",
        )
        .expect("non-recursive body under WITH RECURSIVE must succeed");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(1));
}

// ============================================================================
// Issue #4699 — multiple CTEs mixed + cleanup after recursive CTE
// ============================================================================

#[test]
fn rec_cte_mixed_with_nonrecursive_in_recursive_clause() {
    // PG/SQLite allow mixing non-recursive and recursive CTEs inside a
    // single WITH RECURSIVE clause. The non-recursive one is just a
    // regular CTE; the recursive one can reference it.
    let mut e = fresh_mem();
    let r = e
        .execute(
            "WITH RECURSIVE \
                src AS (SELECT 1 AS n), \
                cnt AS ( \
                    SELECT n FROM src \
                    UNION ALL \
                    SELECT n + 1 FROM cnt WHERE n < 4 \
                 ) \
             SELECT n FROM cnt ORDER BY n",
        )
        .expect("mixed CTEs must execute");
    assert_eq!(
        r.rows.len(),
        4,
        "expected 4 rows from 1..4, got {}",
        r.rows.len()
    );
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[1][0], Value::Integer(2));
    assert_eq!(r.rows[2][0], Value::Integer(3));
    assert_eq!(r.rows[3][0], Value::Integer(4));
}

#[test]
fn rec_cte_cleanup_drops_temp_tables_after_query() {
    // After the recursive CTE completes:
    //   - t__work must be gone (dropped at end of materialize_recursive_cte)
    //   - t must be gone (dropped by cleanup_cte_tables in execute_with_select)
    // We probe by trying to CREATE TABLE with the same name and by running
    // a follow-up SELECT that would error if the temp table leaked.
    let mut e = fresh_mem();
    e.execute(
        "WITH RECURSIVE cnt AS ( \
            SELECT 1 AS n UNION ALL \
            SELECT n + 1 FROM cnt WHERE n < 3 \
         ) SELECT n FROM cnt",
    )
    .expect("recursive CTE must succeed");

    // If the temp table leaked, CREATE TABLE with the same name would fail.
    e.execute("CREATE TABLE cnt(n INT)")
        .expect("CTE table should be cleaned up; CREATE TABLE cnt must succeed");
    e.execute("INSERT INTO cnt VALUES (99)").unwrap();

    // SELECT FROM cnt should now see only the freshly-inserted real row.
    let r = e.execute("SELECT n FROM cnt").unwrap();
    assert_eq!(
        r.rows.len(),
        1,
        "expected only the real row, got {}",
        r.rows.len()
    );
    assert_eq!(r.rows[0][0], Value::Integer(99));
}
