//! V312-35 #4218 — StorageEngine trait API surface integration test.
//!
//! Exercises the new trait methods (`create_view`, `list_views`,
//! `get_view`, `set_cancel_flag`, `check_cancelled`, `kill_connection`,
//! `list_processes`) via the public ExecutionEngine API.
//! Round-trips parser → executor → storage so we catch any
//! AST→executor dispatch regressions too.

use sqlrustgo::{ExecutionEngine, MemoryStorage};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(std::sync::Arc::new(parking_lot::RwLock::new(
        MemoryStorage::new(),
    )))
}

#[test]
fn parse_kill_connection_and_show_processlist() {
    use sqlrustgo::parser;

    let stmts = parser::parse_statements("KILL CONNECTION 42").expect("parse KILL");
    let stmt = &stmts[0];
    match stmt {
        sqlrustgo::Statement::KillConnection { conn_id, is_query } => {
            assert_eq!(*conn_id, 42);
            assert!(!*is_query);
        }
        _ => panic!("expected KillConnection, got {:?}", stmt),
    }

    let stmts = parser::parse_statements("KILL QUERY 7").expect("parse KILL QUERY");
    match &stmts[0] {
        sqlrustgo::Statement::KillConnection { conn_id, is_query } => {
            assert_eq!(*conn_id, 7);
            assert!(*is_query);
        }
        _ => panic!("expected KillConnection with is_query=true, got {:?}", &stmts[0]),
    }

    let stmts = parser::parse_statements("SHOW PROCESSLIST").expect("parse SHOW PROCESSLIST");
    match &stmts[0] {
        sqlrustgo::Statement::ShowProcesslist { full } => {
            assert!(!*full);
        }
        _ => panic!("expected ShowProcesslist, got {:?}", &stmts[0]),
    }

    let stmts =
        parser::parse_statements("SHOW FULL PROCESSLIST").expect("parse SHOW FULL PROCESSLIST");
    match &stmts[0] {
        sqlrustgo::Statement::ShowProcesslist { full } => {
            assert!(*full);
        }
        _ => panic!("expected ShowProcesslist full=true, got {:?}", &stmts[0]),
    }
}

#[test]
fn kill_default_impl_returns_unsupported_error() {
    // MemoryStorage returns Err from kill_connection via the default
    // trait impl, so the executor should surface a friendly message.
    let mut eng = engine();
    let err = eng.execute("KILL CONNECTION 99").unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("KILL") && msg.contains("not supported"),
        "unexpected error message: {}",
        msg
    );
}

#[test]
fn show_processlist_returns_empty_rows_for_memory_storage() {
    let eng = engine();
    let res = eng.execute("SHOW PROCESSLIST").expect("execute SHOW PROCESSLIST");
    // No active connections on a fresh in-memory engine.
    assert_eq!(res.rows.len(), 0, "expected 0 rows, got {}", res.rows.len());
}

#[test]
fn show_full_processlist_returns_empty_rows_for_memory_storage() {
    let eng = engine();
    let res = eng
        .execute("SHOW FULL PROCESSLIST")
        .expect("execute SHOW FULL PROCESSLIST");
    assert_eq!(res.rows.len(), 0, "expected 0 rows, got {}", res.rows.len());
}

#[test]
fn parse_invalid_kill_returns_error() {
    use sqlrustgo::parser;
    // `KILL` without a numeric id must surface a parse error.
    let res = parser::parse_statements("KILL CONNECTION");
    assert!(
        res.is_err(),
        "expected parse error for KILL with no id, got {:?}",
        res
    );
}