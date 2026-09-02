//! V312-63 batch-1 parser/executor integration tests for issues
//! #4627 (TIMESTAMPDIFF unit), #4635 (CASE val WHEN NULL),
//! #4640 (short INSERT pad NULL), #4642 (UPSERT ON CONFLICT +
//! ODKU with row-context expression evaluation).
//!
//! Runs the public `ExecutionEngine::execute` path so the parser,
//! executor, and storage layers are all exercised end-to-end.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn extract_int(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> i64 {
    match &res.rows[row][col] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer at [{}][{}], got {:?}", row, col, other),
    }
}

fn extract_text(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> String {
    match &res.rows[row][col] {
        Value::Text(s) => s.clone(),
        other => panic!("expected Text at [{}][{}], got {:?}", row, col, other),
    }
}

// ---------------------------------------------------------------------
// #4627 TIMESTAMPDIFF(unit, ts1, ts2)
// ---------------------------------------------------------------------

#[test]
fn v312_63_timestampdiff_minute_returns_correct_delta() {
    let mut x = fresh();
    x.execute("CREATE TABLE events (id INTEGER, started_at TEXT, ended_at TEXT)")
        .unwrap();
    x.execute(
        "INSERT INTO events VALUES \
         (1, '2024-01-01 00:00:00', '2024-01-01 00:30:00'),\
         (2, '2024-01-01 12:00:00', '2024-01-01 13:15:00')",
    )
    .unwrap();
    let r = x
        .execute("SELECT TIMESTAMPDIFF(MINUTE, started_at, ended_at) FROM events")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(extract_int(&r, 0, 0), 30);
    assert_eq!(extract_int(&r, 1, 0), 75);
}

#[test]
fn v312_63_timestampdiff_year_returns_correct_delta() {
    let mut x = fresh();
    x.execute("CREATE TABLE s (dummy INTEGER)").unwrap();
    x.execute("INSERT INTO s VALUES (1)").unwrap();
    let r = x
        .execute("SELECT TIMESTAMPDIFF(YEAR, '2000-06-15', '2024-06-15') FROM s")
        .unwrap();
    assert_eq!(extract_int(&r, 0, 0), 24);
}

#[test]
fn v312_63_timestampdiff_hour_handles_seconds_input() {
    let mut x = fresh();
    x.execute("CREATE TABLE s (dummy INTEGER)").unwrap();
    x.execute("INSERT INTO s VALUES (1)").unwrap();
    let r = x
        .execute("SELECT TIMESTAMPDIFF(HOUR, '2024-01-01 00:00:00', '2024-01-01 05:30:00') FROM s")
        .unwrap();
    assert_eq!(extract_int(&r, 0, 0), 5);
}

// ---------------------------------------------------------------------
// #4635 CASE val WHEN NULL THEN ...
// ---------------------------------------------------------------------

#[test]
fn v312_63_case_value_when_null_matches_null_row() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER, x INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1, NULL), (2, 0), (3, 7)")
        .unwrap();
    let r = x
        .execute("SELECT id, CASE x WHEN NULL THEN 'is_null' ELSE 'not_null' END FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(extract_text(&r, 0, 1), "is_null");
    assert_eq!(extract_text(&r, 1, 1), "not_null");
    assert_eq!(extract_text(&r, 2, 1), "not_null");
}

#[test]
fn v312_63_case_value_when_null_mixed_with_literals() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (x INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (NULL), (0), (1)").unwrap();
    let r = x
        .execute("SELECT CASE x WHEN NULL THEN 'N' WHEN 0 THEN 'Z' WHEN 1 THEN 'O' END FROM t")
        .unwrap();
    assert_eq!(extract_text(&r, 0, 0), "N");
    assert_eq!(extract_text(&r, 1, 0), "Z");
    assert_eq!(extract_text(&r, 2, 0), "O");
}

// ---------------------------------------------------------------------
// #4640 short INSERT INTO ... VALUES (fewer columns than table)
// ---------------------------------------------------------------------

#[test]
fn v312_63_short_insert_values_pads_missing_columns_with_null() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER, name TEXT, age INTEGER)")
        .unwrap();
    // Only (id) supplied; name and age should default to NULL.
    x.execute("INSERT INTO t (id) VALUES (1), (2)").unwrap();
    let r = x
        .execute("SELECT id, name, age FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(extract_int(&r, 0, 0), 1);
    assert!(matches!(r.rows[0][1], Value::Null));
    assert!(matches!(r.rows[0][2], Value::Null));
    assert_eq!(extract_int(&r, 1, 0), 2);
    assert!(matches!(r.rows[1][1], Value::Null));
    assert!(matches!(r.rows[1][2], Value::Null));
}

// ---------------------------------------------------------------------
// #4642 UPSERT — ODKU with row-context evaluation
// ---------------------------------------------------------------------

#[test]
fn v312_63_odku_increments_existing_value() {
    let mut x = fresh();
    x.execute("CREATE TABLE counters (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    x.execute("INSERT INTO counters VALUES (1, 10)").unwrap();
    // v = v + 1 must read the EXISTING row's v (10), not NULL.
    x.execute("INSERT INTO counters VALUES (1, 999) ON DUPLICATE KEY UPDATE v = v + 1")
        .unwrap();
    let r = x.execute("SELECT v FROM counters").unwrap();
    assert_eq!(extract_int(&r, 0, 0), 11);
}

#[test]
fn v312_63_odku_set_literal() {
    let mut x = fresh();
    x.execute("CREATE TABLE counters (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    x.execute("INSERT INTO counters VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO counters VALUES (1, 999) ON DUPLICATE KEY UPDATE v = 42")
        .unwrap();
    let r = x.execute("SELECT v FROM counters").unwrap();
    assert_eq!(extract_int(&r, 0, 0), 42);
}

#[test]
fn v312_63_odku_insert_when_no_conflict() {
    let mut x = fresh();
    x.execute("CREATE TABLE counters (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    x.execute("INSERT INTO counters VALUES (1, 10) ON DUPLICATE KEY UPDATE v = v + 100")
        .unwrap();
    let r = x.execute("SELECT v FROM counters").unwrap();
    assert_eq!(extract_int(&r, 0, 0), 10);
}

// ---------------------------------------------------------------------
// #4642 UPSERT — SQLite/Postgres ON CONFLICT
// ---------------------------------------------------------------------

#[test]
fn v312_63_on_conflict_do_nothing_skips_conflict() {
    let mut x = fresh();
    x.execute("CREATE TABLE counters (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    x.execute("INSERT INTO counters VALUES (1, 10)").unwrap();
    // No error, no update.
    x.execute("INSERT INTO counters VALUES (1, 999) ON CONFLICT (id) DO NOTHING")
        .unwrap();
    let r = x.execute("SELECT v FROM counters").unwrap();
    assert_eq!(extract_int(&r, 0, 0), 10);
}

#[test]
fn v312_63_on_conflict_do_update_set_increments() {
    let mut x = fresh();
    x.execute("CREATE TABLE counters (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    x.execute("INSERT INTO counters VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO counters VALUES (1, 999) ON CONFLICT (id) DO UPDATE SET v = v + 5")
        .unwrap();
    let r = x.execute("SELECT v FROM counters").unwrap();
    assert_eq!(extract_int(&r, 0, 0), 15);
}

#[test]
fn v312_63_on_conflict_do_update_set_mixed_columns() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, x INTEGER, v INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 2, 3)").unwrap();
    x.execute(
        "INSERT INTO t VALUES (1, 999, 999) ON CONFLICT (id) DO UPDATE SET x = x + 1, v = 42",
    )
    .unwrap();
    let r = x.execute("SELECT x, v FROM t").unwrap();
    assert_eq!(extract_int(&r, 0, 0), 3);
    assert_eq!(extract_int(&r, 0, 1), 42);
}
