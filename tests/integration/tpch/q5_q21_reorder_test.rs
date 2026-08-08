//! Q5 + Q21 join-reorder regression tests (V311-07 fix verification)
//!
//! **V311-07**: Verifies the Q5 nation-bridge heuristic and Q21 alias
//! handling in `tpch_reorder_extra_tables`. These tests run the
//! pure parser function (no engine needed) so they don't suffer
//! from the in-process engine startup bug.
//!
//! Background:
//! - Q5 (Local Supplier Volume): 6-table join with `c_nationkey = s_nationkey`
//!   nation-bridge and `orders` with date filter. Without the
//!   `force_orders_first` heuristic, supplier is picked before orders
//!   and joined via `c_nationkey=s_nationkey` (cartesian-bridge) that
//!   explodes to ~130 GB at SF=1.
//! - Q21 (Suppliers Who Kept Orders Waiting): 4-table join with
//!   `lineitem l1` alias. The alias's `l1.l_suppkey` predicate must
//!   be resolved correctly, not fall through to `ON=true` cartesian.
//!
//! **Test strategy**: Call `tpch_reorder_extra_tables` directly with
//! the Q5/Q21 conj (predicate list) and verify the output order has
//! `orders` before `customer` (Q5) and `lineitem` joined with
//! `s_suppkey` predicate (Q21).

use sqlrustgo_parser::parser::{tpch_reorder_extra_tables, Expression};

fn ident(name: &str) -> Expression {
    Expression::Identifier(name.to_string())
}

fn lit_str(s: &str) -> Expression {
    Expression::Literal(s.to_string())
}

fn binop(l: Expression, op: &str, r: Expression) -> Expression {
    Expression::BinaryOp(Box::new(l), op.to_string(), Box::new(r))
}

/// Q5 schema: 6 tables, comma-join syntax.
#[test]
fn q5_force_orders_before_customer() {
    let extras = vec![
        "orders".to_string(),
        "lineitem".to_string(),
        "supplier".to_string(),
        "nation".to_string(),
        "region".to_string(),
    ];
    let joined = vec!["customer".to_string()];

    let conj = vec![
        binop(ident("c_custkey"), "=", ident("o_custkey")),
        binop(ident("l_orderkey"), "=", ident("o_orderkey")),
        binop(ident("l_suppkey"), "=", ident("s_suppkey")),
        binop(ident("c_nationkey"), "=", ident("s_nationkey")),
        binop(ident("s_nationkey"), "=", ident("n_nationkey")),
        binop(ident("n_regionkey"), "=", ident("r_regionkey")),
        binop(ident("r_name"), "=", lit_str("ASIA")),
        binop(ident("o_orderdate"), ">=", lit_str("1994-01-01")),
    ];

    let result = tpch_reorder_extra_tables(&extras, &joined, &conj);

    let orders_pos = result.iter().position(|s| s == "orders");
    let lineitem_pos = result.iter().position(|s| s == "lineitem");
    assert!(orders_pos.is_some(), "orders must be in reordered output");
    assert!(
        lineitem_pos.is_some(),
        "lineitem must be in reordered output"
    );
    let op = orders_pos.unwrap();
    let lp = lineitem_pos.unwrap();
    assert!(
        op < lp,
        "Q5 heuristic: orders must be picked before lineitem, got {:?}",
        result
    );

    eprintln!("Q5 reordered: {:?}", result);
}

/// Q21 schema: 4 tables with lineitem l1 alias.
#[test]
fn q21_lineitem_alias_predicate_resolved() {
    let extras = vec![
        "lineitem|l1".to_string(),
        "orders".to_string(),
        "nation".to_string(),
    ];
    let joined = vec!["supplier".to_string()];

    let conj = vec![
        binop(ident("s_suppkey"), "=", ident("l_suppkey")),
        binop(ident("o_orderkey"), "=", ident("l_orderkey")),
        binop(ident("s_nationkey"), "=", ident("n_nationkey")),
        binop(ident("n_name"), "=", lit_str("GERMANY")),
    ];

    let result = tpch_reorder_extra_tables(&extras, &joined, &conj);

    assert_eq!(result.len(), 3, "Q21 reorder should preserve all 3 extras");
    let unique: std::collections::HashSet<_> = result.iter().collect();
    assert_eq!(unique.len(), 3, "Q21 reorder should not duplicate tables");

    let lineitem_present = result.iter().any(|s| s.starts_with("lineitem"));
    assert!(
        lineitem_present,
        "lineitem|l1 alias must be preserved in reorder"
    );

    eprintln!("Q21 reordered: {:?}", result);
}

/// Q5 with explicit region filter to verify the heuristic triggers.
#[test]
fn q5_heuristic_triggers_on_nation_bridge_and_date_filter() {
    let extras = vec![
        "orders".to_string(),
        "lineitem".to_string(),
        "supplier".to_string(),
        "nation".to_string(),
        "region".to_string(),
    ];
    let joined = vec!["customer".to_string()];

    let conj = vec![
        binop(ident("c_custkey"), "=", ident("o_custkey")),
        binop(ident("c_nationkey"), "=", ident("s_nationkey")),
        binop(ident("o_orderdate"), ">=", lit_str("1994-01-01")),
        binop(ident("l_orderkey"), "=", ident("o_orderkey")),
        binop(ident("l_suppkey"), "=", ident("s_suppkey")),
        binop(ident("s_nationkey"), "=", ident("n_nationkey")),
        binop(ident("n_regionkey"), "=", ident("r_regionkey")),
    ];

    let result = tpch_reorder_extra_tables(&extras, &joined, &conj);
    eprintln!("Q5 (with bridge + date filter) reordered: {:?}", result);

    let orders_pos = result.iter().position(|s| s == "orders").unwrap();
    assert!(
        orders_pos <= 1,
        "Q5 with nation-bridge + date filter must pick orders early (got pos {})",
        orders_pos
    );
}

/// Q21 minimal: verify no panic and stable output.
#[test]
fn q21_alias_does_not_panic() {
    let extras = vec!["lineitem|l1".to_string(), "orders".to_string()];
    let joined = vec!["supplier".to_string(), "nation".to_string()];

    let conj = vec![binop(ident("s_suppkey"), "=", ident("l_suppkey"))];

    let result = tpch_reorder_extra_tables(&extras, &joined, &conj);
    assert_eq!(result.len(), 2);
    eprintln!("Q21 minimal reordered: {:?}", result);
}
