//! Regression test: GROUP BY projection must honour the declared width.
//!
//! Issue #5432 — `SELECT cust FROM orders GROUP BY cust` and friends used to
//! return a row **wider** than the SELECT list. The aggregate row is laid
//! out as
//!
//! ```text
//! [group_key..., aggregate..., table_column_snapshot...]
//! ```
//!
//! but the grouped path used to short-circuit the re-projection whenever the
//! SELECT list had at most one column (`is_star_agg || select.columns.len() <= 1`)
//! and hand the **unprojected** row back. The declared columns, however, come
//! from the SELECT list — so the two widths disagreed.
//!
//! What that produced:
//!
//! * debug builds: `ResultSet::with_columns` hit its `debug_assert!` and the
//!   whole test binary aborted (`crates/executor/src/executor.rs:70`);
//! * release builds: the row was **silently truncated to the declared width**,
//!   so `SELECT count(*) FROM orders GROUP BY cust` returned the *group key*
//!   instead of the count. That is worse than a panic: a wrong answer, quietly.
//!
//! The fix removed the single-column short-circuit (single-column selects go
//! through the same re-projection as everything else) and made the `*` branch
//! take the **tail** of the aggregate row — the table's own columns, which sit
//! *after* the group keys and the aggregates.
//!
//! This file pins the *contract* rather than one query: after any grouped
//! SELECT, every row's width must equal the number of declared columns.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// alice: 2 rows / bob: 1 row / carol: 1 row.
fn orders() -> ExecutionEngine<MemoryStorage> {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE orders(cust varchar(20), amt int)")
        .unwrap();
    x.execute(
        "INSERT INTO orders VALUES ('alice', 100), ('alice', 50), ('bob', 200), ('carol', 300)",
    )
    .unwrap();
    x
}

/// Every row must be exactly as wide as the result set declares.
///
/// The declared width is observable through the number of columns the engine
/// reports; `ExecutorResult` exposes it as `columns.len()`.
fn assert_width_contract(x: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> Vec<Vec<Value>> {
    let r = x
        .execute(sql)
        .unwrap_or_else(|e| panic!("{} must execute: {}", sql, e));
    let declared = r.columns.len();
    assert!(declared > 0, "{} must declare at least one column", sql);
    for (i, row) in r.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            declared,
            "{}: row {} has width {} but the result set declares {} columns — \
             the aggregate row leaked past the projection",
            sql,
            i,
            row.len(),
            declared
        );
    }
    r.rows
}

fn texts(rows: &[Vec<Value>]) -> Vec<String> {
    rows.iter()
        .map(|r| match &r[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected Text, got {:?}", other),
        })
        .collect()
}

fn ints(rows: &[Vec<Value>]) -> Vec<i64> {
    rows.iter()
        .map(|r| match &r[0] {
            Value::Integer(n) => *n,
            other => panic!("expected Integer, got {:?}", other),
        })
        .collect()
}

#[test]
fn group_by_single_column_select_is_not_truncated() {
    let mut x = orders();
    let rows = assert_width_contract(&mut x, "SELECT cust FROM orders GROUP BY cust");
    assert_eq!(rows.len(), 3, "three distinct customers");
    let mut got = texts(&rows);
    got.sort();
    assert_eq!(got, vec!["alice", "bob", "carol"]);
}

#[test]
fn group_by_single_aggregate_select_returns_the_aggregate() {
    // The release-build failure mode: the row used to be the unprojected
    // aggregate row, truncated to one column => the *group key* came back
    // instead of the count.
    let mut x = orders();
    let rows = assert_width_contract(&mut x, "SELECT count(*) FROM orders GROUP BY cust");
    assert_eq!(rows.len(), 3, "three groups");
    let mut got = ints(&rows);
    got.sort();
    assert_eq!(
        got,
        vec![1, 1, 2],
        "alice has 2 rows, bob and carol have 1 each"
    );
}

#[test]
fn group_by_star_takes_the_table_columns_not_the_aggregate_row() {
    // `SELECT *` projects the table's own columns, which are the TAIL of the
    // aggregate row — after the group keys and the aggregates.
    let mut x = orders();
    let rows = assert_width_contract(&mut x, "SELECT * FROM orders GROUP BY cust");
    assert_eq!(rows.len(), 3, "three groups");
    assert_eq!(rows[0].len(), 2, "cust + amt");
    let mut names: Vec<String> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected Text, got {:?}", other),
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["alice", "bob", "carol"]);
}

#[test]
fn group_by_single_expression_select_is_not_truncated() {
    let mut x = orders();
    let rows = assert_width_contract(&mut x, "SELECT UPPER(cust) FROM orders GROUP BY cust");
    assert_eq!(rows.len(), 3);
    let mut got = texts(&rows);
    got.sort();
    assert_eq!(got, vec!["ALICE", "BOB", "CAROL"]);
}

#[test]
fn group_by_multi_column_select_keeps_its_width() {
    // The multi-column path was already correct; the single-column
    // short-circuit is what broke it. Keep both under the contract.
    let mut x = orders();
    let rows = assert_width_contract(&mut x, "SELECT cust, count(*) FROM orders GROUP BY cust");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].len(), 2);
    let mut got: Vec<(String, i64)> = rows
        .iter()
        .map(|r| {
            (
                match &r[0] {
                    Value::Text(s) => s.clone(),
                    other => panic!("expected Text, got {:?}", other),
                },
                match &r[1] {
                    Value::Integer(n) => *n,
                    other => panic!("expected Integer, got {:?}", other),
                },
            )
        })
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec![
            ("alice".to_string(), 2),
            ("bob".to_string(), 1),
            ("carol".to_string(), 1)
        ]
    );
}

#[test]
fn group_by_having_keeps_the_width_contract() {
    // #4657 (the sibling of #5432): HAVING filters groups *before* the
    // projection, so a HAVING clause must not change the row width either.
    let mut x = orders();
    let rows = assert_width_contract(
        &mut x,
        "SELECT cust FROM orders GROUP BY cust HAVING count(*) >= 2",
    );
    assert_eq!(rows.len(), 1, "only alice has 2+ rows");
    assert_eq!(texts(&rows), vec!["alice"]);
}
