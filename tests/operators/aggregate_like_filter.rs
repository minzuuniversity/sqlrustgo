//! Operator-level regression test for WHERE col LIKE pattern filter.
//!
//! Sprint 3 (Operator Regression Suite) / Issue #3283 Task 4.
//!
//! Locks in the Q14/Q02 cell-diff pattern: WHERE col LIKE 'pattern%'
//! should correctly filter rows by pattern match.
//!
//! Acceptance (per Issue #3283):
//! - Use a minimal fixture (3-5 rows)
//! - Assert cell values, not just row count
//! - Run in < 100ms

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn load_fixture(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("CREATE TABLE p (p_name TEXT, p_val INTEGER)").unwrap();
    e.execute("INSERT INTO p VALUES ('forest almond', 1)").unwrap();
    e.execute("INSERT INTO p VALUES ('forest blue', 2)").unwrap();
    e.execute("INSERT INTO p VALUES ('lemon yellow', 3)").unwrap();
    e.execute("INSERT INTO p VALUES ('forest green', 4)").unwrap();
    e.execute("INSERT INTO p VALUES ('almond chocolate', 5)").unwrap();
}

#[test]
fn like_prefix_filters_correctly() {
    let mut e = engine();
    load_fixture(&mut e);
    let r = e.execute("SELECT COUNT(*) FROM p WHERE p_name LIKE 'forest%'").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "3");
}

#[test]
fn like_with_aggregate() {
    let mut e = engine();
    load_fixture(&mut e);
    let r = e.execute("SELECT SUM(p_val) FROM p WHERE p_name LIKE 'forest%'").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "7");
}

#[test]
fn like_no_match_returns_zero() {
    let mut e = engine();
    load_fixture(&mut e);
    let r = e.execute("SELECT COUNT(*) FROM p WHERE p_name LIKE 'banana%'").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "0");
}

#[test]
fn like_with_underscore_wildcard() {
    // '_' matches exactly one character
    let mut e = engine();
    e.execute("CREATE TABLE t (s TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES ('ab')").unwrap();
    e.execute("INSERT INTO t VALUES ('abc')").unwrap();
    e.execute("INSERT INTO t VALUES ('abcd')").unwrap();
    e.execute("INSERT INTO t VALUES ('a')").unwrap();
    let r = e.execute("SELECT COUNT(*) FROM t WHERE s LIKE 'ab_'").unwrap();
    // Matches 'abc' only (3 chars: ab + 1)
    assert_eq!(r.rows[0][0].to_string(), "1");
}