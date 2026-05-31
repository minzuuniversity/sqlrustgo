use sqlrustgo_executor::stored_proc::{ProcedureContext, StoredProcError};
use sqlrustgo_types::Value;

#[test]
fn test_procedure_context_new() {
    let ctx = ProcedureContext::new();
    assert!(ctx.get_return().is_none());
    assert!(!ctx.should_leave());
    assert!(!ctx.should_iterate());
    assert!(ctx.get_label().is_none());
    assert!(!ctx.is_handling_exception());
    assert!(ctx.get_exception().is_none());
}

#[test]
fn test_set_and_get_local_var() {
    let mut ctx = ProcedureContext::new();
    ctx.set_local_var("x", Value::Integer(42));
    assert_eq!(ctx.get_local_var("x"), Some(&Value::Integer(42)));
    assert_eq!(ctx.get_var("x"), Some(&Value::Integer(42)));
    assert!(ctx.has_var("x"));
    assert!(!ctx.has_var("nonexistent"));
}

#[test]
fn test_set_and_get_session_var() {
    let mut ctx = ProcedureContext::new();
    ctx.set_session_var("uid", Value::Text("alice".to_string()));
    assert_eq!(
        ctx.get_session_var("uid"),
        Some(&Value::Text("alice".to_string()))
    );
    assert_eq!(ctx.get_var("@uid"), Some(&Value::Text("alice".to_string())));
    assert!(ctx.has_var("@uid"));
}

#[test]
fn test_set_var_auto_dispatch() {
    let mut ctx = ProcedureContext::new();
    ctx.set_var("local_x", Value::Integer(1));
    assert_eq!(ctx.get_local_var("local_x"), Some(&Value::Integer(1)));
    ctx.set_var("@session_y", Value::Float(3.14));
    assert_eq!(ctx.get_session_var("session_y"), Some(&Value::Float(3.14)));
}

#[test]
fn test_has_var_local_and_session() {
    let mut ctx = ProcedureContext::new();
    ctx.set_local_var("a", Value::Integer(1));
    ctx.set_session_var("b", Value::Integer(2));
    assert!(ctx.has_var("a"));
    assert!(ctx.has_var("@b"));
    assert!(!ctx.has_var("c"));
}

#[test]
fn test_get_var_prefers_local() {
    let mut ctx = ProcedureContext::new();
    ctx.set_local_var("dup", Value::Integer(1));
    ctx.set_session_var("dup", Value::Integer(2));
    assert_eq!(ctx.get_var("dup"), Some(&Value::Integer(1)));
}

#[test]
fn test_clear_local_vars() {
    let mut ctx = ProcedureContext::new();
    ctx.set_local_var("x", Value::Integer(42));
    ctx.set_local_var("y", Value::Text("hello".to_string()));
    ctx.clear_local_vars();
    assert!(ctx.get_local_var("x").is_none());
    assert!(ctx.get_local_var("y").is_none());
}

#[test]
fn test_get_session_vars_persistence() {
    let mut ctx = ProcedureContext::new();
    ctx.set_session_var("k1", Value::Integer(1));
    ctx.set_session_var("k2", Value::Text("v2".to_string()));
    let vars = ctx.get_session_vars();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars.get("k1"), Some(&Value::Integer(1)));
    assert_eq!(vars.get("k2"), Some(&Value::Text("v2".to_string())));
}

#[test]
fn test_return_value_lifecycle() {
    let mut ctx = ProcedureContext::new();
    assert!(ctx.get_return().is_none());
    ctx.set_return(Value::Integer(99));
    assert_eq!(ctx.get_return(), Some(Value::Integer(99)));
}

#[test]
fn test_return_value_overwrite() {
    let mut ctx = ProcedureContext::new();
    ctx.set_return(Value::Text("first".to_string()));
    ctx.set_return(Value::Text("second".to_string()));
    assert_eq!(ctx.get_return(), Some(Value::Text("second".to_string())));
}

#[test]
fn test_return_value_none() {
    let ctx = ProcedureContext::new();
    assert_eq!(ctx.get_return(), None);
}

#[test]
fn test_leave_behavior() {
    let mut ctx = ProcedureContext::new();
    assert!(!ctx.should_leave());
    ctx.set_leave();
    assert!(ctx.should_leave());
    ctx.reset_leave();
    assert!(!ctx.should_leave());
}

#[test]
fn test_iterate_behavior() {
    let mut ctx = ProcedureContext::new();
    assert!(!ctx.should_iterate());
    ctx.set_iterate();
    assert!(ctx.should_iterate());
    ctx.reset_iterate();
    assert!(!ctx.should_iterate());
}

#[test]
fn test_leave_and_iterate_independent() {
    let mut ctx = ProcedureContext::new();
    ctx.set_leave();
    ctx.set_iterate();
    assert!(ctx.should_leave());
    assert!(ctx.should_iterate());
    ctx.reset_leave();
    assert!(!ctx.should_leave());
    assert!(ctx.should_iterate());
}

#[test]
fn test_label_stack_enter_exit() {
    let mut ctx = ProcedureContext::new();
    assert!(ctx.get_label().is_none());
    ctx.enter_label("outer".to_string());
    assert!(ctx.has_label("outer"));
    assert_eq!(ctx.get_label(), Some(&"outer".to_string()));
    ctx.enter_label("inner".to_string());
    assert!(ctx.has_label("inner"));
    assert_eq!(ctx.get_label(), Some(&"inner".to_string()));
    ctx.exit_label();
    assert_eq!(ctx.get_label(), Some(&"outer".to_string()));
    ctx.exit_label();
    assert!(ctx.get_label().is_none());
}

#[test]
fn test_set_label() {
    let mut ctx = ProcedureContext::new();
    ctx.set_label(Some("loop1".to_string()));
    assert_eq!(ctx.get_label(), Some(&"loop1".to_string()));
    ctx.set_label(None);
    assert!(ctx.get_label().is_none());
}

#[test]
fn test_scope_stack_basic() {
    let mut ctx = ProcedureContext::new();
    ctx.set_local_var("x", Value::Integer(1));
    ctx.enter_scope();
    assert!(ctx.get_local_var("x").is_none());
    ctx.set_local_var("y", Value::Integer(2));
    ctx.exit_scope();
    assert_eq!(ctx.get_local_var("x"), Some(&Value::Integer(1)));
    assert!(ctx.get_local_var("y").is_none());
}

#[test]
fn test_scope_stack_nested() {
    let mut ctx = ProcedureContext::new();
    ctx.set_local_var("a", Value::Integer(1));
    ctx.enter_scope();
    ctx.set_local_var("b", Value::Integer(2));
    ctx.enter_scope();
    ctx.set_local_var("c", Value::Integer(3));
    assert!(ctx.get_local_var("a").is_none());
    assert!(ctx.get_local_var("b").is_none());
    ctx.exit_scope();
    assert_eq!(ctx.get_local_var("b"), Some(&Value::Integer(2)));
    ctx.exit_scope();
    assert_eq!(ctx.get_local_var("a"), Some(&Value::Integer(1)));
}

#[test]
fn test_cursor_declare_and_check() {
    let mut ctx = ProcedureContext::new();
    assert!(!ctx.has_cursor("my_cursor"));
    ctx.declare_cursor("my_cursor".to_string(), "SELECT * FROM t".to_string());
    assert!(ctx.has_cursor("my_cursor"));
}

#[test]
fn test_cursor_open() {
    let mut ctx = ProcedureContext::new();
    ctx.declare_cursor("c".to_string(), "SELECT id FROM users".to_string());
    assert!(ctx.open_cursor("c").is_ok());
    assert!(ctx.open_cursor("c").is_ok());
}

#[test]
fn test_cursor_open_not_found() {
    let mut ctx = ProcedureContext::new();
    let result = ctx.open_cursor("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_cursor_close() {
    let mut ctx = ProcedureContext::new();
    ctx.declare_cursor("c".to_string(), "SELECT 1".to_string());
    assert!(ctx.close_cursor("c").is_ok());
}

#[test]
fn test_cursor_close_not_found() {
    let mut ctx = ProcedureContext::new();
    assert!(ctx.close_cursor("ghost").is_err());
}

#[test]
fn test_cursor_fetch_empty() {
    let mut ctx = ProcedureContext::new();
    ctx.declare_cursor("c".to_string(), "SELECT id FROM t".to_string());
    ctx.open_cursor("c").unwrap();
    let has_rows = ctx.fetch_cursor("c", &["v".to_string()]).unwrap();
    assert!(!has_rows);
}

#[test]
fn test_cursor_fetch_with_records() {
    let mut ctx = ProcedureContext::new();
    ctx.declare_cursor("c".to_string(), "SELECT id FROM t".to_string());
    ctx.set_cursor_records(
        "c",
        vec![
            vec![Value::Integer(10)],
            vec![Value::Integer(20)],
            vec![Value::Integer(30)],
        ],
    );
    ctx.open_cursor("c").unwrap();
    let mut has_rows = ctx.fetch_cursor("c", &["val".to_string()]).unwrap();
    assert!(has_rows);
    assert_eq!(ctx.get_local_var("val"), Some(&Value::Integer(10)));
    has_rows = ctx.fetch_cursor("c", &["val".to_string()]).unwrap();
    assert!(has_rows);
    assert_eq!(ctx.get_local_var("val"), Some(&Value::Integer(20)));
    has_rows = ctx.fetch_cursor("c", &["val".to_string()]).unwrap();
    assert!(has_rows);
    assert_eq!(ctx.get_local_var("val"), Some(&Value::Integer(30)));
    has_rows = ctx.fetch_cursor("c", &["val".to_string()]).unwrap();
    assert!(!has_rows);
}

#[test]
fn test_cursor_fetch_not_open() {
    let mut ctx = ProcedureContext::new();
    ctx.declare_cursor("c".to_string(), "SELECT 1".to_string());
    let result = ctx.fetch_cursor("c", &["v".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_cursor_fetch_not_found() {
    let mut ctx = ProcedureContext::new();
    let result = ctx.fetch_cursor("ghost", &["v".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_handler_push_pop() {
    let mut ctx = ProcedureContext::new();
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "02000".to_string(),
            message: "not found".to_string()
        })
        .is_none());
    ctx.push_handler(sqlrustgo_catalog::HandlerCondition::NotFound, vec![]);
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "02000".to_string(),
            message: "not found".to_string()
        })
        .is_some());
    ctx.pop_handler();
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "02000".to_string(),
            message: "not found".to_string()
        })
        .is_none());
}

#[test]
fn test_handler_matching_sqlexception() {
    let mut ctx = ProcedureContext::new();
    ctx.push_handler(sqlrustgo_catalog::HandlerCondition::SqlException, vec![]);
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "45000".to_string(),
            message: "custom error".to_string(),
        })
        .is_some());
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "22000".to_string(),
            message: "data exception".to_string(),
        })
        .is_some());
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "02000".to_string(),
            message: "not found".to_string(),
        })
        .is_none());
}

#[test]
fn test_handler_matching_sqlwarning() {
    let mut ctx = ProcedureContext::new();
    ctx.push_handler(sqlrustgo_catalog::HandlerCondition::SqlWarning, vec![]);
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "01000".to_string(),
            message: "warning".to_string(),
        })
        .is_some());
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "45000".to_string(),
            message: "error".to_string(),
        })
        .is_none());
}

#[test]
fn test_handler_matching_sqlstate() {
    let mut ctx = ProcedureContext::new();
    ctx.push_handler(
        sqlrustgo_catalog::HandlerCondition::SqlState("45000".to_string()),
        vec![],
    );
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "45000".to_string(),
            message: "exact match".to_string(),
        })
        .is_some());
    assert!(ctx
        .find_matching_handler(&StoredProcError {
            sqlstate: "45001".to_string(),
            message: "different".to_string(),
        })
        .is_none());
}

#[test]
fn test_handler_matching_custom() {
    let mut ctx = ProcedureContext::new();
    ctx.push_handler(
        sqlrustgo_catalog::HandlerCondition::Custom("my_error".to_string()),
        vec![],
    );
    let matched = ctx.find_matching_handler(&StoredProcError {
        sqlstate: "45000".to_string(),
        message: "something my_error happened".to_string(),
    });
    assert!(matched.is_some());
    let not_matched = ctx.find_matching_handler(&StoredProcError {
        sqlstate: "45000".to_string(),
        message: "other error".to_string(),
    });
    assert!(not_matched.is_none());
}

#[test]
fn test_exception_lifecycle() {
    let mut ctx = ProcedureContext::new();
    assert!(ctx.get_exception().is_none());
    ctx.set_exception("22000".to_string(), "data error".to_string());
    let exc = ctx.get_exception().unwrap();
    assert_eq!(exc.sqlstate, "22000");
    assert_eq!(exc.message, "data error");
    ctx.clear_exception();
    assert!(ctx.get_exception().is_none());
}

#[test]
fn test_exception_overwrite() {
    let mut ctx = ProcedureContext::new();
    ctx.set_exception("45000".to_string(), "first".to_string());
    ctx.set_exception("02000".to_string(), "second".to_string());
    let exc = ctx.get_exception().unwrap();
    assert_eq!(exc.sqlstate, "02000");
    assert_eq!(exc.message, "second");
}

#[test]
fn test_exception_handling_mode() {
    let mut ctx = ProcedureContext::new();
    assert!(!ctx.is_handling_exception());
    ctx.set_exception_handling(true);
    assert!(ctx.is_handling_exception());
    ctx.set_exception_handling(false);
    assert!(!ctx.is_handling_exception());
}

#[test]
fn test_stored_proc_error_display() {
    let err = StoredProcError {
        sqlstate: "45000".to_string(),
        message: "custom error".to_string(),
    };
    assert_eq!(format!("{}", err), "SQLSTATE 45000: custom error");
}

#[test]
fn test_stored_proc_error_debug() {
    let err = StoredProcError {
        sqlstate: "01000".to_string(),
        message: "warning msg".to_string(),
    };
    let debug = format!("{:?}", err);
    assert!(debug.contains("01000"));
    assert!(debug.contains("warning msg"));
}

#[test]
fn test_stored_proc_error_clone() {
    let err = StoredProcError {
        sqlstate: "22000".to_string(),
        message: "clone test".to_string(),
    };
    let cloned = err.clone();
    assert_eq!(cloned.sqlstate, "22000");
    assert_eq!(cloned.message, "clone test");
}
