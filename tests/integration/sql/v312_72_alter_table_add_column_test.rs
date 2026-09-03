//! V312-72 / Issue #4647 regression integration test:
//! `ALTER TABLE t ADD COLUMN c INT DEFAULT 99` must actually add the
//! column to the schema AND backfill existing rows.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_72_alter_add_column_with_default_backfills() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    x.execute("INSERT INTO t VALUES (1), (2)").unwrap();
    x.execute("ALTER TABLE t ADD COLUMN new_col INT DEFAULT 99")
        .unwrap();
    let res = x.execute("SELECT id, new_col FROM t ORDER BY id").unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(99));
    assert_eq!(res.rows[1][0], Value::Integer(2));
    assert_eq!(res.rows[1][1], Value::Integer(99));
}

#[test]
fn v312_72_alter_add_column_without_default_is_null() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("ALTER TABLE t ADD COLUMN new_col INT").unwrap();
    let res = x.execute("SELECT id, new_col FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert!(matches!(res.rows[0][1], Value::Null));
}

#[test]
fn v312_72_alter_add_column_select_star_shows_new_col() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    x.execute("INSERT INTO t VALUES (1), (2)").unwrap();
    x.execute("ALTER TABLE t ADD COLUMN new_col INT DEFAULT 99")
        .unwrap();
    let res = x.execute("SELECT * FROM t ORDER BY id").unwrap();
    assert_eq!(res.rows.len(), 2);
    for row in &res.rows {
        assert_eq!(row.len(), 2);
    }
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(99));
    assert_eq!(res.rows[1][0], Value::Integer(2));
    assert_eq!(res.rows[1][1], Value::Integer(99));
}
