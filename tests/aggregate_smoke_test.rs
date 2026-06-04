//! RC1 smoke tests for basic SQL aggregate functions (Issue #2969 / EXEC-03)
//!
//! These tests cover the 5 standard SQL aggregates (Count/Sum/Avg/Min/Max)
//! via the public `ExecutionEngine::execute` path. The advanced aggregates
//! (StdDev, Variance, Median, GroupConcat) live in `local_executor.rs` /
//! `parallel_executor.rs` which are **not in the mod tree** — see issue
//! #2969 for the RC1 fix plan and `docs/releases/v3.8.0/V380_FROZEN_TO_V390.md`
//! for the freeze rationale.

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::{Arc, RwLock};

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn aggregate_5_basics() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40),(5,50)").unwrap();
    assert_eq!(
        x.execute("SELECT COUNT(*) FROM t").unwrap().rows[0][0],
        sqlrustgo::Value::Integer(5)
    );
    assert_eq!(
        x.execute("SELECT SUM(v) FROM t").unwrap().rows[0][0],
        sqlrustgo::Value::Integer(150)
    );
    assert_eq!(
        x.execute("SELECT AVG(v) FROM t").unwrap().rows[0][0],
        sqlrustgo::Value::Integer(30)
    );
    let r = x.execute("SELECT MIN(v), MAX(v) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(10));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(50));
}

#[test]
fn aggregate_group_by() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('a',1),('a',2),('b',3),('b',4),('b',5)").unwrap();
    let r = x
        .execute("SELECT g, SUM(v), COUNT(*) FROM t GROUP BY g")
        .unwrap();
    // Order is not strictly guaranteed; assert by group key.
    assert_eq!(r.rows.len(), 2);
    let mut sum_a: i64 = 0;
    let mut sum_b: i64 = 0;
    let mut count_a: i64 = 0;
    let mut count_b: i64 = 0;
    for row in &r.rows {
        match (&row[0], &row[1], &row[2]) {
            (
                sqlrustgo::Value::Text(g),
                sqlrustgo::Value::Integer(v),
                sqlrustgo::Value::Integer(c),
            ) if g == "a" => {
                sum_a = *v;
                count_a = *c;
            }
            (
                sqlrustgo::Value::Text(g),
                sqlrustgo::Value::Integer(v),
                sqlrustgo::Value::Integer(c),
            ) if g == "b" => {
                sum_b = *v;
                count_b = *c;
            }
            _ => panic!("unexpected row: {:?}", row),
        }
    }
    assert_eq!(sum_a, 3, "group a should sum to 3");
    assert_eq!(sum_b, 12, "group b should sum to 12");
    assert_eq!(count_a, 2);
    assert_eq!(count_b, 3);
}

#[test]
fn aggregate_having() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('a',1),('a',2),('b',3),('b',4),('b',5)").unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g HAVING SUM(v) > 5")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    // Only group b (sum=12) should pass HAVING.
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("b".into()));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(12));
}
