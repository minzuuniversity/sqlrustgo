//! FOR UPDATE / LOCK IN SHARE MODE parsing integration tests

use sqlrustgo_parser::{parse, LockClause, SelectStatement, Statement};

#[test]
fn test_parse_for_update_clause() {
    let r = parse("SELECT * FROM t WHERE id = 1 FOR UPDATE").unwrap();
    match r {
        Statement::Select(s) => {
            let lock = s.lock_clause.expect("FOR UPDATE should set lock_clause");
            assert!(lock.for_update, "Should be FOR UPDATE");
            assert!(!lock.skip_locked);
            assert!(!lock.nowait);
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_parse_for_update_skip_locked() {
    let r = parse("SELECT * FROM t FOR UPDATE SKIP LOCKED");
    eprintln!("DEBUG: result = {:?}", r);
    let r = r.unwrap();
    match r {
        Statement::Select(s) => {
            eprintln!("DEBUG: lock_clause = {:?}", s.lock_clause);
            let lock = s.lock_clause.expect("FOR UPDATE should set lock_clause");
            assert!(lock.for_update);
            assert!(lock.skip_locked, "skip_locked should be true");
        }
        _ => panic!("Expected SELECT"),
    }
}

#[test]
fn test_parse_for_update_nowait() {
    let r = parse("SELECT * FROM t FOR UPDATE NOWAIT");
    let r = r.unwrap();
    match r {
        Statement::Select(s) => {
            let lock = s
                .lock_clause
                .expect("FOR UPDATE NOWAIT should set lock_clause");
            assert!(lock.for_update);
            assert!(lock.nowait, "nowait should be true");
        }
        _ => panic!("Expected SELECT"),
    }
}

#[test]
fn test_parse_no_lock_clause() {
    let r = parse("SELECT * FROM t WHERE id = 1").unwrap();
    match r {
        Statement::Select(s) => {
            assert!(
                s.lock_clause.is_none(),
                "Regular SELECT should have no lock clause"
            );
        }
        _ => panic!("Expected SELECT"),
    }
}

#[test]
fn test_lock_clause_default() {
    let lock = LockClause::default();
    assert!(!lock.for_update);
    assert!(!lock.skip_locked);
    assert!(!lock.nowait);
}

#[test]
fn test_select_statement_has_lock_clause_field() {
    let s = SelectStatement {
        columns: vec![],
        table: "t".to_string(),
        schema: None,
        from_alias: None,
        from_subquery: None,
        from_values: None,
        from_function_args: None,
        where_clause: None,
        join_clause: vec![],
        extra_tables: vec![],
        aggregates: vec![],
        group_by: vec![],
        with_rollup: false,
        with_cube: false,
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        distinct: false,
        lock_clause: None,
        index_hints: vec![],
    };
    assert!(s.lock_clause.is_none());
}
