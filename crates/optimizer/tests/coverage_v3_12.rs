//! Coverage tests for `sqlrustgo_optimizer::decorrelate` public API
//! (Issue #4431 followup — B8 COVERAGE_MIN_PER_CRATE).
//!
//! Targets decorrelate.rs (57% lines, 358 missed) via direct calls to
//! count_decorrelatable / find_scalar_subqueries / try_decorrelate with
//! correlated and non-correlated expression shapes.

use sqlrustgo_optimizer::decorrelate::{
    count_decorrelatable, find_correlated_subqueries, find_scalar_subqueries, try_decorrelate,
};
use sqlrustgo_parser::Expression;

fn col(name: &str) -> Expression {
    Expression::Identifier(name.to_string())
}

fn int_lit(n: i64) -> Expression {
    Expression::Literal(format!("{}", n))
}

// --------------------------------------------------------------------------
// count_decorrelatable / find_correlated_subqueries
// --------------------------------------------------------------------------

#[test]
fn cov_count_zero_on_simple_comparison() {
    let expr = Expression::BinaryOp(Box::new(col("a")), "=".to_string(), Box::new(int_lit(1)));
    assert_eq!(count_decorrelatable(&expr), 0);
}

#[test]
fn cov_count_zero_on_literal() {
    assert_eq!(count_decorrelatable(&int_lit(5)), 0);
}

#[test]
fn cov_count_zero_on_column() {
    assert_eq!(count_decorrelatable(&col("x")), 0);
}

fn make_subquery() -> sqlrustgo_parser::SelectStatement {
    sqlrustgo_parser::SelectStatement {
        columns: vec![],
        table: "inner_t".to_string(),
        ..Default::default()
    }
}

#[test]
fn cov_find_exists_correlated() {
    let expr = Expression::Exists(Box::new(make_subquery()));
    let found = find_correlated_subqueries(&expr, &[]);
    assert_eq!(found.len(), 1);
}

#[test]
fn cov_count_exists_is_one() {
    let expr = Expression::Exists(Box::new(make_subquery()));
    assert_eq!(count_decorrelatable(&expr), 1);
}

#[test]
fn cov_find_not_exists() {
    let expr = Expression::NotExists(Box::new(make_subquery()));
    let found = find_correlated_subqueries(&expr, &[]);
    assert_eq!(found.len(), 1);
}

#[test]
fn cov_find_in_where_and_projection() {
    let where_expr = Expression::Exists(Box::new(make_subquery()));
    let proj = vec![Expression::Exists(Box::new(make_subquery()))];
    let found = find_correlated_subqueries(&where_expr, &proj);
    assert_eq!(found.len(), 2);
}

#[test]
fn cov_find_scalar_subqueries_zero_on_column() {
    assert_eq!(find_scalar_subqueries(&col("x")), 0);
}

#[test]
fn cov_try_decorrelate_none_on_uncorrelated() {
    let expr = Expression::BinaryOp(Box::new(col("a")), "=".to_string(), Box::new(int_lit(1)));
    assert!(try_decorrelate(&expr).is_none());
}
