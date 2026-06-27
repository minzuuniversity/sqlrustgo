//! Compare `storage.insert` path vs `INSERT INTO` path for column resolution

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::sync::{Arc, RwLock};

#[test]
fn test_storage_insert_path() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute("CREATE TABLE t (a TEXT, b INTEGER, c REAL)")
        .expect("create");

    // Direct insert via storage
    {
        let mut s = storage.write().unwrap();
        s.insert(
            "t",
            vec![
                vec![
                    SqlValue::Text("1992-01-01".to_string()),
                    SqlValue::Integer(1),
                    SqlValue::Float(0.05),
                ],
                vec![
                    SqlValue::Text("1994-01-01".to_string()),
                    SqlValue::Integer(2),
                    SqlValue::Float(0.07),
                ],
                vec![
                    SqlValue::Text("1995-01-01".to_string()),
                    SqlValue::Integer(3),
                    SqlValue::Float(0.06),
                ],
                vec![
                    SqlValue::Text("1996-12-31".to_string()),
                    SqlValue::Integer(4),
                    SqlValue::Float(0.10),
                ],
            ],
        )
        .expect("insert");
    }

    // Check all rows
    let r = engine.execute("SELECT a FROM t").expect("all");
    eprintln!("storage.insert path - all 4 rows: {:?}", r.rows);

    // a >= '1994-01-01' should return 3 rows
    let r2 = engine
        .execute("SELECT a FROM t WHERE a >= '1994-01-01'")
        .expect(">=");
    eprintln!(
        "storage.insert path - a >= '1994-01-01' (expect 3): {:?}",
        r2.rows
    );
}

#[test]
fn test_insert_via_sql_path() {
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (a TEXT, b INTEGER, c REAL)")
        .expect("create");
    engine
        .execute("INSERT INTO t VALUES ('1992-01-01', 1, 0.05), ('1994-01-01', 2, 0.07), ('1995-01-01', 3, 0.06), ('1996-12-31', 4, 0.10)")
        .expect("insert");

    let r2 = engine
        .execute("SELECT a FROM t WHERE a >= '1994-01-01'")
        .expect(">=");
    eprintln!(
        "SQL insert path - a >= '1994-01-01' (expect 3): {:?}",
        r2.rows
    );
}
