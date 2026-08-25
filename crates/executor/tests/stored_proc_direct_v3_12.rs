//! Coverage tests for `sqlrustgo_executor::stored_proc::ProcedureContext`
//! direct public API (Issue #4431 followup — B8 COVERAGE_MIN_PER_CRATE).

use sqlrustgo_executor::stored_proc::{ProcedureContext, StoredProcError};
use sqlrustgo_types::Value;

#[test]
fn cov_ctx_new_empty() {
    let ctx = ProcedureContext::new();
    assert!(ctx.get_return().is_none());
    assert!(!ctx.should_leave());
}

#[test]
fn cov_ctx_local_vars() {
    let mut ctx = ProcedureContext::new();
    ctx.set_local_var("x", Value::Integer(42));
    assert_eq!(ctx.get_local_var("x"), Some(&Value::Integer(42)));
    assert_eq!(ctx.get_local_var("missing"), None);
}

#[test]
fn cov_ctx_session_vars() {
    let mut ctx = ProcedureContext::new();
    ctx.set_session_var("s", Value::Text("v".to_string()));
    assert_eq!(
        ctx.get_session_var("s"),
        Some(&Value::Text("v".to_string()))
    );
    let _ = ctx.get_session_vars();
}

#[test]
fn cov_ctx_get_var_falls_back_to_session() {
    let mut ctx = ProcedureContext::new();
    ctx.set_session_var("both", Value::Integer(1));
    assert_eq!(ctx.get_var("both"), Some(&Value::Integer(1)));
}

#[test]
fn cov_ctx_return_value() {
    let mut ctx = ProcedureContext::new();
    ctx.set_return(Value::Integer(7));
    assert_eq!(ctx.get_return(), Some(Value::Integer(7)));
}

#[test]
fn cov_ctx_leave_flag() {
    let mut ctx = ProcedureContext::new();
    ctx.set_leave();
    assert!(ctx.should_leave());
    ctx.reset_leave();
    assert!(!ctx.should_leave());
}

#[test]
fn cov_ctx_iterate_flag() {
    let mut ctx = ProcedureContext::new();
    ctx.set_iterate();
    assert!(ctx.should_iterate());
    ctx.reset_iterate();
    assert!(!ctx.should_iterate());
}

#[test]
fn cov_ctx_labels() {
    let mut ctx = ProcedureContext::new();
    ctx.enter_label("outer".to_string());
    assert!(ctx.has_label("outer"));
    assert_eq!(ctx.get_label(), Some(&"outer".to_string()));
    ctx.exit_label();
    assert!(!ctx.has_label("outer"));
}

#[test]
fn cov_ctx_scopes() {
    let mut ctx = ProcedureContext::new();
    ctx.enter_scope();
    ctx.exit_scope();
}

#[test]
fn cov_cursor_lifecycle() {
    let mut ctx = ProcedureContext::new();
    assert!(!ctx.has_cursor("c"));
    ctx.declare_cursor("c".to_string(), "SELECT 1".to_string());
    assert!(ctx.has_cursor("c"));
    ctx.open_cursor("c").unwrap();
    let _ = ctx.fetch_cursor("c", &[]);
    ctx.close_cursor("c").unwrap();
}

#[test]
fn cov_cursor_open_missing_is_err() {
    let mut ctx = ProcedureContext::new();
    assert!(ctx.open_cursor("nope").is_err());
}

#[test]
fn cov_cursor_close_missing_is_err() {
    let mut ctx = ProcedureContext::new();
    assert!(ctx.close_cursor("nope").is_err());
}

#[test]
fn cov_cursor_set_records_and_fetch() {
    let mut ctx = ProcedureContext::new();
    ctx.declare_cursor("c".to_string(), "SELECT 1".to_string());
    ctx.set_cursor_records("c", vec![vec![Value::Integer(1)], vec![Value::Integer(2)]]);
    ctx.open_cursor("c").unwrap();
    let more = ctx.fetch_cursor("c", &["out".to_string()]).unwrap();
    assert!(more);
}

#[test]
fn cov_exception_set_clear() {
    let mut ctx = ProcedureContext::new();
    ctx.set_exception("45000".to_string(), "boom".to_string());
    let exc = ctx.get_exception();
    assert!(exc.is_some());
    ctx.clear_exception();
    assert!(ctx.get_exception().is_none());
}

#[test]
fn cov_exception_handling_flag() {
    let mut ctx = ProcedureContext::new();
    ctx.set_exception_handling(true);
    assert!(ctx.is_handling_exception());
    ctx.set_exception_handling(false);
    assert!(!ctx.is_handling_exception());
}

#[test]
fn cov_stored_proc_error_display() {
    let e = StoredProcError {
        sqlstate: "45000".to_string(),
        message: "custom".to_string(),
    };
    let s = format!("{}", e);
    assert!(s.contains("45000"));
    assert!(s.contains("custom"));
}
