//! #4640 (short INSERT) — the VALUES list length must match the table's
//! column count, exactly like SQLite and MySQL.
//!
//! #4640 introduced a "short INSERT" that **padded** the missing columns
//! with NULL, on the premise that "SQLite allows `INSERT INTO t VALUES (10)`
//! into a two-column table". That premise is wrong: SQLite rejects it with
//! `table a has 2 columns but 1 values were supplied`, and so does MySQL
//! (`Column count doesn't match value count at row 1`). The repository's own
//! SQLite corpus says so — `crates/sqlrustgo_sqllogictest/testdata/
//! insert__test_insert_invalid.test` expects exactly that error for
//! `INSERT INTO a VALUES (1)`.
//!
//! Padding is silent data corruption: a typo'd INSERT drops the remaining
//! columns to NULL **and still succeeds**. The explicit-column form
//! (`INSERT INTO t (id) VALUES (1)`) is a different thing and stays legal —
//! naming a subset of columns is fine, the unnamed ones take their defaults.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn two_column_table(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("CREATE TABLE a(i INTEGER, j INTEGER)").unwrap();
}

#[test]
fn values_list_shorter_than_the_table_is_rejected() {
    let mut e = fresh();
    two_column_table(&mut e);
    let err = e
        .execute("INSERT INTO a VALUES (1)")
        .expect_err("a VALUES list shorter than the table must be rejected");
    let message = err.to_string();
    assert!(
        message.contains("has 2 columns") && message.contains("1 values were supplied"),
        "the error must name the table, its column count and the supplied count; got {message}"
    );

    // Nothing may have been inserted: a rejected statement leaves no row.
    let rows = e.execute("SELECT * FROM a").unwrap().rows;
    assert!(
        rows.is_empty(),
        "the rejected insert must not leave a row: {rows:?}"
    );
}

#[test]
fn values_list_longer_than_the_table_is_rejected() {
    let mut e = fresh();
    two_column_table(&mut e);
    let err = e
        .execute("INSERT INTO a VALUES (1, 2, 3)")
        .expect_err("a VALUES list longer than the table must be rejected");
    assert!(
        err.to_string().contains("has 2 columns"),
        "unexpected error: {err}"
    );
}

#[test]
fn values_list_of_the_same_length_is_accepted() {
    let mut e = fresh();
    two_column_table(&mut e);
    e.execute("INSERT INTO a VALUES (1, 2)").unwrap();
    let rows = e.execute("SELECT * FROM a").unwrap().rows;
    assert_eq!(rows.len(), 1, "the row must be inserted: {rows:?}");
}

#[test]
fn explicit_column_subset_stays_legal() {
    // Naming a subset of columns is a different thing and remains legal:
    // the unnamed columns take their defaults.
    let mut e = fresh();
    two_column_table(&mut e);
    e.execute("INSERT INTO a (i) VALUES (1)").unwrap();
    let rows = e.execute("SELECT i, j FROM a").unwrap().rows;
    assert_eq!(rows.len(), 1, "the row must be inserted: {rows:?}");
    assert!(
        matches!(rows[0][1], sqlrustgo::Value::Null),
        "the unnamed column takes its default (NULL); got {rows:?}"
    );
}

#[test]
fn insert_select_with_a_shorter_select_is_rejected() {
    let mut e = fresh();
    two_column_table(&mut e);
    e.execute("CREATE TABLE b(x INTEGER)").unwrap();
    e.execute("INSERT INTO b VALUES (1)").unwrap();
    let err = e
        .execute("INSERT INTO a SELECT x FROM b")
        .expect_err("a SELECT with fewer columns than the target must be rejected");
    assert!(
        err.to_string().contains("has 2 columns"),
        "unexpected error: {err}"
    );
}

#[test]
fn insert_select_with_the_same_column_count_is_accepted() {
    let mut e = fresh();
    two_column_table(&mut e);
    e.execute("CREATE TABLE b(x INTEGER, y INTEGER)").unwrap();
    e.execute("INSERT INTO b VALUES (1, 2)").unwrap();
    e.execute("INSERT INTO a SELECT x, y FROM b").unwrap();
    let rows = e.execute("SELECT i, j FROM a").unwrap().rows;
    assert_eq!(rows.len(), 1, "the row must be inserted: {rows:?}");
}
