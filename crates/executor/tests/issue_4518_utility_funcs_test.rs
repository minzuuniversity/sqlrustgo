//! Regression tests for Issue #4518 — utility scalar functions (V312-58 / A7).
//!
//! Each test exercises one of:
//!   IFNULL / CONCAT / CHAR_LENGTH / LAST_INSERT_ID / CONVERT / CURRENT_USER
//! and asserts the expected non-NULL scalar result. The full function
//! list in the issue (#4518 body) is:
//!   IFNULL / COALESCE / NULLIF / CONCAT / CHAR_LENGTH / LAST_INSERT_ID
//!   / CURRENT_USER / VERSION / CAST / CONVERT
//!
//! COALESCE / NULLIF / CAST were already covered by earlier PRs (PR #4508
//! builtin coverage, PR #4508 group by functional dependency, BUG-2b fix).
//! VERSION() routes through the system-variable resolver and is exercised
//! by the wire-protocol gate (`scripts/gate/check_v312_13_wire_load_data.sh`).
//! This file adds coverage for the six functions newly implemented in
//! `eval_fn` for #4518.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn first_text(result: &sqlrustgo::ExecutorResult) -> &str {
    match first_value(result) {
        Value::Text(s) => s.as_str(),
        other => panic!("expected Text, got {other:?}"),
    }
}

fn first_int(result: &sqlrustgo::ExecutorResult) -> i64 {
    match first_value(result) {
        Value::Integer(i) => *i,
        other => panic!("expected Integer, got {other:?}"),
    }
}

fn first_value(result: &sqlrustgo::ExecutorResult) -> &Value {
    result
        .rows
        .first()
        .and_then(|row| row.first())
        .expect("at least one row")
}

#[test]
fn ifnull_returns_first_non_null() {
    let mut e = engine();
    let r = e.execute("SELECT IFNULL(NULL, 'fallback')").unwrap();
    assert_eq!(first_text(&r), "fallback");
}

#[test]
fn ifnull_returns_first_arg_when_not_null() {
    let mut e = engine();
    let r = e.execute("SELECT IFNULL('primary', 'fallback')").unwrap();
    assert_eq!(first_text(&r), "primary");
}

#[test]
fn ifnull_returns_null_when_all_null() {
    let mut e = engine();
    let r = e.execute("SELECT IFNULL(NULL, NULL)").unwrap();
    assert!(matches!(first_value(&r), Value::Null));
}

#[test]
fn concat_two_strings() {
    let mut e = engine();
    let r = e.execute("SELECT CONCAT('a', 'b')").unwrap();
    assert_eq!(first_text(&r), "ab");
}

#[test]
fn concat_many_strings() {
    let mut e = engine();
    let r = e.execute("SELECT CONCAT('a', 'b', 'c', 'd')").unwrap();
    assert_eq!(first_text(&r), "abcd");
}

#[test]
fn concat_treats_null_as_empty_string() {
    // MySQL semantics: NULL args inside CONCAT become empty string.
    let mut e = engine();
    let r = e.execute("SELECT CONCAT('x', NULL, 'y')").unwrap();
    assert_eq!(first_text(&r), "xy");
}

#[test]
fn concat_mixed_types_coerces_to_string() {
    let mut e = engine();
    let r = e.execute("SELECT CONCAT('count=', 42)").unwrap();
    assert_eq!(first_text(&r), "count=42");
}

#[test]
fn char_length_counts_codepoints() {
    let mut e = engine();
    let r = e.execute("SELECT CHAR_LENGTH('alice')").unwrap();
    assert_eq!(first_int(&r), 5);
}

#[test]
fn char_length_handles_empty_string() {
    let mut e = engine();
    let r = e.execute("SELECT CHAR_LENGTH('')").unwrap();
    assert_eq!(first_int(&r), 0);
}

#[test]
fn character_length_alias_works() {
    let mut e = engine();
    let r = e.execute("SELECT CHARACTER_LENGTH('hello')").unwrap();
    assert_eq!(first_int(&r), 5);
}

#[test]
fn last_insert_id_returns_zero_stateless() {
    // Stateless eval_fn has no AUTO_INCREMENT session state; per MySQL
    // default-without-INSERT, LAST_INSERT_ID() returns 0.
    let mut e = engine();
    let r = e.execute("SELECT LAST_INSERT_ID()").unwrap();
    assert_eq!(first_int(&r), 0);
}

#[test]
fn convert_returns_input_value_passthrough() {
    // V312-59-D / Issue #4572: CONVERT now performs real MySQL type
    // coercion (was a passthrough before #4572). CONVERT('123', INTEGER)
    // = CAST('123' AS SIGNED) = Integer(123). This is MySQL 2/124
    // semantics; non-numeric text returns 0 (not error).
    let mut e = engine();
    let r = e.execute("SELECT CONVERT('123', INTEGER)").unwrap();
    match first_value(&r) {
        Value::Integer(n) => {
            assert_eq!(*n, 123i64, "CONVERT('123', INTEGER) should be Integer(123)")
        }
        other => panic!("CONVERT should coerce to Integer, got {other:?}"),
    }
    // Coercion from text containing a non-numeric falls back to 0.
    let r2 = e.execute("SELECT CONVERT('abc', INTEGER)").unwrap();
    match first_value(&r2) {
        Value::Integer(n) => assert_eq!(*n, 0i64, "non-numeric text coerces to 0"),
        other => panic!("expected Integer(0), got {other:?}"),
    }
}

#[test]
fn current_user_returns_default_user_identifier() {
    // sqlrustgo runs in single-user mode; CURRENT_USER returns
    // "openclaw@%" (canonical default) instead of empty string.
    let mut e = engine();
    let r = e.execute("SELECT CURRENT_USER()").unwrap();
    let s = first_text(&r);
    assert!(
        s.contains('@'),
        "CURRENT_USER should contain '@', got {s:?}"
    );
    assert!(!s.is_empty(), "CURRENT_USER should not be empty");
}

#[test]
fn user_returns_same_as_current_user() {
    let mut e = engine();
    let r = e.execute("SELECT USER()").unwrap();
    let s = first_text(&r);
    assert!(s.contains('@'), "USER should contain '@', got {s:?}");
}
#[test]
fn convert_date_keyword_path() {
    // DATE keyword in CONVERT second arg slot — exercises Token::Date
    // branch of parser.rs:7483 CONVERT special-case.
    let mut e = engine();
    let r = e.execute("SELECT CONVERT('2026-08-27', DATE)");
    match r {
        Ok(result) => {
            let v = first_value(&result);
            // DATE in Token::Date arm produces Literal("Date") via
            // format!("{:?}", Token::Date). eval_fn's CONVERT arm
            // returns args.first() = '2026-08-27' as Text.
            match v {
                Value::Text(s) => assert_eq!(s, "2026-08-27"),
                other => panic!("CONVERT(DATE) should pass input through, got {other:?}"),
            }
        }
        Err(e) => panic!("CONVERT('x', DATE) should parse, got Err: {e}"),
    }
}
