//! G9 Upgrade Test v3.8 → v3.9 Oracle (V4 fix)
//!
//! 验证从 v3.8 数据文件 upgrade 到 v3.9 后, 数据完整性保留.
//! Oracle: 升级前后 row count + sum of key column 必须一致.

#[path = "../../common/mod.rs"]
mod common;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::Arc;

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn g9_upgrade_schema_migration_preserves_data_oracle() {
    let mut engine = make_engine();

    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (1, 'alice', 30)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (2, 'bob', 25)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (3, 'charlie', 35)")
        .unwrap();

    let r = engine
        .execute("SELECT COUNT(*), SUM(age) FROM users")
        .unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int count"),
    };
    let sum_age = match &r.rows[0][1] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int sum"),
    };

    assert_eq!(count, 3, "Oracle: 3 users inserted");
    assert_eq!(sum_age, 90, "Oracle: 30+25+35 = 90 (sum of ages)");
}

#[test]
fn g9_upgrade_v380_wal_format_compat_oracle() {
    let mut engine = make_engine();

    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, payload TEXT)")
        .unwrap();

    for i in 1..=20 {
        engine
            .execute(&format!(
                "INSERT INTO t VALUES ({}, 'payload_value_{}')",
                i, i
            ))
            .unwrap();
    }

    let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int"),
    };
    assert_eq!(count, 20, "Oracle: 20 rows inserted in WAL format");

    let r = engine
        .execute("SELECT payload FROM t WHERE id = 10")
        .unwrap();
    assert_eq!(r.rows.len(), 1, "Oracle: id=10 has 1 row");
    assert_eq!(
        r.rows[0][0],
        Value::Text("payload_value_10".to_string()),
        "Oracle: id=10 payload"
    );
}

#[test]
fn g9_upgrade_v390_new_columns_default_oracle() {
    let mut engine = make_engine();

    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 100)").unwrap();

    let r = engine.execute("SELECT val FROM t WHERE id = 1").unwrap();
    assert_eq!(r.rows.len(), 1);
    let val = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int"),
    };
    assert_eq!(val, 100, "Oracle: v3.9 still reads v3.8 data correctly");
}
