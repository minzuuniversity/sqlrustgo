//! UNION / INTERSECT / EXCEPT set operations integration tests.
//!
//! Closes task-2.1 of #3536 (Round 2 coverage plan).
//!
//! Coverage matrix:
//!
//! | Operation     | Top-level | Notes                            |
//! |---------------|-----------|----------------------------------|
//! | UNION (DISTINCT) | yes   | `union_all=false` dedup+sort    |
//! | UNION ALL     | yes       | no dedup                         |
//! | Nested UNION  | partial   | `UNION of UNION` works; nested via UNION wrapping |
//! | INTERSECT     | no        | Parser lacks `Statement::Intersect` |
//! | EXCEPT        | no        | Parser lacks `Statement::Except`    |
//! | Type coercion | implicit  | via `execute_select` Value unification |
//! | ORDER BY after UNION | no  | `UnionStatement` has no order_by/limit fields |
//! | LIMIT after UNION    | no  | Same — use subquery LIMIT instead |
//!
//! Tests for unsupported features are `#[ignore]`-ed with TODO comments so
//! they can be enabled when the parser/executor support lands.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn two_column_table(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("CREATE TABLE t1 (a INTEGER, b TEXT)").unwrap();
    e.execute("CREATE TABLE t2 (a INTEGER, b TEXT)").unwrap();
    e.execute("INSERT INTO t1 VALUES (1,'x'),(2,'y'),(3,'z')")
        .unwrap();
    e.execute("INSERT INTO t2 VALUES (2,'y'),(3,'z'),(4,'w')")
        .unwrap();
}

// ---------------------------------------------------------------------------
// UNION ALL — every row from left + every row from right
// ---------------------------------------------------------------------------

#[test]
fn union_all_concatenates() {
    let mut e = fresh();
    two_column_table(&mut e);

    let r = e
        .execute("SELECT a, b FROM t1 UNION ALL SELECT a, b FROM t2")
        .unwrap();
    assert_eq!(r.rows.len(), 6, "UNION ALL should concatenate 3 + 3 rows");
}

#[test]
fn union_all_preserves_duplicates() {
    let mut e = fresh();
    e.execute("CREATE TABLE u1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE u2 (v INTEGER)").unwrap();
    e.execute("INSERT INTO u1 VALUES (1),(2),(3)").unwrap();
    e.execute("INSERT INTO u2 VALUES (2),(3),(4)").unwrap();

    let r = e
        .execute("SELECT v FROM u1 UNION ALL SELECT v FROM u2")
        .unwrap();
    // 3 + 3 rows; values: 1,2,3,2,3,4
    let values: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(i) => *i,
            other => panic!("expected Integer, got {:?}", other),
        })
        .collect();
    assert_eq!(values.len(), 6);
    assert_eq!(values.iter().filter(|v| **v == 2).count(), 2);
    assert_eq!(values.iter().filter(|v| **v == 3).count(), 2);
}

// ---------------------------------------------------------------------------
// UNION (DISTINCT) — dedup + sort
// ---------------------------------------------------------------------------

#[test]
fn union_distinct_dedupes() {
    let mut e = fresh();
    two_column_table(&mut e);

    let r = e
        .execute("SELECT a, b FROM t1 UNION SELECT a, b FROM t2")
        .unwrap();
    // t1 = {1x,2y,3z}, t2 = {2y,3z,4w} → distinct = {1x,2y,3z,4w}
    assert_eq!(r.rows.len(), 4, "UNION DISTINCT should produce 4 rows");
}

#[test]
fn union_distinct_sorts() {
    let mut e = fresh();
    e.execute("CREATE TABLE s1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE s2 (v INTEGER)").unwrap();
    e.execute("INSERT INTO s1 VALUES (3),(1),(2)").unwrap();
    e.execute("INSERT INTO s2 VALUES (5),(2),(4)").unwrap();

    let r = e
        .execute("SELECT v FROM s1 UNION SELECT v FROM s2")
        .unwrap();
    let values: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(i) => *i,
            other => panic!("expected Integer, got {:?}", other),
        })
        .collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5], "UNION DISTINCT must sort");
}

#[test]
fn union_distinct_with_all_overlap_returns_single_set() {
    let mut e = fresh();
    e.execute("CREATE TABLE o1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE o2 (v INTEGER)").unwrap();
    e.execute("INSERT INTO o1 VALUES (1),(2),(3)").unwrap();
    e.execute("INSERT INTO o2 VALUES (1),(2),(3)").unwrap();

    let r = e
        .execute("SELECT v FROM o1 UNION SELECT v FROM o2")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "full overlap → 3 distinct rows");
}

// ---------------------------------------------------------------------------
// Nested UNION — UNION of UNION
// ---------------------------------------------------------------------------

#[test]
fn nested_union_three_legs() {
    let mut e = fresh();
    e.execute("CREATE TABLE n1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE n2 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE n3 (v INTEGER)").unwrap();
    e.execute("INSERT INTO n1 VALUES (1),(2)").unwrap();
    e.execute("INSERT INTO n2 VALUES (2),(3)").unwrap();
    e.execute("INSERT INTO n3 VALUES (3),(4)").unwrap();

    // (n1 ∪ n2) ∪ n3 — chained left-associative parse.
    let r = e
        .execute("SELECT v FROM n1 UNION SELECT v FROM n2 UNION SELECT v FROM n3")
        .unwrap();
    let values: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(i) => *i,
            other => panic!("expected Integer, got {:?}", other),
        })
        .collect();
    assert_eq!(values, vec![1, 2, 3, 4]);
}

#[test]
fn nested_union_mixed_all_and_distinct() {
    let mut e = fresh();
    e.execute("CREATE TABLE m1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE m2 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE m3 (v INTEGER)").unwrap();
    e.execute("INSERT INTO m1 VALUES (1),(2)").unwrap();
    e.execute("INSERT INTO m2 VALUES (2),(3)").unwrap();
    e.execute("INSERT INTO m3 VALUES (3),(3),(4)").unwrap();

    // (m1 UNION ALL m2) UNION DISTINCT m3 — left side keeps dupes,
    // outer UNION dedups across the combined set.
    let r = e
        .execute(
            "SELECT v FROM m1 UNION ALL SELECT v FROM m2 \
             UNION SELECT v FROM m3",
        )
        .unwrap();
    let values: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(i) => *i,
            other => panic!("expected Integer, got {:?}", other),
        })
        .collect();
    // m1+m2 = {1,2,2,3}; UNION DISTINCT m3={3,3,4} → {1,2,3,4}
    assert_eq!(values, vec![1, 2, 3, 4]);
}

// ---------------------------------------------------------------------------
// Type coercion across the UNION boundary
// ---------------------------------------------------------------------------

#[test]
fn union_with_mixed_numeric_types() {
    let mut e = fresh();
    e.execute("CREATE TABLE p1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE p2 (v FLOAT)").unwrap();
    e.execute("INSERT INTO p1 VALUES (1),(2)").unwrap();
    e.execute("INSERT INTO p2 VALUES (2.5),(3.0)").unwrap();

    let r = e
        .execute("SELECT v FROM p1 UNION ALL SELECT v FROM p2")
        .unwrap();
    // Should not error; we don't pin the exact Value variant — just
    // confirm row count and that each row has one numeric column.
    assert_eq!(r.rows.len(), 4);
    for row in &r.rows {
        assert!(matches!(&row[0], Value::Integer(_) | Value::Float(_)));
    }
}

// ---------------------------------------------------------------------------
// ORDER BY + LIMIT interaction
//
// The current parser produces a `UnionStatement { left, right, union_all }`
// without order_by/limit fields, so a top-level
// `... UNION ... ORDER BY ... LIMIT ...` will fail to parse. Workaround:
// wrap the UNION in a subquery / SELECT, since the executor only supports
// ORDER BY+LIMIT inside the SELECT branches, not on the UNION itself.
// ---------------------------------------------------------------------------

#[test]
fn order_by_inside_union_branches() {
    let mut e = fresh();
    e.execute("CREATE TABLE o1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE o2 (v INTEGER)").unwrap();
    e.execute("INSERT INTO o1 VALUES (3),(1),(2)").unwrap();
    e.execute("INSERT INTO o2 VALUES (6),(4),(5)").unwrap();

    // ORDER BY in each leg of the UNION — supported by execute_select.
    let r = e
        .execute(
            "SELECT v FROM o1 ORDER BY v UNION ALL \
             SELECT v FROM o2 ORDER BY v",
        )
        .unwrap();
    let values: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(i) => *i,
            other => panic!("expected Integer, got {:?}", other),
        })
        .collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5, 6]);
}

// ---------------------------------------------------------------------------
// INTERSECT — currently UNSUPPORTED in the parser.
//
// Token::Intersect exists but the parser only matches Token::Union in
// parse_select_statement's chain loop. Re-enable when parser gains
// Statement::Intersect support.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "Parser lacks Statement::Intersect — V312-30 reconciliation"]
fn intersect_returns_common_rows() {
    let mut e = fresh();
    e.execute("CREATE TABLE i1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE i2 (v INTEGER)").unwrap();
    e.execute("INSERT INTO i1 VALUES (1),(2),(3)").unwrap();
    e.execute("INSERT INTO i2 VALUES (2),(3),(4)").unwrap();

    let r = e
        .execute("SELECT v FROM i1 INTERSECT SELECT v FROM i2")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
}

// ---------------------------------------------------------------------------
// EXCEPT — currently UNSUPPORTED in the parser.
// ---------------------------------------------------------------------------

#[test]
fn except_returns_left_minus_right() {
    let mut e = fresh();
    e.execute("CREATE TABLE x1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE x2 (v INTEGER)").unwrap();
    e.execute("INSERT INTO x1 VALUES (1),(2),(3)").unwrap();
    e.execute("INSERT INTO x2 VALUES (2),(3),(4)").unwrap();

    let r = e
        .execute("SELECT v FROM x1 EXCEPT SELECT v FROM x2")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(1));
}

// ---------------------------------------------------------------------------
// ORDER BY / LIMIT applied AFTER a top-level UNION (V310-06 PR2 / Issue #3723 C-2c).
// Parser now lifts trailing ORDER BY/LIMIT/OFFSET onto UnionStatement.
#[test]
fn order_by_after_top_level_union() {
    let mut e = fresh();
    e.execute("CREATE TABLE lo1 (v INTEGER)").unwrap();
    e.execute("CREATE TABLE lo2 (v INTEGER)").unwrap();
    e.execute("INSERT INTO lo1 VALUES (3),(1),(2)").unwrap();
    e.execute("INSERT INTO lo2 VALUES (6),(4),(5)").unwrap();

    let r = e
        .execute("SELECT v FROM lo1 UNION SELECT v FROM lo2 ORDER BY v LIMIT 3")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
}
