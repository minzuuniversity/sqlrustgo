//! Operator-level regression tests for SQL three-valued logic (3VL).
//!
//! Two defects motivated this suite:
//!
//! 1. **`NOT NULL` returned TRUE.** `eval_unary_op` fed `Value::Null` through
//!    `to_bool()` (which maps NULL → false), then negated it to true. The
//!    SQL standard defines `NOT UNKNOWN` as UNKNOWN (NULL), not TRUE.
//!
//! 2. **Comparison / AND / OR projected NULLs as FALSE.** The `eval_binary_op`
//!    arms delegated NULL operands to `eq_cross` / `compare_cmp`, which both
//!    returned `Boolean(false)` for NULL. The SQL standard returns NULL for
//!    every comparison with a NULL operand (`NULL = NULL` → NULL, not FALSE).
//!
//! All expected values match SQLite (the teaching corpus oracle).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

/// Single-row table whose only column `v` holds SQL NULL.
///
/// The cases are written against a column reference rather than a bare
/// `NULL` literal: sqlrustgo's parser does not accept a bare `NULL` as the
/// left operand of a comparison (`SELECT NULL = NULL` is a parse error), so
/// a literal-based form would test the parser, not the evaluator. Every
/// assertion below is about the value the evaluator produces.
fn null_engine() -> ExecutionEngine<MemoryStorage> {
    let mut e = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    e.execute("CREATE TABLE n (v INT)").unwrap();
    e.execute("INSERT INTO n VALUES (NULL)").unwrap();
    e
}

/// Evaluate a SELECT over the `n` table and return the joined first row.
fn scalar_on(e: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> String {
    e.execute(sql)
        .unwrap()
        .rows
        .into_iter()
        .next()
        .map(|row| {
            row.into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

/// Evaluate a literal-only SELECT (no FROM, no NULL operand).
fn scalar(sql: &str) -> String {
    let mut e = engine();
    scalar_on(&mut e, sql)
}

/// Same, over a table holding one non-NULL row (`v = 7`).
fn int_engine() -> ExecutionEngine<MemoryStorage> {
    let mut e = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    e.execute("CREATE TABLE i7 (v INT)").unwrap();
    e.execute("INSERT INTO i7 VALUES (7)").unwrap();
    e
}
// --------------------------------------------------------------------------
// NOT with NULL
// --------------------------------------------------------------------------

/// Previously `eval_unary_op` fed Value::Null through to_bool(), which
/// returned false, and `!false` → true. SQL standard: NOT UNKNOWN = UNKNOWN.
#[test]
fn not_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT NOT v FROM n"), "NULL");
}

#[test]
fn not_not_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT NOT NOT v FROM n"), "NULL");
}

// --------------------------------------------------------------------------
// Comparisons with NULL
// --------------------------------------------------------------------------

/// NULL = NULL should be NULL (unknown), not FALSE.
#[test]
fn null_equals_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v = v FROM n"), "NULL");
}

/// NULL <> 1 should be NULL (unknown), not TRUE.
#[test]
fn null_not_equals_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v <> 1 FROM n"), "NULL");
}

/// 1 = NULL should be NULL (unknown), not FALSE.
#[test]
fn int_equals_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT 1 = v FROM n"), "NULL");
}

/// NULL > 5 should be NULL (unknown), not FALSE.
#[test]
fn null_greater_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v > 5 FROM n"), "NULL");
}

/// NULL <= NULL should be NULL (unknown), not FALSE.
#[test]
fn null_lessequal_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v <= v FROM n"), "NULL");
}

/// A non-NULL comparison is unaffected: 7 > 5 is TRUE.
#[test]
fn int_comparison_still_true() {
    let mut e = int_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v > 5 FROM i7"), "true");
}

// --------------------------------------------------------------------------
// AND with NULL
//
// Boolean AND in column position needs `Token::And` in the parser's
// `is_operator` whitelist; without it the column loop parsed the left
// operand and then choked on `AND`.

#[test]
fn false_and_null_is_false() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT 0 AND v FROM n"), "false");
}

#[test]
fn true_and_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT 1 AND v FROM n"), "NULL");
}

#[test]
fn null_and_true_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v AND 1 FROM n"), "NULL");
}

#[test]
fn null_and_false_is_false() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v AND 0 FROM n"), "false");
}

#[test]
fn null_and_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v AND v FROM n"), "NULL");
}

// --------------------------------------------------------------------------
// OR with NULL (three-valued truth table)
// --------------------------------------------------------------------------

/// TRUE OR NULL = TRUE (TRUE dominates).
#[test]
fn true_or_null_is_true() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v OR 1 FROM n"), "true");
}

/// FALSE OR NULL = NULL (UNKNOWN propagates).
#[test]
fn false_or_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v OR 0 FROM n"), "NULL");
}

/// NULL OR TRUE = TRUE (TRUE dominates).
#[test]
fn null_or_true_is_true() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT 1 OR v FROM n"), "true");
}

/// NULL OR FALSE = NULL (UNKNOWN propagates).
#[test]
fn null_or_false_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT 0 OR v FROM n"), "NULL");
}

/// NULL OR NULL = NULL (UNKNOWN propagates).
#[test]
fn null_or_null_is_null() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v OR v FROM n"), "NULL");
}

// --------------------------------------------------------------------------
// Mixed expressions
// --------------------------------------------------------------------------

/// (NULL OR 1) = TRUE — OR dominates over UNKNOWN.
#[test]
fn complex_null_or_true_is_true() {
    let mut e = null_engine();
    assert_eq!(scalar_on(&mut e, "SELECT v OR 1 FROM n"), "true");
}

// --------------------------------------------------------------------------
// NOT on non-null values (regression: should still work)
// --------------------------------------------------------------------------

#[test]
fn not_true_is_false() {
    assert_eq!(scalar("SELECT NOT 1"), "false");
}

#[test]
fn not_false_is_true() {
    assert_eq!(scalar("SELECT NOT 0"), "true");
}

#[test]
fn not_string_is_false() {
    assert_eq!(scalar("SELECT NOT 'x'"), "false");
}

#[test]
fn not_expr_is_false() {
    assert_eq!(scalar("SELECT NOT (1 = 1)"), "false");
}
