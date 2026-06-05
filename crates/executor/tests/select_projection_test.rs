//! SELECT Projection Tests
//!
//! Sprint 2 follow-up: `execute_select` currently returns the accumulated
//! row set as-is. The real TPC-H Q7/Q8/Q9 use SELECT with expressions like
//! `EXTRACT(YEAR FROM o_orderdate) AS o_year` and `l_extendedprice * (1 -
//! l_discount) AS volume`, and they expect the result to contain only
//! the projected columns (or all columns for `SELECT *`).
//!
//! These tests assert on the projected shape (column count, column values)
//! so the implementation can be validated end-to-end.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_select_star_returns_all_columns() {
    // SELECT * should return every column from the table (or accumulated
    // join schema). For a single-table query, that's the table columns.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'a'), (2, 'b')")
        .unwrap();

    let result = engine.execute("SELECT * FROM t").unwrap();

    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].len(), 2, "SELECT * should yield all columns");
}

#[test]
fn test_select_specific_column_projects_one() {
    // `SELECT o_orderkey FROM t` should yield rows with a single column
    // (o_orderkey's value), not the full table.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE orders (o_orderkey INTEGER, o_orderdate TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO orders VALUES (1, '1995-03-15'), (2, '1995-11-20')")
        .unwrap();

    let result = engine.execute("SELECT o_orderkey FROM orders").unwrap();

    assert_eq!(result.rows.len(), 2);
    assert_eq!(
        result.rows[0].len(),
        1,
        "SELECT o_orderkey should yield 1 column, got row {:?}",
        result.rows[0]
    );
    assert_eq!(result.rows[0][0], Value::Integer(1));
    assert_eq!(result.rows[1][0], Value::Integer(2));
}

#[test]
fn test_select_extract_year_projects_expression() {
    // TPC-H Q7/Q8/Q9: SELECT EXTRACT(YEAR FROM o_orderdate) AS o_year
    // should yield rows with a single year string.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE orders (o_orderkey INTEGER, o_orderdate TEXT)")
        .unwrap();
    engine
        .execute(
            "INSERT INTO orders VALUES \
             (1, '1995-03-15'), \
             (2, '1995-11-20'), \
             (3, '1996-01-01')",
        )
        .unwrap();

    let result = engine
        .execute("SELECT EXTRACT(YEAR FROM o_orderdate) AS o_year FROM orders")
        .unwrap();

    assert_eq!(result.rows.len(), 3);
    assert_eq!(
        result.rows[0].len(),
        1,
        "SELECT EXTRACT should project 1 column"
    );
    assert_eq!(result.rows[0][0], Value::Text("1995".to_string()));
    assert_eq!(result.rows[2][0], Value::Text("1996".to_string()));
}

#[test]
fn test_select_arithmetic_projects_expression() {
    // TPC-H Q7: SELECT l_extendedprice * (1 - l_discount) AS volume FROM lineitem
    // should yield the computed volume, not the raw columns.
    //
    // Note: we use INTEGER columns here (not FLOAT) because the in-memory
    // storage engine currently records the third declared column with
    // INTEGER type even when FLOAT is requested. That pre-existing storage
    // issue is out of scope for this projection test; the projection logic
    // itself is what we're verifying.
    let mut engine = create_engine();
    engine
        .execute(
            "CREATE TABLE lineitem (l_orderkey INTEGER, l_extendedprice INTEGER, l_discount INTEGER)",
        )
        .unwrap();
    engine
        .execute("INSERT INTO lineitem VALUES (1, 100, 10), (2, 200, 20)")
        .unwrap();

    let result = engine
        .execute("SELECT l_extendedprice * (1 - l_discount) AS volume FROM lineitem")
        .unwrap();

    assert_eq!(result.rows.len(), 2);
    assert_eq!(
        result.rows[0].len(),
        1,
        "arithmetic expression should project 1 column"
    );
    // 100 * (1 - 10) = -900 (Integer arithmetic, not Q7's real formula)
    assert_eq!(result.rows[0][0], Value::Integer(-900));
}
