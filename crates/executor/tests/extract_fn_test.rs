//! EXTRACT(YEAR/MONTH/DAY FROM date) Tests
//!
//! Sprint: TPC-H Q7/Q8/Q9 all use `EXTRACT(YEAR FROM o_orderdate) AS o_year`
//! in the SELECT and GROUP BY clauses. Without this fix, the parser
//! fails to recognize the `FROM` keyword inside the function call, and
//! the executor's `evaluate_expression` would return Null for the
//! FunctionCall shape.
//!
//! These tests exercise EXTRACT inside WHERE (which routes through
//! `evaluate_expression` and `eval_predicate`) to verify the dispatch
//! works. SELECT projection for non-trivial expressions is a separate
//! follow-up; until then, these tests assert on row counts that
//! depend on EXTRACT's predicate result.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_extract_year_in_where_filter() {
    // EXTRACT inside WHERE — the predicate path goes through
    // evaluate_expression which now recognizes FunctionCall("EXTRACT", ...).
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE orders (o_orderkey INTEGER, o_orderdate TEXT)")
        .unwrap();
    engine
        .execute(
            "INSERT INTO orders VALUES \
             (1, '1995-03-15'), \
             (2, '1995-11-20'), \
             (3, '1996-01-01'), \
             (4, '1996-08-30')",
        )
        .unwrap();

    let result = engine
        .execute("SELECT o_orderkey FROM orders WHERE EXTRACT(YEAR FROM o_orderdate) = '1995'")
        .unwrap();

    // 2 rows in 1995.
    assert_eq!(
        result.rows.len(),
        2,
        "WHERE EXTRACT(YEAR FROM o_orderdate) = '1995' should keep 2 rows, got {:?}",
        result.rows
    );
}

#[test]
fn test_extract_year_no_match() {
    // No rows match the year — verifies the predicate evaluates correctly
    // even when the result is empty.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE orders (o_orderkey INTEGER, o_orderdate TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO orders VALUES (1, '1995-03-15'), (2, '1996-08-30')")
        .unwrap();

    let result = engine
        .execute("SELECT o_orderkey FROM orders WHERE EXTRACT(YEAR FROM o_orderdate) = '1999'")
        .unwrap();

    assert_eq!(result.rows.len(), 0);
}

#[test]
fn test_extract_month_in_where() {
    // EXTRACT(MONTH FROM ...) — month is the 6-7 char slice of an
    // ISO-8601 date. Verifies the field dispatch is not hard-coded to YEAR.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE orders (o_orderkey INTEGER, o_orderdate TEXT)")
        .unwrap();
    engine
        .execute(
            "INSERT INTO orders VALUES \
             (1, '1995-03-15'), \
             (2, '1995-06-20'), \
             (3, '1996-03-01')",
        )
        .unwrap();

    let result = engine
        .execute("SELECT o_orderkey FROM orders WHERE EXTRACT(MONTH FROM o_orderdate) = '03'")
        .unwrap();

    // Rows 1 and 3 are in March.
    assert_eq!(result.rows.len(), 2);
}
