//! Issues #4569 / #4570 — UNIQUE and FOREIGN KEY constraints were parsed
//! but never enforced.
//!
//! Root cause: `execute_create_table` hard-coded `foreign_keys: vec![]` and
//! `unique_constraints: vec![]` in the storage `TableInfo`, so the INSERT
//! path's `validate_foreign_keys` / duplicate checks never fired. On top of
//! that, a column-level `UNIQUE` modifier (`a INT UNIQUE`) was consumed and
//! silently dropped by the parser, and the INSERT duplicate detector only
//! compared primary-key columns.
//!
//! Fix pinned by these tests:
//!   1. Table-level `UNIQUE (col)` / `CONSTRAINT name UNIQUE (col)` reject
//!      duplicate inserts with `Duplicate entry '<v>' for key '<key>'`.
//!   2. Column-level `col INT UNIQUE` is enforced too.
//!   3. Multiple NULLs are still allowed under UNIQUE (SQL standard).
//!   4. INSERT IGNORE skips; ON DUPLICATE KEY UPDATE fires on a UNIQUE hit.
//!   5. Table-level and column-level FOREIGN KEY reject orphan inserts;
//!      NULL FK values and valid parents are allowed.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

// ---------- #4569: UNIQUE ----------

#[test]
fn table_level_unique_rejects_duplicate() {
    let mut e = engine();
    e.execute("CREATE TABLE t(a INT, b TEXT, UNIQUE (a))")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'x')").unwrap();
    let err = e.execute("INSERT INTO t VALUES (1, 'y')").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Duplicate entry") && msg.contains("for key"),
        "expected duplicate-key error, got: {msg}"
    );
    // Distinct values still insert fine.
    e.execute("INSERT INTO t VALUES (2, 'y')").unwrap();
}

#[test]
fn named_unique_constraint_rejects_duplicate() {
    let mut e = engine();
    e.execute("CREATE TABLE t(a INT, CONSTRAINT uk_a UNIQUE (a))")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    let err = e.execute("INSERT INTO t VALUES (1)").unwrap_err();
    assert!(
        err.to_string().contains("uk_a"),
        "error must name the unique key, got: {err}"
    );
}

#[test]
fn column_level_unique_rejects_duplicate() {
    let mut e = engine();
    e.execute("CREATE TABLE t(email VARCHAR(100) UNIQUE)")
        .unwrap();
    e.execute("INSERT INTO t VALUES ('a@x')").unwrap();
    let err = e.execute("INSERT INTO t VALUES ('a@x')").unwrap_err();
    assert!(
        err.to_string().contains("Duplicate entry"),
        "column-level UNIQUE must be enforced, got: {err}"
    );
}

#[test]
fn unique_allows_multiple_nulls() {
    let mut e = engine();
    e.execute("CREATE TABLE t(a INT UNIQUE)").unwrap();
    e.execute("INSERT INTO t VALUES (NULL)").unwrap();
    e.execute("INSERT INTO t VALUES (NULL)").unwrap();
    let r = e.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(r.rows[0][0], sqlrustgo_types::Value::Integer(2)));
}

#[test]
fn insert_ignore_skips_unique_duplicate() {
    let mut e = engine();
    e.execute("CREATE TABLE t(a INT UNIQUE, b TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'x')").unwrap();
    e.execute("INSERT IGNORE INTO t VALUES (1, 'y')").unwrap();
    let r = e.execute("SELECT b FROM t WHERE a = 1").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(
        matches!(&r.rows[0][0], sqlrustgo_types::Value::Text(s) if s == "x"),
        "IGNORE must keep the original row"
    );
}

#[test]
fn odku_fires_on_unique_conflict() {
    let mut e = engine();
    e.execute("CREATE TABLE t(a INT UNIQUE, b TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'x')").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'y') ON DUPLICATE KEY UPDATE b = 'updated'")
        .unwrap();
    let r = e.execute("SELECT b FROM t").unwrap();
    assert_eq!(r.rows.len(), 1, "ODKU must update, not insert");
    assert!(matches!(&r.rows[0][0], sqlrustgo_types::Value::Text(s) if s == "updated"));
}

// ---------- #4570: FOREIGN KEY ----------

#[test]
fn table_level_fk_rejects_orphan_insert() {
    let mut e = engine();
    e.execute("CREATE TABLE dept(id INT PRIMARY KEY, name TEXT)")
        .unwrap();
    e.execute("CREATE TABLE emp(id INT, dept_id INT, FOREIGN KEY (dept_id) REFERENCES dept(id))")
        .unwrap();
    e.execute("INSERT INTO dept VALUES (1, 'eng')").unwrap();
    // Valid parent → OK.
    e.execute("INSERT INTO emp VALUES (10, 1)").unwrap();
    // Missing parent → error.
    let err = e.execute("INSERT INTO emp VALUES (11, 99)").unwrap_err();
    assert!(
        err.to_string().contains("Foreign key constraint"),
        "expected FK violation, got: {err}"
    );
}

#[test]
fn column_level_fk_rejects_orphan_insert() {
    let mut e = engine();
    e.execute("CREATE TABLE dept(id INT PRIMARY KEY)").unwrap();
    e.execute("CREATE TABLE emp(id INT, dept_id INT REFERENCES dept(id))")
        .unwrap();
    e.execute("INSERT INTO dept VALUES (1)").unwrap();
    e.execute("INSERT INTO emp VALUES (10, 1)").unwrap();
    let err = e.execute("INSERT INTO emp VALUES (11, 99)").unwrap_err();
    assert!(
        err.to_string().contains("Foreign key constraint"),
        "column-level REFERENCES must be enforced, got: {err}"
    );
}

#[test]
fn fk_allows_null_child_value() {
    let mut e = engine();
    e.execute("CREATE TABLE dept(id INT PRIMARY KEY)").unwrap();
    e.execute("CREATE TABLE emp(id INT, dept_id INT, FOREIGN KEY (dept_id) REFERENCES dept(id))")
        .unwrap();
    e.execute("INSERT INTO emp VALUES (1, NULL)").unwrap();
    let r = e.execute("SELECT COUNT(*) FROM emp").unwrap();
    assert!(matches!(r.rows[0][0], sqlrustgo_types::Value::Integer(1)));
}
