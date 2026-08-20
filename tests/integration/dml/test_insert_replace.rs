use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;
fn e() -> ExecutionEngine<MemoryStorage> {
    let s = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(s)
}
#[test]
fn test_insert_basic() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hello')").unwrap();
    let r = x.execute("SELECT INSERT(s, 2, 3, 'XYZ') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hXYZo".into()));
}
#[test]
fn test_insert_pos_out_of_range() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hi')").unwrap();
    let r = x.execute("SELECT INSERT(s, 10, 1, 'X') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hi".into()));
}
#[test]
fn test_insert_zero_len() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hello')").unwrap();
    let r = x.execute("SELECT INSERT(s, 3, 0, 'XX') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("heXXllo".into()));
}
#[test]
fn test_replace_basic() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hello world')").unwrap();
    let r = x
        .execute("SELECT REPLACE(s, 'world', 'Rust') FROM t")
        .unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello Rust".into()));
}
#[test]
fn test_replace_all() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('aaa')").unwrap();
    let r = x.execute("SELECT REPLACE(s, 'a', 'b') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("bbb".into()));
}
#[test]
fn test_replace_no_match() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hello')").unwrap();
    let r = x.execute("SELECT REPLACE(s, 'xyz', 'q') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello".into()));
}
