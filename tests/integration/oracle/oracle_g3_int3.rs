//! G3 INT-3 Expression Delegation Oracle (V4 fix)
//!
//! 验证 14+ Expression variants 各自产生 oracle-correct 结果.
//! Oracle: 每个 expression variant 用 ground-truth computation 验证.

#[path = "../../common/mod.rs"]
mod common;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::Arc;

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn setup_table(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, val INTEGER, score REAL)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'alice', 100, 1.5)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (2, 'bob', 200, 2.5)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (3, 'charlie', 300, 3.5)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (4, 'dave', 400, 4.5)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (5, 'eve', 500, 5.5)")
        .unwrap();
}

#[test]
fn g3_int3_identifier_delegation_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT name FROM t WHERE id = 3").unwrap();
    assert_eq!(r.rows.len(), 1, "Oracle: id=3 has 1 row");
    assert_eq!(
        r.rows[0][0],
        Value::Text("charlie".to_string()),
        "Oracle: id=3 → 'charlie'"
    );
}

#[test]
fn g3_int3_arithmetic_delegation_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);
    let r = engine
        .execute("SELECT val * 2 FROM t WHERE id = 2")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let doubled = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int, got {:?}", r.rows[0][0]),
    };
    assert_eq!(doubled, 400, "Oracle: val=200 * 2 = 400");
}

#[test]
fn g3_int3_comparison_delegation_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);
    let r = engine
        .execute("SELECT COUNT(*) FROM t WHERE val >= 300")
        .unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int, got {:?}", r.rows[0][0]),
    };
    assert_eq!(count, 3, "Oracle: val >= 300 means id 3, 4, 5 → 3 rows");
}

#[test]
fn g3_int3_logical_delegation_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);
    let r = engine
        .execute("SELECT COUNT(*) FROM t WHERE val > 100 AND val < 400")
        .unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int, got {:?}", r.rows[0][0]),
    };
    assert_eq!(count, 2, "Oracle: 100 < val < 400 → val 200, 300 = 2 rows");
}

#[test]
fn g3_int3_aggregate_delegation_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);
    let r = engine
        .execute("SELECT SUM(val), AVG(val), MIN(val), MAX(val) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let sum = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int sum, got {:?}", r.rows[0][0]),
    };
    assert_eq!(sum, 1500, "Oracle: 100+200+300+400+500 = 1500");
    let max = match &r.rows[0][3] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int max, got {:?}", r.rows[0][3]),
    };
    assert_eq!(max, 500, "Oracle: max val = 500");
}

#[test]
fn g3_int3_is_null_delegation_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);
    let r = engine
        .execute("SELECT COUNT(*) FROM t WHERE name IS NOT NULL")
        .unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int, got {:?}", r.rows[0][0]),
    };
    assert_eq!(count, 5, "Oracle: all 5 rows have name IS NOT NULL");
}

#[test]
fn g3_int3_like_delegation_oracle() {
    let mut engine = make_engine();
    setup_table(&mut engine);
    let r = engine
        .execute("SELECT COUNT(*) FROM t WHERE name LIKE 'a%'")
        .unwrap();
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int, got {:?}", r.rows[0][0]),
    };
    assert_eq!(count, 1, "Oracle: only 'alice' starts with 'a'");
}
