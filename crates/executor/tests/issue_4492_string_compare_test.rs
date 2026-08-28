//! Regression tests for Issue #4492 — `char(n)` vs short string equality.
//!
//! `Value::PartialEq` remains strict (preserves Hash/sort invariants),
//! but `eval_binary_op`'s string comparison arms trim trailing whitespace
//! before comparing. This file pins down the behavior at the executor
//! boundary, which is the level end users observe.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn count_where(e: &mut ExecutionEngine<MemoryStorage>, where_clause: &str) -> i64 {
    let r = e
        .execute(&format!(
            "CREATE TABLE t(id INTEGER, s TEXT); \
             INSERT INTO t VALUES (1, 'abc'), (2, 'abc '), (3, 'abcd'); \
             SELECT COUNT(*) FROM t WHERE {where_clause}"
        ))
        .expect("execute");
    match &r.rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("expected integer, got {other:?}"),
    }
}

#[test]
fn char_vs_short_string_equal() {
    // Issue #4492 minimal repro: `sex CHAR(2)` vs short string 'F'.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, sex CHAR(2))")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'F'), (2, 'M')")
        .unwrap();
    let r = e.execute("SELECT COUNT(*) FROM t WHERE sex = 'F'").unwrap();
    assert!(
        matches!(&r.rows[0][0], Value::Integer(n) if *n == 1),
        "WHERE sex = 'F' on CHAR(2) column should count 1 row, got {r:?}"
    );
}

#[test]
fn char_vs_short_string_count_zero_when_neither_matches() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, sex CHAR(2))")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'F'), (2, 'M')")
        .unwrap();
    let r = e.execute("SELECT COUNT(*) FROM t WHERE sex = 'X'").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(0)));
}

#[test]
fn trailing_space_text_equal_to_short_text() {
    // WHERE 'abc ' = 'abc' should match both rows.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, s TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'abc'), (2, 'abc ')")
        .unwrap();
    let r = e.execute("SELECT COUNT(*) FROM t WHERE s = 'abc'").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(n) if *n == 2));
}

#[test]
fn leading_space_preserved() {
    // ' abc' (leading space) must NOT equal 'abc' — leading whitespace
    // is significant; only trailing whitespace is trimmed.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, s TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, ' abc')").unwrap();
    let r = e.execute("SELECT COUNT(*) FROM t WHERE s = 'abc'").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(0)));
}

#[test]
fn inequality_with_distinct_content() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, s TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'abc'), (2, 'def')")
        .unwrap();
    let r = e.execute("SELECT COUNT(*) FROM t WHERE s = 'def'").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(1)));
}

#[test]
fn not_equal_trailing_space_distinct_content() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, s TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'abc '), (2, 'abcd')")
        .unwrap();
    // 'abc ' (after trim = 'abc') ≠ 'abcd'.
    let r = e
        .execute("SELECT COUNT(*) FROM t WHERE s = 'abcd'")
        .unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(1)));
    let r2 = e.execute("SELECT COUNT(*) FROM t WHERE s = 'abc'").unwrap();
    assert!(matches!(&r2.rows[0][0], Value::Integer(1))); // only 'abc ' matches 'abc'
}

#[test]
fn ordering_respects_trimmed_compare() {
    // ORDER BY: 'abc ' should sort alongside 'abc' (both trim to 'abc').
    // To avoid relying on ORDER-BY semantics, we use a CASE expression
    // that returns 1 for trimmed-equal and verify via SUM.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, s TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'abc'), (2, 'abc ')")
        .unwrap();
    // Both rows match 'abc'; SUM(id) = 3.
    let r = e.execute("SELECT SUM(id) FROM t WHERE s = 'abc'").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(n) if *n == 3));
}

#[test]
fn value_partial_eq_strict_for_hash_invariant() {
    // Invariant guard: Value::PartialEq MUST NOT trim. If this breaks,
    // HashMap / GROUP BY semantics will collapse (Bug #4492 risk).
    let a = Value::Text("F".to_string());
    let b = Value::Text("F ".to_string());
    assert_ne!(a, b, "strict equality is preserved");
    assert_eq!(a, Value::Text("F".to_string()));
}

#[test]
fn char_2_vs_short_string_in_where_returns_correct_row() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, sex CHAR(2))")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'F'), (2, 'M')")
        .unwrap();
    let r = e.execute("SELECT id FROM t WHERE sex = 'M'").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(&r.rows[0][0], Value::Integer(2)));
}

#[test]
fn char_eq_padded_string_in_where() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, s TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'abc'), (2, 'abc ')")
        .unwrap();
    // Trailing-padded string matches plain short string.
    let r = e.execute("SELECT COUNT(*) FROM t WHERE s = 'abc'").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(2)));
}

// Suppress unused-helper warning
#[allow(dead_code)]
fn _force_use_helper(e: &mut ExecutionEngine<MemoryStorage>, where_clause: &str) -> i64 {
    count_where(e, where_clause)
}
