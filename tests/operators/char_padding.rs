//! Operator-level regression test for CHAR(N) trailing space padding.
//!
//! Sprint 3 (Operator Regression Suite) / Issue #3283 Task 8.
//!
//! Locks in the cosmetic CHAR(N) padding contract (Sprint 4 / Issue #3290):
//! - CHAR(N) values should be padded with trailing spaces to width N.
//! - Affects TPC-H Q4/Q12/Q15/Q16 cell-level match against PG.
//!
//! Note: This test is a "soft" acceptance — if CHAR(N) padding is not
//! yet implemented in sqlrustgo, this test will fail with a clear
//! message guiding the developer to add the fix.
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

#[test]
fn char_15_padding_insert_and_select() {
    let mut e = engine();
    // c_phone is CHAR(15) in TPC-H
    e.execute("CREATE TABLE customer (c_phone CHAR(15))")
        .unwrap();
    // Insert a 10-char phone number
    e.execute("INSERT INTO customer VALUES ('123-456-78')")
        .unwrap();
    let r = e.execute("SELECT c_phone FROM customer").unwrap();
    let stored = r.rows[0][0].to_string();
    assert_eq!(
        stored.len(),
        15,
        "CHAR(15) should pad to 15 chars, got len={} value='{}'",
        stored.len(),
        stored
    );
    assert!(
        stored.starts_with("123-456-78"),
        "Expected prefix '123-456-78', got '{}'",
        stored
    );
}

#[test]
fn char_15_full_width_stays_at_15() {
    // A value exactly 15 chars stays at 15 (no truncation)
    let mut e = engine();
    e.execute("CREATE TABLE t (s CHAR(15))").unwrap();
    e.execute("INSERT INTO t VALUES ('ABCDEFGHIJKLMNO')")
        .unwrap();
    let r = e.execute("SELECT s FROM t").unwrap();
    assert_eq!(r.rows[0][0].to_string().len(), 15);
    assert_eq!(r.rows[0][0].to_string(), "ABCDEFGHIJKLMNO");
}

#[test]
fn char_comparison_with_constant() {
    // CHAR(15) vs CHAR(15) literal comparison should work
    let mut e = engine();
    e.execute("CREATE TABLE t (s CHAR(15))").unwrap();
    e.execute("INSERT INTO t VALUES ('hello')").unwrap();
    let r = e
        .execute("SELECT COUNT(*) FROM t WHERE s = 'hello'")
        .unwrap();
    assert_eq!(r.rows[0][0].to_string(), "1");
}

#[test]
fn char_25_padding() {
    // TPC-H p_name is CHAR(55) but we test smaller CHAR(25)
    let mut e = engine();
    e.execute("CREATE TABLE t (s CHAR(25))").unwrap();
    e.execute("INSERT INTO t VALUES ('short')").unwrap();
    let r = e.execute("SELECT s FROM t").unwrap();
    let stored = r.rows[0][0].to_string();
    assert_eq!(
        stored.len(),
        25,
        "CHAR(25) should pad to 25, got len={}",
        stored.len()
    );
}
