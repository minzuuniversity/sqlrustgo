//! Statement Whitebox Tests — ISSUE #2627
//!
//! 白盒测试覆盖 crates/planner/src/statement.rs
//!
//! statement.rs 定义了 MergeStatement 和 MergeClause 结构体及其构造函数
//! 验收: cargo test -p sqlrustgo-planner --test statement_tests -- --test-threads=1

use sqlrustgo_parser::{Expression, SelectStatement};
use sqlrustgo_planner::{MergeClause, MergeStatement};

// ============ MergeStatement 白盒测试 ============

#[test]
fn test_merge_statement_new() {
    // 测试 MergeStatement::new() 完整构造
    let on_cond = Expression::Identifier("id".to_string());
    let matched = MergeClause::new(
        vec!["col1".to_string()],
        vec![Expression::Literal("1".to_string())],
        vec![],
        vec![],
    );
    let stmt = MergeStatement::new(
        "target_table".to_string(),
        "source_table".to_string(),
        on_cond.clone(),
        Some(matched),
        None,
    );

    assert_eq!(stmt.target_table, "target_table");
    assert_eq!(stmt.source_table, "source_table");
    assert!(matches!(stmt.on_condition, Expression::Identifier(_)));
    assert!(stmt.matched_clause.is_some());
    assert!(stmt.not_matched_clause.is_none());
}

#[test]
fn test_merge_statement_with_both_clauses() {
    // 测试 MergeStatement 同时有 matched 和 not_matched clause
    let matched = MergeClause::new(
        vec!["name".to_string()],
        vec![Expression::Literal("updated".to_string())],
        vec![],
        vec![],
    );
    let not_matched = MergeClause::new(
        vec![],
        vec![],
        vec!["name".to_string(), "value".to_string()],
        vec![
            Expression::Literal("new".to_string()),
            Expression::Literal("100".to_string()),
        ],
    );

    let stmt = MergeStatement::new(
        "orders".to_string(),
        "changes".to_string(),
        Expression::Identifier("id".to_string()),
        Some(matched),
        Some(not_matched),
    );

    assert_eq!(stmt.target_table, "orders");
    assert_eq!(stmt.source_table, "changes");
    assert!(stmt.matched_clause.is_some());
    assert!(stmt.not_matched_clause.is_some());
}

#[test]
fn test_merge_statement_without_clauses() {
    // 测试 MergeStatement 两个 clause 都为 None
    let stmt = MergeStatement::new(
        "t1".to_string(),
        "t2".to_string(),
        Expression::Identifier("x".to_string()),
        None,
        None,
    );

    assert_eq!(stmt.target_table, "t1");
    assert!(stmt.matched_clause.is_none());
    assert!(stmt.not_matched_clause.is_none());
}

#[test]
fn test_merge_statement_clone() {
    // 测试 Clone trait
    let stmt = MergeStatement::new(
        "target".to_string(),
        "source".to_string(),
        Expression::Identifier("id".to_string()),
        None,
        None,
    );
    let cloned = stmt.clone();
    assert_eq!(cloned.target_table, stmt.target_table);
    assert_eq!(cloned.source_table, stmt.source_table);
}

// ============ MergeClause 白盒测试 ============

#[test]
fn test_merge_clause_new() {
    // 测试 MergeClause::new() 完整构造
    let clause = MergeClause::new(
        vec!["update_col1".to_string(), "update_col2".to_string()],
        vec![
            Expression::Literal("42".to_string()),
            Expression::Literal("hello".to_string()),
        ],
        vec!["insert_col1".to_string()],
        vec![Expression::Literal("3.14".to_string())],
    );

    assert_eq!(clause.update_columns.len(), 2);
    assert_eq!(clause.update_values.len(), 2);
    assert_eq!(clause.insert_columns.len(), 1);
    assert_eq!(clause.insert_values.len(), 1);
}

#[test]
fn test_merge_clause_empty_update() {
    // 测试 MergeClause 仅 insert（not matched）场景
    let clause = MergeClause::new(
        vec![],
        vec![],
        vec!["col1".to_string(), "col2".to_string()],
        vec![
            Expression::Literal("1".to_string()),
            Expression::Literal("new_row".to_string()),
        ],
    );

    assert!(clause.update_columns.is_empty());
    assert!(clause.update_values.is_empty());
    assert_eq!(clause.insert_columns.len(), 2);
    assert_eq!(clause.insert_values.len(), 2);
}

#[test]
fn test_merge_clause_empty_insert() {
    // 测试 MergeClause 仅 update（matched）场景
    let clause = MergeClause::new(
        vec!["status".to_string()],
        vec![Expression::Literal("processed".to_string())],
        vec![],
        vec![],
    );

    assert_eq!(clause.update_columns.len(), 1);
    assert_eq!(clause.update_values.len(), 1);
    assert!(clause.insert_columns.is_empty());
    assert!(clause.insert_values.is_empty());
}

#[test]
fn test_merge_clause_clone() {
    // 测试 Clone trait
    let clause = MergeClause::new(
        vec!["a".to_string()],
        vec![Expression::Literal("1".to_string())],
        vec!["b".to_string()],
        vec![Expression::Literal("2".to_string())],
    );
    let cloned = clause.clone();
    assert_eq!(cloned.update_columns, clause.update_columns);
    assert_eq!(cloned.update_values.len(), clause.update_values.len());
    assert_eq!(cloned.insert_columns, clause.insert_columns);
    assert_eq!(cloned.insert_values.len(), clause.insert_values.len());
}

#[test]
fn test_merge_clause_debug() {
    // 测试 Debug trait
    let clause = MergeClause::new(
        vec!["id".to_string()],
        vec![Expression::Literal("1".to_string())],
        vec![],
        vec![],
    );
    let debug_str = format!("{:?}", clause);
    assert!(debug_str.contains("MergeClause"));
}

// ============ MergeStatement Debug 测试 ============

#[test]
fn test_merge_statement_debug() {
    // 测试 Debug trait
    let stmt = MergeStatement::new(
        "t".to_string(),
        "s".to_string(),
        Expression::Identifier("x".to_string()),
        None,
        None,
    );
    let debug_str = format!("{:?}", stmt);
    assert!(debug_str.contains("MergeStatement"));
}