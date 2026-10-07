//! Fix 5 regression pins (priority 3 — invariant): these statement_kind
//! dispatch arms (`CREATE_ROLE` / `DROP_ROLE` / `SET_ROLE` / `SHOW` for
//! `SHOW ROLES` / `VALUES`) are runtime-reachable ONLY when the lexer
//! emits `Token::Role`/`Token::Roles` and `parse_statement` constructs
//! `Statement::Values`. The pre-existing "coverage" tests for these paths
//! either parsed `""` or discarded the `Result` (`let _ = result;`), so a
//! missing lexer keyword arm stayed invisible while the tests stayed green.

use sqlrustgo_parser::{parse, Statement};

#[test]
fn create_role_parses_via_lexer_keyword() {
    let stmt = parse("CREATE ROLE admin").expect("CREATE ROLE must parse");
    assert!(
        matches!(stmt, Statement::CreateRole(_)),
        "expected CreateRole, got {:?}",
        stmt
    );
}

#[test]
fn drop_role_parses_via_lexer_keyword() {
    let stmt = parse("DROP ROLE admin").expect("DROP ROLE must parse");
    assert!(
        matches!(stmt, Statement::DropRole(_)),
        "expected DropRole, got {:?}",
        stmt
    );
}

#[test]
fn set_role_parses_via_lexer_keyword() {
    // The SET dispatcher peeks `Token::Role` to distinguish SET ROLE from
    // SET <session var>; without the keyword arm this silently became a
    // session-variable parse error.
    let stmt = parse("SET ROLE admin").expect("SET ROLE must parse");
    assert!(
        matches!(stmt, Statement::SetRole(_)),
        "expected SetRole, got {:?}",
        stmt
    );
}

#[test]
fn show_roles_parses_via_lexer_keyword() {
    // SHOW dispatcher matches `Token::Roles`; without the arm
    // `SHOW ROLES` fell through as an unexpected token.
    let stmt = parse("SHOW ROLES").expect("SHOW ROLES must parse");
    assert!(
        matches!(stmt, Statement::ShowRoles),
        "expected ShowRoles, got {:?}",
        stmt
    );
}

#[test]
fn grant_role_constructs_grant_role_variant() {
    // Pins 5c: GRANT/REVOKE role statements route through dedicated
    // variants (they never need Token::Role — the role name is a bare
    // identifier), so statement_kind's `Grant(_) | GrantRole(_) => "GRANT"`
    // or-pattern has a reachable right-hand side.
    let stmt = parse("GRANT admin TO user1").expect("GRANT role must parse");
    assert!(
        matches!(stmt, Statement::GrantRole(_)),
        "expected GrantRole, got {:?}",
        stmt
    );
}

#[test]
fn revoke_role_constructs_revoke_role_variant() {
    let stmt = parse("REVOKE admin FROM user1").expect("REVOKE role must parse");
    assert!(
        matches!(stmt, Statement::RevokeRole(_)),
        "expected RevokeRole, got {:?}",
        stmt
    );
}

#[test]
fn standalone_values_statement_is_constructed() {
    // Production code previously never constructed `Statement::Values`
    // (top-level parse_statement had no `Token::Values` arm), leaving the
    // `Values(_) => "VALUES"` statement_kind arm and the executor's
    // explicit "VALUES cannot be used as a standalone statement" error
    // unreachable in practice — standalone `VALUES` surfaced as a raw
    // "Unexpected token" instead.
    let stmt = parse("VALUES (1, 2), (3, 4)")
        .expect("standalone VALUES must parse into Statement::Values");
    match stmt {
        Statement::Values(rows) => {
            assert_eq!(rows.len(), 2, "expected two value rows");
            assert_eq!(rows[0].len(), 2, "first row has two columns");
            assert_eq!(rows[1].len(), 2, "second row has two columns");
        }
        other => panic!("expected Statement::Values, got {:?}", other),
    }
}
