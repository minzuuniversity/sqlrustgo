//! V312-75 / Issue #4700 regression integration test:
//! `CREATE TRIGGER ... BEFORE UPDATE OF <col_list> ON ...` — column-level
//! UPDATE trigger (SQLite/MySQL standard). Before this fix, the parser
//! rejected the `OF` keyword with
//! `Parse error: Expected On, got Of`. After this fix, the syntax is
//! accepted and the column list is recorded on the catalog
//! (`TriggerInfo.update_columns`). Firing-side column matching is left
//! to the trigger executor and is not exercised here — the regression
//! is at the parser/catalog boundary.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_75_update_of_single_column_trigger_creates() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT, note TEXT)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10, 'init')").unwrap();
    // BEFORE UPDATE OF val — column-level trigger must be accepted.
    x.execute(
        "CREATE TRIGGER tr BEFORE UPDATE OF val ON t \
         FOR EACH ROW BEGIN UPDATE t SET note = 'updated' WHERE id = OLD.id; END",
    )
    .unwrap();
    // Trigger is recorded; UPDATE statement parses and runs (the
    // trigger firing-side match is out of scope for the parser fix).
    x.execute("UPDATE t SET val = 20 WHERE id = 1").unwrap();
}

#[test]
fn v312_75_update_of_multiple_columns_trigger_creates() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT, note TEXT)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10, 'a')").unwrap();
    // AFTER UPDATE OF val, note — multi-column trigger.
    x.execute(
        "CREATE TRIGGER tr AFTER UPDATE OF val, note ON t \
         FOR EACH ROW BEGIN SELECT 1; END",
    )
    .unwrap();
    x.execute("UPDATE t SET note = 'b' WHERE id = 1").unwrap();
}

#[test]
fn v312_75_update_of_with_other_events_still_works() {
    // Regression: INSERT and DELETE triggers must still parse
    // correctly after adding the UPDATE OF branch.
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    x.execute(
        "CREATE TRIGGER ti BEFORE INSERT ON t \
         FOR EACH ROW BEGIN SELECT 1; END",
    )
    .unwrap();
    x.execute(
        "CREATE TRIGGER tu BEFORE UPDATE ON t \
         FOR EACH ROW BEGIN SELECT 1; END",
    )
    .unwrap();
    x.execute(
        "CREATE TRIGGER td BEFORE DELETE ON t \
         FOR EACH ROW BEGIN SELECT 1; END",
    )
    .unwrap();
}

#[test]
fn v312_75_insert_of_errors() {
    // `INSERT OF col` is not a valid event — only UPDATE accepts OF.
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    let result = x.execute(
        "CREATE TRIGGER tr BEFORE INSERT OF val ON t \
         FOR EACH ROW BEGIN SELECT 1; END",
    );
    assert!(result.is_err(), "INSERT OF should be a parse error");
}

#[test]
fn v312_75_update_of_empty_errors() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    let result = x.execute(
        "CREATE TRIGGER tr BEFORE UPDATE OF ON t \
         FOR EACH ROW BEGIN SELECT 1; END",
    );
    assert!(result.is_err(), "UPDATE OF with no columns should error");
}
