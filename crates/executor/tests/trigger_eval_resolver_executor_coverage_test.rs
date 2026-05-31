//! White-box coverage tests for trigger_eval/resolver.rs
//!
//! This file tests the `resolve_column` function and related ResolutionContext usage.
//! Target: 0% -> 75%+ region coverage

use sqlrustgo_executor::trigger_eval::context::{EvalContext, TriggerContext};
use sqlrustgo_executor::trigger_eval::resolver::resolve_column;
use sqlrustgo_types::Value;

/// Helper to make a simple Record from a slice of i64 values.
fn make_record(values: &[i64]) -> sqlrustgo_storage::Record {
    values.iter().map(|&v| Value::Integer(v)).collect()
}

#[test]
fn test_resolve_column_returns_null() {
    // resolve_column is a stub that always returns Value::Null
    // but we test it to establish baseline coverage
    let trigger_ctx = TriggerContext::new(None, None);
    let eval_ctx = EvalContext::new(&trigger_ctx, None);
    let result = resolve_column("any_column", &eval_ctx);
    assert!(matches!(result, Value::Null));
}

#[test]
fn test_resolve_column_with_various_names() {
    let trigger_ctx = TriggerContext::new(None, None);
    let eval_ctx = EvalContext::new(&trigger_ctx, None);

    // Any column name returns Null (stub behavior)
    assert!(matches!(resolve_column("col1", &eval_ctx), Value::Null));
    assert!(matches!(resolve_column("a", &eval_ctx), Value::Null));
    assert!(matches!(resolve_column("NEW.id", &eval_ctx), Value::Null));
    assert!(matches!(
        resolve_column("OLD.amount", &eval_ctx),
        Value::Null
    ));
    assert!(matches!(resolve_column("", &eval_ctx), Value::Null));
}

#[test]
fn test_resolve_column_with_target_row() {
    let trigger_ctx = TriggerContext::new(None, None);
    let target = make_record(&[10, 20, 30]);
    let eval_ctx = EvalContext::new(&trigger_ctx, Some(&target)).with_target_col_names(vec![
        "x".into(),
        "y".into(),
        "z".into(),
    ]);

    // Stub still returns Null (function is not yet fully implemented)
    assert!(matches!(resolve_column("x", &eval_ctx), Value::Null));
    assert!(matches!(resolve_column("y", &eval_ctx), Value::Null));
}

#[test]
fn test_resolve_column_with_new_row() {
    let new_record = make_record(&[1, 100, 5000]);
    let trigger_ctx = TriggerContext::new(Some(&new_record), None).with_new_col_names(vec![
        "id".into(),
        "amt".into(),
        "total".into(),
    ]);
    let eval_ctx = EvalContext::new(&trigger_ctx, None);

    // Stub returns Null regardless of trigger context
    assert!(matches!(resolve_column("id", &eval_ctx), Value::Null));
    assert!(matches!(resolve_column("NEW.id", &eval_ctx), Value::Null));
}

#[test]
fn test_resolve_column_with_old_row() {
    let old_record = make_record(&[1, 50, 3000]);
    let trigger_ctx = TriggerContext::new(None, Some(&old_record)).with_old_col_names(vec![
        "id".into(),
        "amt".into(),
        "total".into(),
    ]);
    let eval_ctx = EvalContext::new(&trigger_ctx, None);

    assert!(matches!(resolve_column("OLD.id", &eval_ctx), Value::Null));
    assert!(matches!(resolve_column("amt", &eval_ctx), Value::Null));
}
