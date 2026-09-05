//! V312-95 v2 / Issue #4717: `INSERT INTO ... SELECT ... FROM (WITH [RECURSIVE] ...)`
//! and `SELECT ... FROM (WITH [RECURSIVE] ...)` must execute successfully.
//!
//! v313-96 / PR #4800 fixed the parser side by routing `LParen+WITH` through
//! `parse_with_select` in `parse_select_statement`. That fix strips the
//! `WithClause` from the AST so the executor lost the CTE definitions.
//!
//! V312-95 v2 layers an executor fix on top: a new `from_with_subquery`
//! field on `SelectStatement` preserves the `WithClause` through the FROM
//! dispatch, and `execute_select` materializes CTEs before running the
//! inner SELECT.
//!
//! Three regression tests:
//!   1. The exact failure mode reported in the issue: INSERT into t with a
//!      `FROM (WITH RECURSIVE ...)` subquery.
//!   2. The corresponding `SELECT ... FROM (WITH RECURSIVE ...)` form.
//!   3. A non-recursive guard so the FROM (WITH ...) path is exercised even
//!      without recursion.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn insert_from_recursive_cte_select_anchor() {
    // Verbatim shape from the issue body:
    //   INSERT INTO t SELECT x, x*10 FROM (WITH RECURSIVE s(x) AS (
    //     VALUES (1) UNION ALL SELECT x+1 FROM s WHERE x<3
    //   ) SELECT * FROM s) AS sub
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t (x INTEGER, y INTEGER)")
        .expect("CREATE TABLE t should succeed");
    x.execute(
        "INSERT INTO t SELECT x, x * 10 \
         FROM (WITH RECURSIVE s(x) AS ( \
             VALUES (1) \
             UNION ALL \
             SELECT x + 1 FROM s WHERE x < 3 \
         ) SELECT * FROM s) AS sub",
    )
    .expect("INSERT FROM (WITH RECURSIVE) subquery should execute");

    let r = x
        .execute("SELECT x, y FROM t ORDER BY x")
        .expect("SELECT FROM t should succeed");
    assert_eq!(r.rows.len(), 3, "expected 3 rows in t, got {}", r.rows.len());

    // Rows are (1, 10), (2, 20), (3, 30) ordered by x.
    let pairs: Vec<(i64, i64)> = r
        .rows
        .iter()
        .map(|row| match (&row[0], &row[1]) {
            (Value::Integer(a), Value::Integer(b)) => (*a, *b),
            other => panic!("expected (int, int), got {:?}", other),
        })
        .collect();
    assert_eq!(pairs, vec![(1, 10), (2, 20), (3, 30)]);
}

#[test]
fn select_from_recursive_cte_subquery_alone() {
    // SELECT ... FROM (WITH RECURSIVE walk(n) AS (...) SELECT * FROM walk) sub
    let mut x = fresh_mem();
    let r = x
        .execute(
            "SELECT * FROM ( \
                WITH RECURSIVE walk(n) AS ( \
                    VALUES (1) \
                    UNION ALL \
                    SELECT n + 1 FROM walk WHERE n < 5 \
                ) SELECT * FROM walk \
             ) sub",
        )
        .expect("SELECT FROM (WITH RECURSIVE) subquery should execute");
    assert_eq!(r.rows.len(), 5, "expected 5 rows from recursive walk");

    let values: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(v) => *v,
            other => panic!("expected integer, got {:?}", other),
        })
        .collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
}

#[test]
fn select_from_non_recursive_cte_subquery() {
    // Non-recursive guard: SELECT a, b FROM (WITH c AS (SELECT 1 AS a, 2 AS b) SELECT * FROM c) sub
    let mut x = fresh_mem();
    let r = x
        .execute(
            "SELECT a, b FROM ( \
                WITH c AS (SELECT 1 AS a, 2 AS b) \
                SELECT * FROM c \
             ) sub",
        )
        .expect("SELECT FROM (WITH) subquery should execute");
    assert_eq!(r.rows.len(), 1, "expected 1 row from non-recursive CTE");
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[0][1], Value::Integer(2));
}
