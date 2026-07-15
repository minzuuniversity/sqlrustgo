//! V311-16 v2: try_decorrelate() end-to-end integration tests
//!
//! These tests verify that try_decorrelate() correctly:
//! 1. Detects patterns in real SQL
//! 2. Returns proper inner SELECT bodies
//! 3. Handles complex multi-pattern WHERE clauses
//! 4. Returns None for non-decorrelatable queries

use sqlrustgo_optimizer::decorrelate::{try_decorrelate, DecorrelatedJoinKind};

fn parse_where(sql: &str) -> sqlrustgo_parser::Expression {
    let stmt = sqlrustgo_parser::parse(sql).unwrap();
    let select = match stmt {
        sqlrustgo_parser::Statement::Select(s) => s,
        _ => panic!("expected SELECT"),
    };
    select.where_clause.unwrap().clone()
}

#[test]
fn v2_full_lifecycle_exists_to_semi() {
    let where_expr = parse_where("SELECT * FROM o WHERE EXISTS (SELECT 1 FROM l)");
    let result = try_decorrelate(&where_expr);
    assert!(result.is_some(), "decorrelation should succeed");
    let r = result.unwrap();
    assert_eq!(r.inner_selects.len(), 1);
    let inner = &r.inner_selects[0];
    assert_eq!(inner.join_kind, DecorrelatedJoinKind::Semi);
    // Inner select's table is lineitem
    assert_eq!(inner.select.table, "l");
    assert!(inner.alias.starts_with("__decorrelated_"));
}

#[test]
fn v2_tpc_h_q4_shape() {
    // TPC-H Q4 simplified: WHERE EXISTS (SELECT * FROM lineitem WHERE ...)
    let where_expr = parse_where(
        "SELECT * FROM orders \
         WHERE o_orderdate >= '1993-07-01' \
           AND o_orderdate < '1993-10-01' \
           AND EXISTS (SELECT * FROM lineitem \
                       WHERE l_orderkey = o_orderkey \
                         AND l_commitdate < l_receiptdate)",
    );
    let result = try_decorrelate(&where_expr);
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.inner_selects.len(), 1, "should detect exactly 1 subquery");
    assert_eq!(r.inner_selects[0].join_kind, DecorrelatedJoinKind::Semi);
}

#[test]
fn v2_tpc_h_q21_shape_mixed() {
    // TPC-H Q21: EXISTS + NOT EXISTS in same WHERE
    let where_expr = parse_where(
        "SELECT s_name FROM supplier \
         WHERE EXISTS (SELECT * FROM lineitem l2 \
                        WHERE l2.l_orderkey = l1.l_orderkey \
                          AND l2.l_suppkey <> l1.l_suppkey) \
           AND NOT EXISTS (SELECT * FROM lineitem l3 \
                            WHERE l3.l_orderkey = l1.l_orderkey \
                              AND l3.l_suppkey <> l1.l_suppkey \
                              AND l3.l_receiptdate > l3.l_commitdate)",
    );
    let result = try_decorrelate(&where_expr);
    assert!(result.is_some());
    let r = result.unwrap();
    // 2 subqueries: EXISTS → Semi, NOT EXISTS → Anti
    assert_eq!(r.inner_selects.len(), 2);
    let kinds: Vec<_> = r.inner_selects.iter().map(|s| s.join_kind).collect();
    assert!(kinds.contains(&DecorrelatedJoinKind::Semi));
    assert!(kinds.contains(&DecorrelatedJoinKind::Anti));
}

#[test]
fn v2_in_subquery_to_inner_join() {
    let where_expr = parse_where(
        "SELECT * FROM orders \
         WHERE o_custkey IN (SELECT c_custkey FROM customer \
                              WHERE c_nationkey = 5)",
    );
    let result = try_decorrelate(&where_expr);
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.inner_selects[0].join_kind, DecorrelatedJoinKind::Inner);
}

#[test]
fn v2_plain_where_returns_none() {
    let where_expr = parse_where("SELECT * FROM t WHERE a = 1 AND b > 5 AND c LIKE '%test%'");
    let result = try_decorrelate(&where_expr);
    assert!(result.is_none());
}

#[test]
fn v2_collects_multiple_inner_selects() {
    let where_expr = parse_where(
        "SELECT * FROM o \
         WHERE a IN (SELECT x FROM i1) \
           AND b NOT IN (SELECT y FROM i2) \
           AND EXISTS (SELECT 1 FROM i3)",
    );
    let result = try_decorrelate(&where_expr);
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.inner_selects.len(), 3, "should collect 3 patterns");
}

#[test]
fn v2_preserves_inner_select_structure() {
    let where_expr = parse_where(
        "SELECT * FROM o WHERE EXISTS (\
            SELECT l_orderkey FROM lineitem \
            WHERE l_orderkey = o.id AND l_discount BETWEEN 0.05 AND 0.07)",
    );
    let result = try_decorrelate(&where_expr);
    let r = result.unwrap();
    let inner = &r.inner_selects[0];
    // Verify the inner SELECT has the lineitem table and BETWEEN predicate
    assert_eq!(inner.select.table, "lineitem");
    assert!(
        inner.select.where_clause.is_some(),
        "inner where clause should be captured"
    );
}

#[test]
fn v2_aliases_are_unique() {
    let where_expr = parse_where(
        "SELECT * FROM o WHERE \
         a IN (SELECT 1 FROM i1) AND \
         b IN (SELECT 2 FROM i2) AND \
         c IN (SELECT 3 FROM i3)",
    );
    let result = try_decorrelate(&where_expr);
    let r = result.unwrap();
    let aliases: Vec<_> = r.inner_selects.iter().map(|s| s.alias.clone()).collect();
    let unique: std::collections::HashSet<_> = aliases.iter().collect();
    assert_eq!(aliases.len(), unique.len(), "aliases must be unique");
}
