//! Tests for CBO-driven should_parallelize logic
//!
//! v3.10.0 Issue #3703: CBO determines when to parallelize

use sqlrustgo_optimizer::rules::{BinaryOperator, Expr};
use sqlrustgo_optimizer::unified_cost::UnifiedCostModel;
use sqlrustgo_optimizer::unified_plan::UnifiedPlan;

fn make_table_scan(table: &str) -> UnifiedPlan {
    UnifiedPlan::TableScan {
        table_name: table.to_string(),
        projection: None,
    }
}

fn make_filter(input: UnifiedPlan, predicate: Expr) -> UnifiedPlan {
    UnifiedPlan::Filter {
        input: Box::new(input),
        predicate,
    }
}

fn make_eq_expr(col: &str, val: &str) -> Expr {
    Expr::BinaryExpr {
        left: Box::new(Expr::Column(col.to_string())),
        op: BinaryOperator::Eq,
        right: Box::new(Expr::Literal(val.to_string())),
    }
}

fn make_lt_expr(col: &str, val: &str) -> Expr {
    Expr::BinaryExpr {
        left: Box::new(Expr::Column(col.to_string())),
        op: BinaryOperator::Lt,
        right: Box::new(Expr::Literal(val.to_string())),
    }
}

#[test]
fn test_should_parallelize_small_table() {
    // 100K rows - below PARALLEL_MIN_ROWS (500K)
    let mut model = UnifiedCostModel::default_model(128, 10000);
    model.update_table_stats("t".to_string(), 100_000, 1_000);
    let plan = make_table_scan("t");
    assert!(!model.should_parallelize(&plan));
}

#[test]
fn test_should_parallelize_large_table() {
    // 3M rows - above PARALLEL_MIN_ROWS (2M)
    let mut model = UnifiedCostModel::default_model(128, 10000);
    model.update_table_stats("t".to_string(), 3_000_000, 10_000);
    let plan = make_table_scan("t");
    assert!(model.should_parallelize(&plan));
}

#[test]
fn test_should_parallelize_for_update_disables() {
    // Even with large table, FOR UPDATE disables parallel
    // v3.10.0 Issue #3792: updated to 3M rows (above new PARALLEL_MIN_ROWS=2M)
    let mut model = UnifiedCostModel::default_model(128, 10000);
    model.update_table_stats("t".to_string(), 3_000_000, 10_000);
    let plan = make_table_scan("t");
    assert!(!model.should_parallelize_with(&plan, true));
    // Without FOR UPDATE, should be parallel
    assert!(model.should_parallelize_with(&plan, false));
}

#[test]
fn test_should_parallelize_unknown_table() {
    // No stats available - default to NOT parallel (conservative)
    let model = UnifiedCostModel::default_model(128, 10000);
    let plan = make_table_scan("unknown");
    // Unknown table - default row count; just ensure no panic
    let _ = model.should_parallelize(&plan);
}

#[test]
fn test_estimate_selectivity_eq() {
    let model = UnifiedCostModel::default_model(128, 10000);
    let expr = make_eq_expr("id", "5");
    let selectivity = model.estimate_selectivity_from_expr(&expr);
    assert!((selectivity - 0.1).abs() < 0.001);
}

#[test]
fn test_estimate_selectivity_lt() {
    let model = UnifiedCostModel::default_model(128, 10000);
    let expr = make_lt_expr("k", "5000");
    let selectivity = model.estimate_selectivity_from_expr(&expr);
    assert!((selectivity - 0.3).abs() < 0.001);
}

#[test]
fn test_estimate_selectivity_and() {
    let model = UnifiedCostModel::default_model(128, 10000);
    let expr = Expr::And(
        Box::new(make_eq_expr("a", "1")),
        Box::new(make_eq_expr("b", "2")),
    );
    // 0.1 * 0.1 = 0.01
    let selectivity = model.estimate_selectivity_from_expr(&expr);
    assert!((selectivity - 0.01).abs() < 0.001);
}

#[test]
fn test_estimate_selectivity_or() {
    let model = UnifiedCostModel::default_model(128, 10000);
    let expr = Expr::Or(
        Box::new(make_eq_expr("a", "1")),
        Box::new(make_eq_expr("b", "2")),
    );
    // 0.1 + 0.1 - 0.1*0.1 = 0.19
    let selectivity = model.estimate_selectivity_from_expr(&expr);
    assert!((selectivity - 0.19).abs() < 0.001);
}

#[test]
fn test_estimate_selectivity_not() {
    let model = UnifiedCostModel::default_model(128, 10000);
    let expr = Expr::Not(Box::new(make_eq_expr("a", "1")));
    // 1.0 - 0.1 = 0.9
    let selectivity = model.estimate_selectivity_from_expr(&expr);
    assert!((selectivity - 0.9).abs() < 0.001);
}

#[test]
fn test_is_compute_bound_high_selectivity_large_input() {
    // 1M rows with 30% selectivity = 300K output - above 100K threshold
    let mut model = UnifiedCostModel::default_model(128, 10000);
    model.update_table_stats("t".to_string(), 1_000_000, 10_000);
    let filter = make_filter(make_table_scan("t"), make_lt_expr("k", "5000"));
    assert!(model.should_parallelize(&filter));
}

#[test]
fn test_is_compute_bound_high_selectivity_small_input() {
    // 100K rows with 30% selectivity = 30K output - below 100K threshold
    let mut model = UnifiedCostModel::default_model(128, 10000);
    model.update_table_stats("t".to_string(), 100_000, 1_000);
    let filter = make_filter(make_table_scan("t"), make_lt_expr("k", "5000"));
    // Input too small to start with, so not parallel
    assert!(!model.should_parallelize(&filter));
}
