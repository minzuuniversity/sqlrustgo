//! Reproduction / regression tests for the v312-64c batch.
//!
//! Closes:
//!   - #4653 — `INSERT INTO ... RETURNING col` returns no rows
//!   - #4654 — AUTO_INCREMENT columns stay NULL after INSERT
//!   - #4658 — Combined: `INSERT ... RETURNING id` returns no id
//!
//! Both root causes (parser silently drops RETURNING, executor does not
//! fill AUTO_INCREMENT slots) live in `src/engine_dml.rs::execute_insert`
//! and `crates/parser/src/parser.rs::parse_insert`. The previous
//! `tests/integration/autoinc_test.rs` only counted rows and never
//! inspected id values, which is why CI did not catch #4654 — these
//! tests verify both count AND actual id values.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn row_int(row: &[Value], col: usize) -> i64 {
    match row.get(col) {
        Some(Value::Integer(n)) => *n,
        other => panic!("expected Integer at column {}, got {:?}", col, other),
    }
}

fn row_text(row: &[Value], col: usize) -> String {
    match row.get(col) {
        Some(Value::Text(s)) => s.clone(),
        other => panic!("expected Text at column {}, got {:?}", col, other),
    }
}

// ============================================================================
// Issue #4654 — AUTO_INCREMENT column population
// ============================================================================

#[test]
fn repro_4654_autoinc_first_row_assigns_one() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();
    x.execute("INSERT INTO t(name) VALUES ('alice')").unwrap();

    let r = x
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("SELECT");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(
        row_int(&r.rows[0], 0),
        1,
        "first AUTO_INCREMENT row must be id=1"
    );
    assert_eq!(row_text(&r.rows[0], 1), "alice");
}

#[test]
fn repro_4654_autoinc_sequential_three_rows() {
    // Exact reproduction of the issue body.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();
    x.execute("INSERT INTO t(name) VALUES ('alice'),('bob'),('carol')")
        .unwrap();

    let r = x
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("SELECT");
    assert_eq!(r.rows.len(), 3);
    assert_eq!(row_int(&r.rows[0], 0), 1);
    assert_eq!(row_int(&r.rows[1], 0), 2);
    assert_eq!(row_int(&r.rows[2], 0), 3);
    assert_eq!(row_text(&r.rows[0], 1), "alice");
    assert_eq!(row_text(&r.rows[1], 1), "bob");
    assert_eq!(row_text(&r.rows[2], 1), "carol");
}

#[test]
fn repro_4654_autoinc_after_delete_uses_max_plus_one() {
    // After deleting id=2, the next INSERT must allocate id=4 (MAX+1),
    // not id=2 (the freed slot). Matches MySQL InnoDB semantics.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();
    x.execute("INSERT INTO t(name) VALUES ('a'),('b'),('c')")
        .unwrap();
    x.execute("DELETE FROM t WHERE id = 2").unwrap();
    x.execute("INSERT INTO t(name) VALUES ('d')").unwrap();

    let r = x
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("SELECT");
    assert_eq!(r.rows.len(), 3, "a, c, d should remain");
    assert_eq!(row_int(&r.rows[0], 0), 1, "a=1");
    assert_eq!(row_int(&r.rows[1], 0), 3, "c=3");
    assert_eq!(row_int(&r.rows[2], 0), 4, "d=4 (MAX+1, not freed slot 2)");
}

#[test]
fn repro_4654_autoinc_explicit_id_preserved_then_resumes_after_max() {
    // User-supplied explicit ids are preserved; the next auto-generated
    // id resumes at MAX(existing) + 1.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();
    x.execute("INSERT INTO t(id, name) VALUES (5, 'x')")
        .unwrap();
    x.execute("INSERT INTO t(name) VALUES ('y')").unwrap();

    let r = x
        .execute("SELECT id, name FROM t ORDER BY id")
        .expect("SELECT");
    assert_eq!(row_int(&r.rows[0], 0), 5, "explicit id preserved");
    assert_eq!(row_int(&r.rows[1], 0), 6, "auto next is MAX(5)+1 = 6");
}

// ============================================================================
// Issue #4653 — INSERT ... RETURNING <expr-list>
// ============================================================================

#[test]
fn repro_4653_insert_returning_single_col() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();

    let r = x
        .execute("INSERT INTO t(name) VALUES ('alice') RETURNING id")
        .expect("INSERT RETURNING must execute");

    assert_eq!(
        r.rows.len(),
        1,
        "RETURNING must project exactly one row for a single-row INSERT"
    );
    assert_eq!(row_int(&r.rows[0], 0), 1);
}

#[test]
fn repro_4653_insert_returning_multi_col() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();

    let r = x
        .execute("INSERT INTO t(name) VALUES ('alice') RETURNING id, name")
        .expect("INSERT RETURNING must execute");

    assert_eq!(r.rows.len(), 1);
    assert_eq!(
        r.rows[0].len(),
        2,
        "RETURNING id, name must produce 2 columns"
    );
    assert_eq!(row_int(&r.rows[0], 0), 1);
    assert_eq!(row_text(&r.rows[0], 1), "alice");
}

#[test]
fn repro_4653_insert_without_returning_returns_no_rows() {
    // Backward compat: no RETURNING clause → empty rows vector
    // (legacy behaviour preserved by evaluate_returning_rows early-out).
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, name VARCHAR(20))")
        .unwrap();
    let r = x
        .execute("INSERT INTO t VALUES (1, 'alice')")
        .expect("plain INSERT");
    assert_eq!(r.rows.len(), 0, "no RETURNING → 0 rows in result");
    assert_eq!(r.affected_rows, 1);
}

// ============================================================================
// Issue #4658 — Combined: AUTO_INCREMENT + RETURNING id
// ============================================================================

#[test]
fn repro_4658_insert_returning_autoinc_id() {
    // Exact reproduction of the issue body.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();

    let r = x
        .execute("INSERT INTO t(name) VALUES ('dave') RETURNING id")
        .expect("INSERT RETURNING id must execute and return new id");

    assert_eq!(r.rows.len(), 1, "RETURNING must emit one row");
    assert_eq!(
        row_int(&r.rows[0], 0),
        1,
        "RETURNING id must surface the auto-generated id (1)"
    );
}

#[test]
fn repro_4658_insert_returning_autoinc_multi_rows() {
    // 3-row bulk INSERT ... RETURNING id must return (1,), (2,), (3,).
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();
    x.execute("INSERT INTO t(name) VALUES ('a'),('b'),('c') RETURNING id")
        .expect("bulk INSERT RETURNING id");

    let r = x.execute("SELECT id FROM t ORDER BY id").expect("SELECT");
    assert_eq!(r.rows.len(), 3);
    assert_eq!(row_int(&r.rows[0], 0), 1);
    assert_eq!(row_int(&r.rows[1], 0), 2);
    assert_eq!(row_int(&r.rows[2], 0), 3);
}

#[test]
fn repro_4658_insert_returning_autoinc_with_explicit_id() {
    // Mixed: explicit id rows + auto id rows, RETURNING both.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(20))")
        .unwrap();
    x.execute("INSERT INTO t(id, name) VALUES (10, 'x')")
        .unwrap();
    let r = x
        .execute("INSERT INTO t(name) VALUES ('y') RETURNING id")
        .expect("INSERT RETURNING id");
    assert_eq!(row_int(&r.rows[0], 0), 11, "MAX(10)+1 = 11");
}
