//! V312-75 / Issue #4636 regression integration test:
//! correlated scalar subquery in the SELECT list
//! (`SELECT (SELECT b.val FROM b WHERE b.id = a.id) AS bval FROM a`)
//! must bind outer references per row and return the matching value
//! instead of an empty column or a "from-table form not yet
//! implemented" runtime error.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn seed(x: &mut ExecutionEngine<MemoryStorage>) {
    x.execute("CREATE TABLE a (id INT, val INT)").unwrap();
    x.execute("CREATE TABLE b (id INT, val INT)").unwrap();
    x.execute("INSERT INTO a VALUES (1, 100), (2, 200)").unwrap();
    x.execute("INSERT INTO b VALUES (1, 10), (2, 20)").unwrap();
}

fn extract_int(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> i64 {
    match &res.rows[row][col] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer at [{}][{}], got {:?}", row, col, other),
    }
}

/// The exact repro from issue #4636: qualified inner (`b.val`) and
/// qualified outer (`a.id`) references.
#[test]
fn v312_75_correlated_scalar_subquery_qualified() {
    let mut x = fresh();
    seed(&mut x);
    let res = x
        .execute("SELECT id, (SELECT b.val FROM b WHERE b.id = a.id) AS bval FROM a")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_int(&res, 0, 1), 10);
    assert_eq!(extract_int(&res, 1, 0), 2);
    assert_eq!(extract_int(&res, 1, 1), 20);
}

/// Unqualified inner column (`val`) with a qualified outer reference
/// (`a.id`) — the inner column must NOT be substituted with the outer
/// row's value of the same-named column (shadow protection).
#[test]
fn v312_75_correlated_scalar_subquery_unqualified_inner() {
    let mut x = fresh();
    seed(&mut x);
    let res = x
        .execute("SELECT id, (SELECT val FROM b WHERE id = a.id) AS v FROM a")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(extract_int(&res, 0, 1), 10);
    assert_eq!(extract_int(&res, 1, 1), 20);
}

/// No matching inner row → NULL (standard scalar-subquery semantics).
#[test]
fn v312_75_correlated_scalar_subquery_no_match_yields_null() {
    let mut x = fresh();
    seed(&mut x);
    let res = x
        .execute("SELECT id, (SELECT b.val FROM b WHERE b.id = 999) AS nm FROM a")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    for row in 0..2 {
        assert!(
            matches!(res.rows[row][1], Value::Null),
            "expected NULL at row {}, got {:?}",
            row,
            res.rows[row][1]
        );
    }
}

/// Uncorrelated aggregate form (previously the exact shape that
/// surfaced "not yet implemented") now executes and repeats per row.
#[test]
fn v312_75_uncorrelated_aggregate_scalar_subquery() {
    let mut x = fresh();
    seed(&mut x);
    let res = x
        .execute("SELECT id, (SELECT MAX(val) FROM b) AS m FROM a")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(extract_int(&res, 0, 1), 20);
    assert_eq!(extract_int(&res, 1, 1), 20);
}

/// Literal (no-table) form keeps working — no regression on v312-67.
#[test]
fn v312_75_literal_scalar_subquery_regression() {
    let mut x = fresh();
    seed(&mut x);
    let res = x.execute("SELECT (SELECT 1) AS x, (SELECT 1 + 2) AS y").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_int(&res, 0, 1), 3);
}

/// Correlated EXISTS in WHERE keeps working (Step 1.5 machinery).
#[test]
fn v312_75_correlated_exists_regression() {
    let mut x = fresh();
    seed(&mut x);
    let res = x
        .execute("SELECT id FROM a WHERE EXISTS (SELECT 1 FROM b WHERE b.id = a.id)")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_int(&res, 1, 0), 2);
}
