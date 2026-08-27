//! Regression tests for Issue #4511 — PREPARE / EXECUTE / USING with
//! MySQL session variables (`@name`).
//!
//! MySQL pre-compiles SQL bodies via `PREPARE name FROM '...'`, then runs
//! them via `EXECUTE name [USING @var, ...]`. The USING clause binds
//! session variables to the `?` placeholders in the prepared SQL (positional).
//!
//! Coverage:
//! - basic PREPARE + EXECUTE round-trip on the same body
//! - session var lookup via `SET @var = expr; SELECT @var`
//! - `EXECUTE name USING @var` with one positional placeholder
//! - `EXECUTE name USING @a, @b` with multiple positional placeholders
//! - session var appears in a WHERE filter
//! - unbound session var returns NULL (MySQL semantics)
//! - prepared statement can be executed multiple times
//! - too few / too many USING params return explicit errors
//! - DEALLOCATE PREPARE name clears the cached statement

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_storage::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn prepare_execute_without_using() {
    // Plain PREPARE + EXECUTE (no bind params) is the simple case.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, name TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    e.execute("PREPARE p FROM 'SELECT name FROM t WHERE id = 1'")
        .unwrap();
    let r = e.execute("EXECUTE p").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "alice"));
}

#[test]
fn session_var_set_then_select() {
    // SET @a = expr stores into the engine's session map; SELECT @a
    // returns the bound value rather than the legacy Value::Text("@a")
    // fallback.
    let mut e = engine();
    e.execute("SET @a = 42").unwrap();
    let r = e.execute("SELECT @a").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(&r.rows[0][0], Value::Integer(42)));
}

#[test]
fn session_var_text_then_select() {
    let mut e = engine();
    e.execute("SET @name = 'alice'").unwrap();
    let r = e.execute("SELECT @name").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "alice"));
}

#[test]
fn session_var_bool_then_select() {
    let mut e = engine();
    e.execute("SET @flag = true").unwrap();
    let r = e.execute("SELECT @flag").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Boolean(true)));
}

#[test]
fn session_var_null_literal() {
    let mut e = engine();
    e.execute("SET @n = NULL").unwrap();
    let r = e.execute("SELECT @n").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Null));
}

#[test]
fn unbound_session_var_returns_null() {
    // MySQL semantics: reading an unset @var returns NULL, not an error.
    let mut e = engine();
    let r = e.execute("SELECT @missing").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(&r.rows[0][0], Value::Null));
}

#[test]
fn execute_using_one_placeholder() {
    // PREPARE with `?` placeholder + EXECUTE USING binds the variable.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, name TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    e.execute("PREPARE p FROM 'SELECT name FROM t WHERE id = ?'")
        .unwrap();
    e.execute("SET @k = 2").unwrap();
    let r = e.execute("EXECUTE p USING @k").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "bob"));
}

#[test]
fn execute_using_two_placeholders_positional() {
    // Two USING params bind to the first and second `?` in source order.
    let mut e = engine();
    e.execute("CREATE TABLE t(a INTEGER, b INTEGER, c TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 10, 'one'), (2, 20, 'two'), (3, 30, 'three')")
        .unwrap();
    e.execute("PREPARE q FROM 'SELECT c FROM t WHERE a = ? AND b = ?'")
        .unwrap();
    e.execute("SET @x = 2").unwrap();
    e.execute("SET @y = 20").unwrap();
    let r = e.execute("EXECUTE q USING @x, @y").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "two"));
}

#[test]
fn execute_reuse_after_rebind() {
    // Same prepared statement, different params. Tests that the cache
    // doesn't capture values from the first execution.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, name TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob'), (3, 'carol')")
        .unwrap();
    e.execute("PREPARE p FROM 'SELECT name FROM t WHERE id = ?'")
        .unwrap();
    e.execute("SET @k = 1").unwrap();
    let r1 = e.execute("EXECUTE p USING @k").unwrap();
    assert!(matches!(&r1.rows[0][0], Value::Text(s) if s == "alice"));
    e.execute("SET @k = 3").unwrap();
    let r2 = e.execute("EXECUTE p USING @k").unwrap();
    assert!(matches!(&r2.rows[0][0], Value::Text(s) if s == "carol"));
}

#[test]
fn execute_using_string_placeholder() {
    let mut e = engine();
    e.execute("CREATE TABLE t(name TEXT, val INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('alice', 1), ('bob', 2)")
        .unwrap();
    e.execute("PREPARE q FROM 'SELECT val FROM t WHERE name = ?'")
        .unwrap();
    e.execute("SET @who = 'alice'").unwrap();
    let r = e.execute("EXECUTE q USING @who").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Integer(1)));
}

#[test]
fn execute_missing_prepared_returns_error() {
    let mut e = engine();
    let err = e.execute("EXECUTE nonexistent").unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("prepared statement 'nonexistent' not found"),
        "expected 'not found' error, got {msg}"
    );
}

#[test]
fn execute_using_with_too_few_params_errors() {
    let mut e = engine();
    e.execute("PREPARE p FROM 'SELECT ?'").unwrap();
    e.execute("SET @a = 1").unwrap();
    // Two `?` placeholders, only one USING var → explicit error.
    e.execute("PREPARE q FROM 'SELECT ?, ?'").unwrap();
    let err = e.execute("EXECUTE q USING @a").unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("not enough parameters"),
        "expected not-enough error, got {msg}"
    );
}

#[test]
fn execute_using_with_too_many_params_errors() {
    let mut e = engine();
    e.execute("SET @a = 1").unwrap();
    e.execute("SET @b = 2").unwrap();
    e.execute("PREPARE p FROM 'SELECT ?'").unwrap();
    // One `?`, two USING vars → excess var errors.
    let err = e.execute("EXECUTE p USING @a, @b").unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("too many parameters"),
        "expected too-many error, got {msg}"
    );
}

#[test]
fn deallocate_clears_prepared_statement() {
    let mut e = engine();
    e.execute("PREPARE p FROM 'SELECT 1'").unwrap();
    e.execute("EXECUTE p").unwrap();
    e.execute("DEALLOCATE PREPARE p").unwrap();
    let err = e.execute("EXECUTE p").unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("not found"),
        "expected not-found error after DEALLOCATE, got {msg}"
    );
}

#[test]
fn session_var_in_expression_arithmetic() {
    // Session var interpolated in arithmetic: `SELECT @a + 1` should
    // resolve `@a` then add 1.
    let mut e = engine();
    e.execute("SET @a = 10").unwrap();
    let r = e.execute("SELECT @a + 1").unwrap();
    assert!(matches!(&r.rows[0][0], Value::Integer(11)));
}

#[test]
fn session_var_in_where_clause() {
    // SELECT ... WHERE col = @var must use the bound session var.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, name TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    e.execute("SET @target = 'bob'").unwrap();
    let r = e.execute("SELECT id FROM t WHERE name = @target").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Integer(2)));
}
