//! V312-71 / Issue #4664 regression integration test:
//! `sqlite_master` (a.k.a. `sqlite_schema`) system table must be
//! synthesized at query time, returning one row per user table.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_71_sqlite_master_returns_one_row_per_table() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT)").unwrap();
    x.execute("CREATE TABLE b (y INT, z INT)").unwrap();
    let res = x.execute("SELECT * FROM sqlite_master").unwrap();
    // The system table is synthesized with 5 columns:
    // type, name, tbl_name, rootpage, sql.
    assert!(
        res.rows.len() >= 2,
        "expected ≥2 rows, got {}",
        res.rows.len()
    );
    for row in &res.rows {
        assert_eq!(row.len(), 5);
        assert_eq!(row[0], Value::Text("table".to_string()));
        assert_eq!(row[1], row[2], "tbl_name must equal name");
        assert_eq!(row[3], Value::Integer(0));
        let sql = match &row[4] {
            Value::Text(s) => s,
            other => panic!("expected Text sql, got {:?}", other),
        };
        assert!(
            sql.starts_with("CREATE TABLE"),
            "sql must start with CREATE TABLE, got: {}",
            sql
        );
    }
}

#[test]
fn v312_71_sqlite_schema_alias_works() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (x INT)").unwrap();
    // sqlite_schema is the SQLite 3.33+ alias for sqlite_master.
    // The dispatcher accepts both names. Use SELECT * since the
    // projection path isn't reached for the system-table shortcut.
    let res = x.execute("SELECT * FROM sqlite_schema").unwrap();
    assert_eq!(res.rows.len(), 1);
    // First row: type, name, tbl_name, rootpage, sql.
    assert_eq!(res.rows[0][1], Value::Text("t".to_string()));
}

#[test]
fn v312_71_sqlite_master_empty_catalog() {
    let mut x = fresh();
    let res = x.execute("SELECT * FROM sqlite_master").unwrap();
    assert_eq!(res.rows.len(), 0);
}

#[test]
fn v312_71_user_table_query_still_works_regression() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    // Regression: real user tables must still work.
    let res = x.execute("SELECT id, val FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(100));
}
