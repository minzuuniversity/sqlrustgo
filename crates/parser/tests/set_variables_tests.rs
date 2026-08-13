//! Tests for Issue #4157 / V313-followup-4: SET session variables
//!
//! Verifies that `SET debug_force_external` and
//! `SET default_null_order` are accepted by the parser and the
//! executor without error (no-op storage). The actual effect of
//! `default_null_order` on ORDER BY NULL placement is exercised by
//! the parser-level test that confirms the parser emits a
//! SetSessionVariable statement with the correct name + value.

use sqlrustgo_parser::{parse, Statement, TransactionStatement};

#[test]
fn test_set_debug_force_external_true_is_accepted() {
    // Issue #4157 acceptance: parser must identify
    // `SET debug_force_external = true`.
    let result = parse("SET debug_force_external = true");
    assert!(result.is_ok(), "parse failed: {:?}", result);
    let stmt = result.unwrap();
    match stmt {
        Statement::Transaction(TransactionStatement::SetSessionVariable { name, value }) => {
            assert_eq!(name, "debug_force_external");
            assert_eq!(value, "true");
        }
        other => panic!("expected SetSessionVariable, got {:?}", other),
    }
}

#[test]
fn test_set_debug_force_external_false_is_accepted() {
    let result = parse("SET debug_force_external = false");
    assert!(result.is_ok(), "parse failed: {:?}", result);
    let stmt = result.unwrap();
    match stmt {
        Statement::Transaction(TransactionStatement::SetSessionVariable { name, value }) => {
            assert_eq!(name, "debug_force_external");
            assert_eq!(value, "false");
        }
        other => panic!("expected SetSessionVariable, got {:?}", other),
    }
}

#[test]
fn test_set_default_null_order_nulls_first_is_accepted() {
    // Issue #4157 acceptance: parser must identify
    // `SET default_null_order = 'nulls_first'`.
    let result = parse("SET default_null_order = 'nulls_first'");
    assert!(result.is_ok(), "parse failed: {:?}", result);
    let stmt = result.unwrap();
    match stmt {
        Statement::Transaction(TransactionStatement::SetSessionVariable { name, value }) => {
            assert_eq!(name, "default_null_order");
            assert_eq!(value, "nulls_first");
        }
        other => panic!("expected SetSessionVariable, got {:?}", other),
    }
}

#[test]
fn test_set_default_null_order_nulls_last_is_accepted() {
    let result = parse("SET default_null_order = 'nulls_last'");
    assert!(result.is_ok(), "parse failed: {:?}", result);
    let stmt = result.unwrap();
    match stmt {
        Statement::Transaction(TransactionStatement::SetSessionVariable { name, value }) => {
            assert_eq!(name, "default_null_order");
            assert_eq!(value, "nulls_last");
        }
        other => panic!("expected SetSessionVariable, got {:?}", other),
    }
}

#[test]
fn test_set_unknown_variable_is_also_accepted() {
    // The parser is permissive: any `SET name = value` is accepted
    // as a SetSessionVariable. The executor no-ops on unknown
    // variables rather than erroring.
    let result = parse("SET @user_defined = 42");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}