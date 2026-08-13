//! VTU IR (Versioned Transactional Update Intermediate Representation)
//! integration tests for the three previously-untouched files:
//!
//! - `crates/storage/src/vtu_ir/update_plan.rs`  (UpdatePlan + PlanTrace)
//! - `crates/storage/src/vtu_ir/mutation_ir.rs`  (MutationIR + AssignmentIR)
//! - `crates/storage/src/vtu_ir/predicate_ir.rs` (PredicateIR + ExprIR)
//!
//! Closes task-3.2 of #3537 (Round 3 coverage plan).
//!
//! These exercise the public APIs from outside the crate so that coverage
//! measurement sees them hit from the integration test path. The unit tests
//! inside each `mod tests` already exercise in-crate paths; this file adds
//! end-to-end scenarios that compose multiple IR nodes together.

use sqlrustgo_storage::vtu_ir::{AssignmentIR, ExprIR, MutationIR, PredicateIR, UpdatePlan};
use sqlrustgo_storage::{ColumnDefinition, TableInfo, Value};

fn users_table() -> TableInfo {
    TableInfo {
        name: "users".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                ..Default::default()
            },
            ColumnDefinition {
                name: "active".to_string(),
                data_type: "BOOLEAN".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                ..Default::default()
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                ..Default::default()
            },
        ],
        compression: None,
// V312-26 / #4077: collations field added by d48b0a1a71;
        // this initializer was missed by the propagation PR #4140.
        collations: std::collections::HashMap::new(),
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    }
}

fn products_table() -> TableInfo {
    TableInfo {
        name: "products".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "sku".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                ..Default::default()
            },
            ColumnDefinition {
                name: "price".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                ..Default::default()
            },
            ColumnDefinition {
                name: "stock".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                ..Default::default()
            },
        ],
        compression: None,
// V312-26 / #4077: collations field added by d48b0a1a71;
        // this initializer was missed by the propagation PR #4140.
        collations: std::collections::HashMap::new(),
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    }
}

// ===========================================================================
// update_plan.rs (UpdatePlan + PlanTrace)
// ===========================================================================

#[test]
fn test_update_plan_no_predicate() {
    let mutation = MutationIR::new(vec![AssignmentIR {
        column: "active".to_string(),
        column_index: 1,
        expr: ExprIR::Literal(Value::Boolean(false)),
    }]);
    let plan = UpdatePlan::new("users".to_string(), PredicateIR::All, mutation, 0);

    assert_eq!(plan.table, "users");
    assert!(matches!(plan.predicate(), PredicateIR::All));
    assert_eq!(plan.mutation().assignments().len(), 1);
    assert_eq!(plan.mutation().assignments()[0].column, "active");
    assert_eq!(plan.trace().rows_affected(), 0);
    assert!(plan.trace().plan_id.starts_with("update_"));
    assert_eq!(plan.trace().mutation_hash, plan.mutation().mutation_hash());
}

#[test]
fn test_update_plan_with_predicate() {
    let predicate = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(42))),
    });
    let mutation = MutationIR::new(vec![AssignmentIR {
        column: "name".to_string(),
        column_index: 2,
        expr: ExprIR::Literal(Value::Text("alice".to_string())),
    }]);
    let plan = UpdatePlan::new("users".to_string(), predicate, mutation, 1);

    assert_eq!(plan.trace().rows_affected(), 1);
    // PredicateIR doesn't derive PartialEq — confirm structurally instead.
    assert!(matches!(plan.predicate(), PredicateIR::Expr(_)));
    assert_eq!(plan.mutation().assignments().len(), 1);
}

#[test]
fn test_plan_trace_constructor_hashes_are_deterministic_for_same_inputs() {
    // PlanTrace::new combines the two input hashes via DefaultHasher; we
    // verify two plans with identical inputs produce the same plan_id and
    // combined_hash, while distinct mutations produce different hashes.
    let pred1 = PredicateIR::Expr(ExprIR::Column("id".to_string()));
    let pred2 = PredicateIR::Expr(ExprIR::Column("name".to_string()));

    let m1 = MutationIR::new(vec![AssignmentIR {
        column: "name".to_string(),
        column_index: 2,
        expr: ExprIR::Literal(Value::Text("bob".to_string())),
    }]);

    let p1a = UpdatePlan::new("t".to_string(), pred1.clone(), m1.clone(), 5);
    let p1b = UpdatePlan::new("t".to_string(), pred1.clone(), m1.clone(), 5);
    assert_eq!(p1a.trace().plan_id, p1b.trace().plan_id);
    assert_eq!(p1a.trace().combined_hash, p1b.trace().combined_hash);

    let p2 = UpdatePlan::new("t".to_string(), pred2, m1, 5);
    assert_ne!(p1a.trace().combined_hash, p2.trace().combined_hash);
}

// ===========================================================================
// mutation_ir.rs (MutationIR + AssignmentIR)
// ===========================================================================

#[test]
fn test_mutation_ir_empty_assignments_returns_default_hasher_state() {
    // MutationIR hashes via DefaultHasher; even an empty assignment list
    // produces the hasher's initial state (which is non-zero). What matters
    // is that two empty mutations agree.
    let m = MutationIR::new(vec![]);
    assert!(m.assignments().is_empty());
    let m2 = MutationIR::new(vec![]);
    assert_eq!(m.mutation_hash(), m2.mutation_hash());
}

#[test]
fn test_mutation_ir_single_assignment_has_nonzero_hash() {
    let m = MutationIR::new(vec![AssignmentIR {
        column: "name".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Text("alice".to_string())),
    }]);
    assert_eq!(m.assignments().len(), 1);
    assert_ne!(m.mutation_hash(), 0);
}

#[test]
fn test_mutation_ir_hash_differs_for_column_index_change() {
    let a = AssignmentIR {
        column: "name".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Text("x".to_string())),
    };
    let b = AssignmentIR {
        column: "name".to_string(),
        column_index: 1, // different column_index -> different hash
        expr: ExprIR::Literal(Value::Text("x".to_string())),
    };
    let ma = MutationIR::new(vec![a]);
    let mb = MutationIR::new(vec![b]);
    assert_ne!(ma.mutation_hash(), mb.mutation_hash());
}

#[test]
fn test_mutation_ir_hash_differs_for_column_name_change() {
    let a = AssignmentIR {
        column: "name".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Text("x".to_string())),
    };
    let b = AssignmentIR {
        column: "nick".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Text("x".to_string())),
    };
    let ma = MutationIR::new(vec![a]);
    let mb = MutationIR::new(vec![b]);
    assert_ne!(ma.mutation_hash(), mb.mutation_hash());
}

#[test]
fn test_mutation_ir_to_row_mutation_uses_nulls() {
    let m = MutationIR::new(vec![
        AssignmentIR {
            column: "name".to_string(),
            column_index: 2,
            expr: ExprIR::Literal(Value::Text("alice".to_string())),
        },
        AssignmentIR {
            column: "active".to_string(),
            column_index: 1,
            expr: ExprIR::Literal(Value::Boolean(true)),
        },
    ]);
    let row = m.to_row_mutation();
    assert_eq!(row.assignments().len(), 2);
    // to_row_mutation drops the expression and writes Null placeholders keyed
    // by column_index. The hash is preserved.
    assert_eq!(row.mutation_hash(), m.mutation_hash());
}

// ===========================================================================
// predicate_ir.rs (PredicateIR + ExprIR) — end-to-end evaluation
// ===========================================================================

#[test]
fn test_predicate_all_evaluates_true_for_any_row() {
    let p = PredicateIR::All;
    let info = users_table();
    assert!(p.evaluate(&[], &info));
    assert!(p.evaluate(
        &[Value::Integer(1), Value::Boolean(false), Value::Null],
        &info
    ));
}

#[test]
fn test_predicate_column_with_truthy_boolean() {
    let info = users_table();
    let p = PredicateIR::Expr(ExprIR::Column("active".to_string()));
    assert!(p.evaluate(
        &[Value::Integer(1), Value::Boolean(true), Value::Null],
        &info
    ));
    assert!(!p.evaluate(
        &[Value::Integer(1), Value::Boolean(false), Value::Null],
        &info
    ));
}

#[test]
fn test_predicate_column_with_non_boolean_value_is_false() {
    let info = users_table();
    let p = PredicateIR::Expr(ExprIR::Column("id".to_string()));
    // id is INTEGER, not BOOLEAN — predicate returns false even though the
    // column exists. This is intentional in PredicateIR::eval_expr.
    assert!(!p.evaluate(
        &[Value::Integer(1), Value::Boolean(true), Value::Null],
        &info
    ));
}

#[test]
fn test_predicate_binary_eq_integer() {
    let info = users_table();
    let p = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(7))),
    });
    assert!(p.evaluate(
        &[Value::Integer(7), Value::Boolean(true), Value::Null],
        &info
    ));
    assert!(!p.evaluate(
        &[Value::Integer(8), Value::Boolean(true), Value::Null],
        &info
    ));
}

#[test]
fn test_predicate_binary_eq_text() {
    let info = users_table();
    let p = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("name".to_string())),
        right: Box::new(ExprIR::Literal(Value::Text("alice".to_string()))),
    });
    assert!(p.evaluate(
        &[
            Value::Integer(1),
            Value::Boolean(true),
            Value::Text("alice".to_string())
        ],
        &info
    ));
    assert!(!p.evaluate(
        &[
            Value::Integer(1),
            Value::Boolean(true),
            Value::Text("bob".to_string())
        ],
        &info
    ));
}

#[test]
fn test_predicate_binary_comparison_operators() {
    let info = users_table();
    let row = &[Value::Integer(5), Value::Boolean(true), Value::Null];

    let mk = |op: &str| {
        PredicateIR::Expr(ExprIR::Binary {
            op: op.to_string(),
            left: Box::new(ExprIR::Column("id".to_string())),
            right: Box::new(ExprIR::Literal(Value::Integer(5))),
        })
    };

    assert!(mk("=").evaluate(row, &info));
    assert!(!mk("!=").evaluate(row, &info));
    assert!(mk("<=").evaluate(row, &info));
    assert!(mk(">=").evaluate(row, &info));
    assert!(!mk("<").evaluate(row, &info));
    assert!(!mk(">").evaluate(row, &info));
}

#[test]
fn test_predicate_compound_and_or() {
    let info = users_table();
    let row = &[Value::Integer(1), Value::Boolean(true), Value::Null];

    let and = PredicateIR::Expr(ExprIR::Binary {
        op: "AND".to_string(),
        left: Box::new(ExprIR::Column("active".to_string())),
        right: Box::new(ExprIR::Column("active".to_string())),
    });
    assert!(and.evaluate(row, &info));

    let or = PredicateIR::Expr(ExprIR::Binary {
        op: "OR".to_string(),
        left: Box::new(ExprIR::Column("active".to_string())),
        right: Box::new(ExprIR::Column("active".to_string())),
    });
    assert!(or.evaluate(row, &info));
}

#[test]
fn test_predicate_isnull_isnotnull() {
    let info = users_table();
    let is_null = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column("name".to_string()))));
    assert!(is_null.evaluate(
        &[Value::Integer(1), Value::Boolean(true), Value::Null],
        &info
    ));
    assert!(!is_null.evaluate(
        &[
            Value::Integer(1),
            Value::Boolean(true),
            Value::Text("x".to_string())
        ],
        &info
    ));

    let is_not_null = PredicateIR::Expr(ExprIR::IsNotNull(Box::new(ExprIR::Column(
        "name".to_string(),
    ))));
    assert!(!is_not_null.evaluate(
        &[Value::Integer(1), Value::Boolean(true), Value::Null],
        &info
    ));
    assert!(is_not_null.evaluate(
        &[
            Value::Integer(1),
            Value::Boolean(true),
            Value::Text("x".to_string())
        ],
        &info
    ));
}

#[test]
fn test_predicate_null_short_circuits_comparison() {
    let info = users_table();
    // Right side is Null — comparison must return false (not error, not
    // implicitly true).
    let p = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("name".to_string())),
        right: Box::new(ExprIR::Literal(Value::Null)),
    });
    assert!(!p.evaluate(
        &[Value::Integer(1), Value::Boolean(true), Value::Null],
        &info
    ));
}

// ===========================================================================
// Composed scenarios — exercising IR types together
// ===========================================================================

#[test]
fn test_full_update_plan_filters_then_mutates_in_evaluation() {
    // Build a complete UpdatePlan with a WHERE clause that selects a single
    // row, then verify evaluate() against multiple rows picks only the match.
    let info = products_table();

    // Predicate: sku = 'ABC'
    let predicate = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("sku".to_string())),
        right: Box::new(ExprIR::Literal(Value::Text("ABC".to_string()))),
    });
    // Mutation: stock = stock (placeholder expression, column_index 2)
    let mutation = MutationIR::new(vec![AssignmentIR {
        column: "stock".to_string(),
        column_index: 2,
        expr: ExprIR::Column("stock".to_string()),
    }]);
    let plan = UpdatePlan::new("products".to_string(), predicate, mutation, 1);

    // Three rows — only ABC should be matched.
    let rows = [
        vec![
            Value::Text("ABC".to_string()),
            Value::Integer(10),
            Value::Integer(100),
        ],
        vec![
            Value::Text("XYZ".to_string()),
            Value::Integer(20),
            Value::Integer(200),
        ],
        vec![
            Value::Text("DEF".to_string()),
            Value::Integer(30),
            Value::Integer(300),
        ],
    ];
    let mut matched = 0;
    for row in &rows {
        if plan.predicate().evaluate(row, &info) {
            matched += 1;
        }
    }
    assert_eq!(matched, 1, "exactly one row matches sku='ABC'");
    assert_eq!(plan.mutation().assignments().len(), 1);
}

#[test]
fn test_predicate_ir_equality_via_hash_and_partialeq() {
    // ExprIR derives PartialEq + Eq + Hash — confirm it round-trips through
    // both. Useful for plan de-duplication and cache keys.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let a = ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(1))),
    };
    let b = ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(1))),
    };
    assert_eq!(a, b);

    let mut ha = DefaultHasher::new();
    a.hash(&mut ha);
    let mut hb = DefaultHasher::new();
    b.hash(&mut hb);
    assert_eq!(ha.finish(), hb.finish());
}
