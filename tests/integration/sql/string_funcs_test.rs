#![allow(dead_code)]

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;
fn e() -> ExecutionEngine<MemoryStorage> {
    let s = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(s)
}

fn t(s: &str) -> sqlrustgo::Value {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute(&format!(
        "INSERT INTO t VALUES ('{}')",
        s.replace('\'', "''")
    ))
    .unwrap();
    x.execute("SELECT s FROM t").unwrap().rows[0][0].clone()
}

#[test]
fn test_left() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hello world')").unwrap();
    let r = x.execute("SELECT LEFT(s, 5) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello".into()));
    let r = x.execute("SELECT LEFT(s, 100) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello world".into()));
    let r = x.execute("SELECT LEFT(s, 0) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text(String::new()));
}

#[test]
fn test_right() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hello world')").unwrap();
    let r = x.execute("SELECT RIGHT(s, 5) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("world".into()));
    let r = x.execute("SELECT RIGHT(s, 100) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello world".into()));
}

#[test]
fn test_lpad() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hi')").unwrap();
    let r = x.execute("SELECT LPAD(s, 5, 'ab') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("abahi".into()));
    let r = x.execute("SELECT LPAD(s, 5, 'X') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("XXXhi".into()));
    // Truncation
    let r = x.execute("SELECT LPAD(s, 1, 'X') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("h".into()));
}

#[test]
fn test_rpad() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hi')").unwrap();
    let r = x.execute("SELECT RPAD(s, 5, 'ab') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hiaba".into()));
    let r = x.execute("SELECT RPAD(s, 5, 'X') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hiXXX".into()));
}

#[test]
fn test_repeat() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('ab')").unwrap();
    let r = x.execute("SELECT REPEAT(s, 3) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("ababab".into()));
    let r = x.execute("SELECT REPEAT(s, 0) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text(String::new()));
}

#[test]
fn test_reverse() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('hello')").unwrap();
    let r = x.execute("SELECT REVERSE(s) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("olleh".into()));
}

#[test]
fn test_space() {
    let mut x = e();
    x.execute("CREATE TABLE t (n INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (5)").unwrap();
    let r = x.execute("SELECT SPACE(n) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text(" ".repeat(5)));
    let r = x.execute("SELECT SPACE(0) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text(String::new()));
}

#[test]
fn test_ltrim_rtrim() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('  hello  ')").unwrap();
    let r = x.execute("SELECT LTRIM(s) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello  ".into()));
    let r = x.execute("SELECT RTRIM(s) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("  hello".into()));
    let r = x.execute("SELECT TRIM(s) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello".into()));
}

#[test]
fn test_field() {
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('b')").unwrap();
    let r = x.execute("SELECT FIELD(s, 'a', 'b', 'c') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(2));
    let r = x.execute("SELECT FIELD(s, 'x', 'y', 'z') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(0));
}

#[test]
fn test_elt() {
    let mut x = e();
    x.execute("CREATE TABLE t (n INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();
    let r = x.execute("SELECT ELT(n, 'a', 'b', 'c') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("b".into()));
    let r = x.execute("SELECT ELT(0, 'a', 'b') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Null);
    let r = x.execute("SELECT ELT(5, 'a', 'b') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Null);
}

// ----- Issue #2988 sub-task A: TRIM with LEADING/TRAILING/BOTH modifier -----
// MySQL 5.7 standard SQL form: TRIM(LEADING/TRAILING/BOTH remstr FROM str)
// Default (no modifier) = BOTH.
// See: https://dev.mysql.com/doc/refman/5.7/en/string-functions.html#function_trim

#[test]
fn test_trim_leading_modifier() {
    // TRIM(LEADING 'x' FROM 'xxxhelloxxx') = 'helloxxx'
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('xxxhelloxxx')").unwrap();
    let r = x
        .execute("SELECT TRIM(LEADING 'x' FROM s) FROM t")
        .expect("TRIM(LEADING remstr FROM str) should parse and execute");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("helloxxx".into()));
}

#[test]
fn test_trim_trailing_modifier() {
    // TRIM(TRAILING 'x' FROM 'xxxhelloxxx') = 'xxxhello'
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('xxxhelloxxx')").unwrap();
    let r = x
        .execute("SELECT TRIM(TRAILING 'x' FROM s) FROM t")
        .expect("TRIM(TRAILING remstr FROM str) should parse and execute");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("xxxhello".into()));
}

#[test]
fn test_trim_both_modifier_explicit() {
    // TRIM(BOTH 'x' FROM 'xxxhelloxxx') = 'hello' (same as default)
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('xxxhelloxxx')").unwrap();
    let r = x
        .execute("SELECT TRIM(BOTH 'x' FROM s) FROM t")
        .expect("TRIM(BOTH remstr FROM str) should parse and execute");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello".into()));
}

#[test]
fn test_trim_no_modifier_default() {
    // TRIM(remstr FROM str) without modifier = TRIM(BOTH remstr FROM str)
    // Already in 2-arg form per PR #3045, but verify it still works.
    let mut x = e();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('xxxhelloxxx')").unwrap();
    let r = x
        .execute("SELECT TRIM('x' FROM s) FROM t")
        .expect("TRIM(remstr FROM str) should parse and execute");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("hello".into()));
}
