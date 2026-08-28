//! Issue #4571 — ALTER TABLE ADD COLUMN discarded the DEFAULT clause and
//! never backfilled existing rows.
//!
//! Root causes:
//!   1. The parser hard-coded `nullable = true, default_value = None` for
//!      ADD COLUMN, so `ADD COLUMN c INT DEFAULT 5` lost its DEFAULT.
//!   2. `execute_alter_table` discarded the parsed `default_value` field.
//!   3. `MemoryStorage::add_column` extended only the schema — existing
//!      records stayed short, so `SELECT *` after the ALTER returned rows
//!      missing the new column's cell.
//!
//! Fix pinned by these tests:
//!   1. `ADD COLUMN ... DEFAULT <lit>` backfills existing rows with the
//!      default (numeric / string / NULL literal forms).
//!   2. Without DEFAULT, existing rows see NULL.
//!   3. `SELECT *` after ADD COLUMN returns full-width rows.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn setup_two_rows(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("CREATE TABLE t(a INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1), (2)").unwrap();
}

#[test]
fn add_column_default_backfills_existing_rows() {
    let mut e = engine();
    setup_two_rows(&mut e);
    e.execute("ALTER TABLE t ADD COLUMN b INT DEFAULT 7")
        .unwrap();
    let r = e.execute("SELECT a, b FROM t").unwrap();
    assert_eq!(r.rows.len(), 2);
    for row in &r.rows {
        assert_eq!(row.len(), 2, "each row must have both columns");
        assert!(matches!(row[0], Value::Integer(1) | Value::Integer(2)));
        assert!(
            matches!(row[1], Value::Integer(7)),
            "b must backfill 7, got {:?}",
            row[1]
        );
    }
}

#[test]
fn add_column_string_default_backfills() {
    let mut e = engine();
    setup_two_rows(&mut e);
    e.execute("ALTER TABLE t ADD COLUMN tag TEXT DEFAULT 'x'")
        .unwrap();
    let r = e.execute("SELECT tag FROM t").unwrap();
    for row in &r.rows {
        assert!(
            matches!(&row[0], Value::Text(s) if s == "x"),
            "text default must backfill, got {:?}",
            row[0]
        );
    }
}

#[test]
fn add_column_without_default_backfills_null() {
    let mut e = engine();
    setup_two_rows(&mut e);
    e.execute("ALTER TABLE t ADD COLUMN b TEXT").unwrap();
    let r = e.execute("SELECT b FROM t").unwrap();
    for row in &r.rows {
        assert!(
            matches!(row[0], Value::Null),
            "no default → NULL, got {:?}",
            row[0]
        );
    }
}

#[test]
fn select_star_full_width_after_add_column() {
    let mut e = engine();
    setup_two_rows(&mut e);
    e.execute("ALTER TABLE t ADD COLUMN b INT DEFAULT 0")
        .unwrap();
    let r = e.execute("SELECT * FROM t").unwrap();
    for row in &r.rows {
        assert_eq!(row.len(), 2, "SELECT * must include the new column");
    }
    // New inserts also fill the new column.
    e.execute("INSERT INTO t VALUES (3, 9)").unwrap();
    let r = e.execute("SELECT a, b FROM t WHERE a = 3").unwrap();
    assert!(matches!(r.rows[0][1], Value::Integer(9)));
}

#[test]
fn add_column_not_null_default_parses() {
    let mut e = engine();
    setup_two_rows(&mut e);
    // NOT NULL + DEFAULT must both parse and apply.
    e.execute("ALTER TABLE t ADD COLUMN c INT NOT NULL DEFAULT 3")
        .unwrap();
    let r = e.execute("SELECT c FROM t").unwrap();
    for row in &r.rows {
        assert!(matches!(row[0], Value::Integer(3)), "got {:?}", row[0]);
    }
}
