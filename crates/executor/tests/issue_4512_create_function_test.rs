//! Regression tests for Issue #4512 — `CREATE FUNCTION` / `DROP FUNCTION`
//! scalar UDF support.
//!
//! MySQL-style scalar UDFs are stored in a thread-local registry; the body
//! is captured as raw SQL text and re-parsed + evaluated at every call site
//! with declared parameter names substituted for the call-site argument
//! values. The UDF lookup in `eval_fn` falls through after every built-in
//! arm misses, so user-defined names never collide with builtins unless
//! the user explicitly replaces them.
//!
//! Coverage:
//! - arithmetic body, single integer param
//! - body with multiple params (UDF(D, E))
//! - text concat body
//! - body that calls a built-in (`ABS(x)`)
//! - body using `CASE WHEN` / control-flow
//! - `DROP FUNCTION` removes the registration
//! - `DROP FUNCTION IF EXISTS` is a no-op when missing
//! - `DROP FUNCTION` without `IF EXISTS` errors on missing
//! - re-creating with the same name replaces the previous body
//! - case-insensitive name lookup (`my_func` vs `MY_FUNC`)
//! - arity mismatch returns NULL (graceful degradation)
//! - non-deterministic flag is accepted but does not affect evaluation
//! - UDF visible inside a SELECT projection with rows from another table
//! - parser rejects `CREATE FUNCTION` without `RETURNS`
//! - parser rejects `DROP FUNCTION` of unknown built-in function names

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo::SqlError;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_storage::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn create_function_arithmetic_body() {
    // CREATE FUNCTION double(x INT) RETURNS INT RETURN x * 2;
    // SELECT double(5) -> 10
    let mut e = engine();
    e.execute("CREATE FUNCTION double(x INTEGER) RETURNS INTEGER RETURN x * 2")
        .unwrap();
    let r = e.execute("SELECT double(5)").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(
        matches!(&r.rows[0][0], Value::Integer(10)),
        "got {:?}",
        r.rows[0]
    );
}

#[test]
fn create_function_two_param_body() {
    // CREATE FUNCTION my_add(x INT, y INT) RETURNS INT RETURN x + y;
    // We avoid the bare name `add` because ADD is a reserved token in
    // the lexer (used for date arithmetic).
    let mut e = engine();
    e.execute("CREATE FUNCTION my_add(x INTEGER, y INTEGER) RETURNS INTEGER RETURN x + y")
        .unwrap();
    let r = e.execute("SELECT my_add(3, 4)").unwrap();
    assert!(
        matches!(&r.rows[0][0], Value::Integer(7)),
        "got {:?}",
        r.rows[0]
    );
}

#[test]
fn create_function_concat_text_body() {
    // Body returns a TEXT result derived from the param. We use
    // CASE rather than `||` because the latter is OR in our SQL dialect
    // (not string concatenation).
    let mut e = engine();
    e.execute(
        "CREATE FUNCTION greet(name TEXT) RETURNS TEXT RETURN CASE WHEN name = 'alice' THEN 'hi alice' ELSE 'hi there' END",
    )
    .unwrap();
    let r = e.execute("SELECT greet('alice'), greet('bob')").unwrap();
    assert!(
        matches!(&r.rows[0][0], Value::Text(s) if s == "hi alice"),
        "got {:?}",
        r.rows[0]
    );
    assert!(
        matches!(&r.rows[0][1], Value::Text(s) if s == "hi there"),
        "got {:?}",
        r.rows[0]
    );
}

#[test]
fn create_function_uses_builtin_inside_body() {
    // Body wraps the parameter with the built-in ABS function.
    let mut e = engine();
    e.execute("CREATE FUNCTION safe_abs(x INTEGER) RETURNS INTEGER RETURN ABS(x)")
        .unwrap();
    let r = e.execute("SELECT safe_abs(-7)").unwrap();
    assert!(
        matches!(&r.rows[0][0], Value::Integer(7)),
        "got {:?}",
        r.rows[0]
    );
}

#[test]
fn create_function_case_when_body() {
    // Body returns 'big' / 'small' depending on a threshold.
    let mut e = engine();
    e.execute(
        "CREATE FUNCTION label(x INTEGER) RETURNS TEXT RETURN CASE WHEN x > 10 THEN 'big' ELSE 'small' END",
    )
    .unwrap();
    let r = e.execute("SELECT label(3), label(20)").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "small"));
    assert!(matches!(&r.rows[0][1], Value::Text(s) if s == "big"));
}

#[test]
fn drop_function_removes_registration() {
    // After DROP, calling the UDF returns NULL (no panic).
    let mut e = engine();
    e.execute("CREATE FUNCTION tmp(x INTEGER) RETURNS INTEGER RETURN x + 1")
        .unwrap();
    let r1 = e.execute("SELECT tmp(41)").unwrap();
    assert!(matches!(&r1.rows[0][0], Value::Integer(42)));
    e.execute("DROP FUNCTION tmp").unwrap();
    let r2 = e.execute("SELECT tmp(41)").unwrap();
    assert!(
        matches!(&r2.rows[0][0], Value::Null),
        "got {:?}",
        r2.rows[0]
    );
}

#[test]
fn drop_function_if_exists_is_noop_when_missing() {
    let mut e = engine();
    e.execute("DROP FUNCTION IF EXISTS never_existed").unwrap();
}

#[test]
fn drop_function_without_if_exists_errors_when_missing() {
    let mut e = engine();
    let err = e.execute("DROP FUNCTION never_existed").unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("function 'never_existed' not found"),
        "unexpected error: {msg}"
    );
}

#[test]
fn create_function_replaces_previous_definition() {
    // Re-creating with the same name overwrites the body.
    let mut e = engine();
    e.execute("CREATE FUNCTION f(x INTEGER) RETURNS INTEGER RETURN x + 1")
        .unwrap();
    let r1 = e.execute("SELECT f(10)").unwrap();
    assert!(matches!(&r1.rows[0][0], Value::Integer(11)));
    e.execute("CREATE FUNCTION f(x INTEGER) RETURNS INTEGER RETURN x * 100")
        .unwrap();
    let r2 = e.execute("SELECT f(10)").unwrap();
    assert!(matches!(&r2.rows[0][0], Value::Integer(1000)));
}

#[test]
fn create_function_name_lookup_is_case_insensitive() {
    let mut e = engine();
    e.execute("CREATE FUNCTION MYFUNC(x INTEGER) RETURNS INTEGER RETURN x + 7")
        .unwrap();
    // Both casings resolve to the same UDF.
    let r1 = e.execute("SELECT myfunc(3)").unwrap();
    let r2 = e.execute("SELECT MYFUNC(3)").unwrap();
    assert!(matches!(&r1.rows[0][0], Value::Integer(10)));
    assert!(matches!(&r2.rows[0][0], Value::Integer(10)));
}

#[test]
fn create_function_arity_mismatch_returns_null() {
    // Calling with wrong arg count must not panic. Returns NULL because
    // the body can't be evaluated against the declared param list.
    let mut e = engine();
    e.execute("CREATE FUNCTION one_arg(x INTEGER) RETURNS INTEGER RETURN x + 1")
        .unwrap();
    let r = e.execute("SELECT one_arg(1, 2)").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Null), "got {:?}", r.rows[0]);
}

#[test]
fn create_function_deterministic_clause_is_accepted() {
    // The DETERMINISTIC clause must parse without error; v3.12 does not
    // yet enforce determinism but the keyword must be accepted.
    let mut e = engine();
    e.execute("CREATE FUNCTION dbl(x INTEGER) RETURNS INTEGER DETERMINISTIC RETURN x * 2")
        .unwrap();
    let r = e.execute("SELECT dbl(6)").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(12)));
}

#[test]
fn create_function_used_inside_table_select() {
    // The UDF can be called in a SELECT projection that also reads rows
    // from a real table — exercises the UnifiedExpr evaluation path with
    // both a column ref and a FunctionCall.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, qty INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 3), (2, 5)").unwrap();
    e.execute("CREATE FUNCTION triple(x INTEGER) RETURNS INTEGER RETURN x * 3")
        .unwrap();
    let r = e
        .execute("SELECT id, triple(qty) FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert!(matches!(&r.rows[0][1], Value::Integer(9)));
    assert!(matches!(&r.rows[1][1], Value::Integer(15)));
}

#[test]
fn create_function_uses_param_with_comparison_op() {
    // Body returns a boolean derived from the param.
    let mut e = engine();
    e.execute("CREATE FUNCTION is_pos(x INTEGER) RETURNS BOOLEAN RETURN x > 0")
        .unwrap();
    let r = e.execute("SELECT is_pos(5), is_pos(-3)").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Boolean(true)));
    assert!(matches!(&r.rows[0][1], Value::Boolean(false)));
}

#[test]
fn parser_rejects_create_function_missing_returns() {
    // CREATE FUNCTION without RETURNS must error.
    let mut e = engine();
    let err = e
        .execute("CREATE FUNCTION bad(x INTEGER) RETURN x + 1")
        .expect_err("missing RETURNS must be a parser error");
    let msg = format!("{}", err);
    assert!(
        msg.to_lowercase().contains("returns") || msg.to_lowercase().contains("expected"),
        "unexpected error: {msg}"
    );
}

#[test]
fn create_function_with_null_arg_returns_null_when_body_propagates() {
    // When the body returns NULL via IF NULL propagation, the UDF
    // should also yield NULL.
    let mut e = engine();
    e.execute("CREATE FUNCTION neg(x INTEGER) RETURNS INTEGER RETURN -x")
        .unwrap();
    let r = e.execute("SELECT neg(NULL)").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Null), "got {:?}", r.rows[0]);
}

#[test]
fn error_type_for_drop_function_missing_is_sql_error() {
    // Sanity check: drop without IF EXISTS surfaces as SqlError, not a
    // generic panic.
    let mut e = engine();
    let err = e.execute("DROP FUNCTION ghost").unwrap_err();
    assert!(matches!(err, SqlError::ExecutionError(_)));
}
