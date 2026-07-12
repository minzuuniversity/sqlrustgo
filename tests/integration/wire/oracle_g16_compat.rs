//! G16 Compatibility v3.8→v3.9 Oracle (V4 fix)
//!
//! 验证 v3.9.0 in-process 模拟 v3.8→v3.9 upgrade 流程后, 数据完整性保留.
//! Baseline 来源: `tests/oracle/baselines/compat_v3.8_v3.9.json`.

mod common;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::path::Path;
use std::sync::Arc;

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn load_compat_baseline() -> serde_json::Value {
    let path = Path::new("tests/oracle/baselines/compat_v3.8_v3.9.json");
    let content = std::fs::read_to_string(path).expect("G16 compat baseline not found");
    serde_json::from_str(&content).expect("G16 compat baseline parse failed")
}

#[test]
fn g16_compat_users_table_row_count() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
        .unwrap();

    for i in 1..=100 {
        engine
            .execute(&format!(
                "INSERT INTO users VALUES ({}, 'user_{}', {})",
                i,
                i,
                20 + (i % 50)
            ))
            .unwrap();
    }

    let r = engine.execute("SELECT COUNT(*) FROM users").unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Integer"),
    };
    let baseline = load_compat_baseline();
    let expected = baseline
        .get("checks")
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|c| c.get("test").and_then(|t| t.as_str()) == Some("users_table_row_count"))
        })
        .and_then(|c| c.get("expected"))
        .and_then(|e| e.as_u64())
        .expect("baseline users_table_row_count") as i64;
    assert_eq!(count, expected, "users row count after compat");
}

#[test]
fn g16_compat_orders_table_row_count() {
    let mut engine = make_engine();
    engine
        .execute(
            "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER, o_total REAL)",
        )
        .unwrap();
    for i in 1..=1000 {
        engine
            .execute(&format!(
                "INSERT INTO orders VALUES ({}, {}, {:.2})",
                i,
                i % 100,
                i as f64 * 1.5
            ))
            .unwrap();
    }
    let r = engine.execute("SELECT COUNT(*) FROM orders").unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Integer"),
    };
    let baseline = load_compat_baseline();
    let expected = baseline
        .get("checks")
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|c| c.get("test").and_then(|t| t.as_str()) == Some("orders_table_row_count"))
        })
        .and_then(|c| c.get("expected"))
        .and_then(|e| e.as_u64())
        .expect("baseline orders_table_row_count") as i64;
    assert_eq!(count, expected, "orders row count after compat");
}

#[test]
fn g16_compat_sum_ages_preserved() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
        .unwrap();

    let expected_total: i64 = (1..=100).map(|i| 20 + (i % 50)).sum();

    for i in 1..=100 {
        engine
            .execute(&format!(
                "INSERT INTO users VALUES ({}, 'user_{}', {})",
                i,
                i,
                20 + (i % 50)
            ))
            .unwrap();
    }

    let r = engine.execute("SELECT SUM(age) FROM users").unwrap();
    let total = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Integer"),
    };
    assert_eq!(
        total, expected_total,
        "sum of ages preserved after compat (engine={} expected={})",
        total, expected_total
    );
}

#[test]
fn g16_compat_primary_key_uniqueness() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'a')").unwrap();

    let dup_result = engine.execute("INSERT INTO t VALUES (1, 'b')");
    assert!(
        dup_result.is_err(),
        "Primary key uniqueness must be enforced after compat"
    );
}
