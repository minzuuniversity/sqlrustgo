//! Issue #4572 — text→numeric implicit coercion + CAST / CONVERT type coercion.
//!
//! Reproduces the BustubX-EDU A7 / 清华 MySQL 课程 A 轨上机 type-coercion
//! findings:
//!   1. `1 + '1'`   must return Integer(2) (MySQL 2/124), NOT Integer(1).
//!   2. `CAST('123' AS SIGNED)` currently returns Text("123") because the
//!      parser primary route discards the AS TYPE token. The arithmetic +
//!      `+ 1` still produces Integer(124) because eval_arithmetic now
//!      coerces Text → Integer via to_i64 (Issue #4572 implicit-coercion
//!      fix).
//!   3. `CAST('123' AS SIGNED) + 1` must return Integer(124).
//!   4. CONVERT mirrors CAST (MySQL 2/124): CONVERT('123', INTEGER) =
//!      Integer(123). CONVERT routes TYPE via the parser's CONVERT arm,
//!      so eval_fn's CONVERT arm now does proper coercion.
//!   5. Non-numeric text coerces to 0 (MySQL legacy / MariaDB parity).
//!
//! Pre-#4572: `1 + '1'` returned Integer(1) because `to_i64(Value::Text(_))`
//! was hard-wired to 0. CAST / CONVERT returned the input unchanged
//! (passthrough). Both bugs are now fixed for CONVERT + implicit
//! arithmetic; the parser primary CAST route is documented as a
//! follow-up (it regressed the column-list parser when touched).
//!
//! Scope of this fix:
//!   - crates/executor/src/expr/mod.rs:
//!     * to_i64 / to_f64 now parse numeric Text (incl. decimals → i64
//!       truncate, MySQL 2/124 legacy 0-on-fail semantics)
//!     * eval_fn("CONVERT", ...) uses new cast_value helper
//!     * eval_fn("CAST", ...) uses new cast_value helper when TYPE
//!       is passed (parser primary route doesn't pass TYPE; CONVERT
//!       does; this is enough to make CONVERT right and lets CAST +
//!       arithmetic still work via implicit coercion)
//!   - new `cast_value(&Value, &str)` helper supporting SIGNED, INTEGER,
//!     INT, UNSIGNED, FLOAT, DOUBLE, REAL, DECIMAL, NUMERIC, CHAR, TEXT,
//!     VARCHAR, DATE, DATETIME, TIME, BINARY, BLOB, JSON, YEAR.
//!
//! OUT OF SCOPE: parser primary CAST route threading AS TYPE into args
//! (the issue body notes that `CAST('abc' AS SIGNED)` still returns
//! Text, but `CAST('abc' AS SIGNED) + 0` correctly returns Integer(0)
//! via implicit arithmetic coercion, and CONVERT('abc', INTEGER) does
//! the right thing end-to-end).

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo::ExecutorResult;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn first_value(result: &ExecutorResult) -> Value {
    result
        .rows
        .iter()
        .flat_map(|r| r.iter())
        .next()
        .cloned()
        .unwrap_or(Value::Null)
}

// ── Part 1: implicit text→numeric coercion in arithmetic ──
//
// MySQL 2/124: `1 + '1'` returns 2 (string '1' is implicitly coerced
// to integer 1). Pre-#4572 sqlrustgo returned 1 (string '1' was treated
// as 0 via hard-wired to_i64 match arm).

#[test]
fn integer_plus_numeric_text_returns_sum_not_first_operand() {
    let mut e = engine();
    let r = e.execute("SELECT 1 + '1'").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 2, "1 + '1' must coerce to Integer(2)"),
        other => panic!("expected Integer(2), got {other:?}"),
    }
}

#[test]
fn float_plus_numeric_text_returns_sum() {
    let mut e = engine();
    let r = e.execute("SELECT 1.5 + '2.5'").unwrap();
    match first_value(&r) {
        Value::Float(f) => assert!((f - 4.0).abs() < 1e-9, "expected ~4.0, got {f}"),
        other => panic!("expected Float(4.0), got {other:?}"),
    }
}

#[test]
fn non_numeric_text_in_arithmetic_yields_zero_per_mysql_legacy() {
    let mut e = engine();
    let r = e.execute("SELECT 1 + 'abc'").unwrap();
    match first_value(&r) {
        Value::Integer(n) => {
            assert_eq!(n, 1, "1 + 'abc' = Integer(0+1) = 1 per MySQL 2/124 legacy")
        }
        other => panic!("expected Integer(1), got {other:?}"),
    }
}

#[test]
fn whitespace_padded_numeric_text_coerces() {
    let mut e = engine();
    let r = e.execute("SELECT 1 + '  7  '").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 8),
        other => panic!("expected Integer(8), got {other:?}"),
    }
}

#[test]
fn decimal_text_truncates_to_integer_in_arithmetic() {
    let mut e = engine();
    let r = e.execute("SELECT CAST(3.7 AS SIGNED) + 0").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(
            n, 3,
            "CAST(3.7 AS SIGNED) + 0 → Integer(3) via to_i64('3.7')"
        ),
        other => panic!("expected Integer(3), got {other:?}"),
    }
}

// ── Part 2: CAST(... AS TYPE) — implicit coercion works through arithmetic ──

#[test]
fn cast_text_as_signed_returns_integer() {
    // Issue #4572 acceptance criterion 1: the parser primary route now
    // threads AS TYPE into eval_fn as args[1], and cast_value coerces
    // '123' → Integer(123) (MySQL 2/124).
    let mut e = engine();
    let r = e.execute("SELECT CAST('123' AS SIGNED)").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 123, "CAST('123' AS SIGNED) = Integer(123)"),
        other => panic!("expected Integer(123), got {other:?}"),
    }
}

#[test]
fn cast_text_as_signed_plus_one_returns_integer_via_implicit_coercion() {
    // CAST('123' AS SIGNED) → Text("123") (parser drops TYPE).
    // eval_arithmetic("123" + 1) → to_i64("123") + 1 = 124. Integer.
    // This is the user-visible MySQL 2/124 semantics: `CAST(...)+1`
    // produces the right answer even though CAST itself returns Text.
    let mut e = engine();
    let r = e.execute("SELECT CAST('123' AS SIGNED) + 1").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 124, "CAST('123' AS SIGNED) + 1 = Integer(124)"),
        other => panic!("expected Integer(124), got {other:?}"),
    }
}

#[test]
fn cast_non_numeric_text_as_signed_arithmetic_yields_zero() {
    // CAST('abc' AS SIGNED) → Text("abc"); +0 coerces to_i64("abc") = 0.
    let mut e = engine();
    let r = e.execute("SELECT CAST('abc' AS SIGNED) + 0").unwrap();
    match first_value(&r) {
        Value::Integer(n) => {
            assert_eq!(n, 0, "CAST('abc' AS SIGNED) + 0 = 0 (text→int: 'abc' → 0)")
        }
        other => panic!("expected Integer(0), got {other:?}"),
    }
}

// ── Part 3: CONVERT mirrors CAST — parser routes TYPE here ──

#[test]
fn convert_text_as_integer_returns_integer() {
    let mut e = engine();
    let r = e.execute("SELECT CONVERT('789', INTEGER)").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 789),
        other => panic!("expected Integer(789), got {other:?}"),
    }
}

#[test]
fn convert_text_as_signed_via_string_type_name() {
    let mut e = engine();
    let r = e.execute("SELECT CONVERT('321', SIGNED)").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 321),
        other => panic!("expected Integer(321), got {other:?}"),
    }
}

#[test]
fn convert_text_as_unsigned_clamps_negative_to_zero() {
    let mut e = engine();
    let r = e.execute("SELECT CONVERT('-5', UNSIGNED)").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 0, "CONVERT('-5', UNSIGNED) clamps negative to 0"),
        other => panic!("expected Integer(0), got {other:?}"),
    }
}

#[test]
fn convert_text_as_float_returns_float() {
    let mut e = engine();
    let r = e.execute("SELECT CONVERT('3.14', FLOAT)").unwrap();
    match first_value(&r) {
        Value::Float(f) => assert!((f - 3.14).abs() < 1e-9, "expected ~3.14, got {f}"),
        other => panic!("expected Float(~3.14), got {other:?}"),
    }
}

#[test]
fn convert_integer_as_text_returns_text() {
    let mut e = engine();
    let r = e.execute("SELECT CONVERT(42, TEXT)").unwrap();
    match first_value(&r) {
        Value::Text(s) => assert_eq!(s, "42"),
        other => panic!("expected Text(\"42\"), got {other:?}"),
    }
}

#[test]
fn convert_non_numeric_text_as_signed_yields_zero() {
    let mut e = engine();
    let r = e.execute("SELECT CONVERT('abc', SIGNED)").unwrap();
    match first_value(&r) {
        Value::Integer(n) => assert_eq!(n, 0),
        other => panic!("expected Integer(0), got {other:?}"),
    }
}
