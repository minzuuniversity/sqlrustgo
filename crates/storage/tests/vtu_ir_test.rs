//! White-box tests for VTU IR modules (mutation_ir, predicate_ir, update_plan)
//!
//! These tests exercise the internal logic of the VTU IR components.

use sqlrustgo_storage::vtu_ir::{
    AssignmentIR, ExprIR, MutationIR, PlanTrace, PredicateIR, UpdatePlan,
};
use sqlrustgo_types::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Test MutationIR::new with various assignments
#[test]
fn test_mutation_ir_new() {
    let assignments = vec![
        AssignmentIR {
            column: "id".to_string(),
            column_index: 0,
            expr: ExprIR::Literal(Value::Integer(42)),
        },
        AssignmentIR {
            column: "name".to_string(),
            column_index: 1,
            expr: ExprIR::Literal(Value::Text("test".to_string())),
        },
    ];

    let mutation = MutationIR::new(assignments.clone());
    assert_eq!(mutation.assignments().len(), 2);
    assert_eq!(mutation.assignments()[0].column, "id");
    assert_eq!(mutation.assignments()[1].column, "name");
    assert_ne!(mutation.mutation_hash(), 0);
}

/// Test MutationIR::new with empty assignments
/// Note: DefaultHasher on empty input still produces a non-zero hash
#[test]
fn test_mutation_ir_empty() {
    let mutation = MutationIR::new(vec![]);
    assert_eq!(mutation.assignments().len(), 0);
    // DefaultHasher on empty produces non-zero hash
    assert_ne!(mutation.mutation_hash(), 0);
}

/// Test MutationIR::to_row_mutation
#[test]
fn test_mutation_ir_to_row_mutation() {
    let assignments = vec![
        AssignmentIR {
            column: "col1".to_string(),
            column_index: 0,
            expr: ExprIR::Literal(Value::Integer(100)),
        },
        AssignmentIR {
            column: "col2".to_string(),
            column_index: 2,
            expr: ExprIR::Literal(Value::Text("value".to_string())),
        },
    ];

    let mutation = MutationIR::new(assignments);
    let row_mutation = mutation.to_row_mutation();

    // Verify the row mutation has correct assignments (index, Value::Null)
    let assigns = row_mutation.assignments();
    assert_eq!(assigns.len(), 2);
    assert_eq!(assigns[0].0, 0); // column_index
    assert_eq!(assigns[1].0, 2); // column_index
    assert!(matches!(assigns[0].1, Value::Null));
    assert!(matches!(assigns[1].1, Value::Null));
}

/// Test MutationIR hash consistency - same assignments should produce same hash
#[test]
fn test_mutation_ir_hash_consistency() {
    let assignments1 = vec![AssignmentIR {
        column: "id".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(1)),
    }];
    let assignments2 = vec![AssignmentIR {
        column: "id".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(1)),
    }];

    let mutation1 = MutationIR::new(assignments1);
    let mutation2 = MutationIR::new(assignments2);

    assert_eq!(mutation1.mutation_hash(), mutation2.mutation_hash());
}

/// Test MutationIR hash differs for different assignments
#[test]
fn test_mutation_ir_hash_differs() {
    let assignments1 = vec![AssignmentIR {
        column: "id".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(1)),
    }];
    let assignments2 = vec![AssignmentIR {
        column: "id".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(2)),
    }];

    let mutation1 = MutationIR::new(assignments1);
    let mutation2 = MutationIR::new(assignments2);

    assert_ne!(mutation1.mutation_hash(), mutation2.mutation_hash());
}

// =============================================================================
// PredicateIR Tests
// =============================================================================

use sqlrustgo_storage::engine::TableInfo;

/// Helper to create a simple TableInfo for testing
fn make_table_info(columns: Vec<(&str, &str)>) -> TableInfo {
    TableInfo {
        name: "test_table".to_string(),
        columns: columns
            .into_iter()
            .map(|(n, t)| sqlrustgo_storage::ColumnDefinition {
                name: n.to_string(),
                data_type: t.to_string(),
                nullable: false,
                primary_key: false,
            })
            .collect(),
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    }
}

#[test]
fn test_predicate_ir_all() {
    let predicate = PredicateIR::All;
    let table_info = make_table_info(vec![("id", "INTEGER"), ("name", "TEXT")]);
    let row = vec![Value::Integer(1), Value::Text("test".to_string())];

    // PredicateIR::All should always return true
    assert!(predicate.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_expr_literal_boolean() {
    let table_info = make_table_info(vec![]);
    let empty_row: Vec<Value> = vec![];

    // Boolean literal true
    let pred = PredicateIR::Expr(ExprIR::Literal(Value::Boolean(true)));
    // Literal(Value::Boolean(b)) returns *b directly
    assert!(pred.evaluate(&empty_row, &table_info));

    // Boolean literal false
    let pred = PredicateIR::Expr(ExprIR::Literal(Value::Boolean(false)));
    assert!(!pred.evaluate(&empty_row, &table_info));

    // Non-boolean literal (should return true per eval_expr)
    let pred = PredicateIR::Expr(ExprIR::Literal(Value::Integer(42)));
    assert!(pred.evaluate(&empty_row, &table_info));
}

#[test]
fn test_predicate_ir_expr_column() {
    let table_info = make_table_info(vec![("active", "BOOLEAN"), ("id", "INTEGER")]);
    let row_true = vec![Value::Boolean(true), Value::Integer(1)];
    let row_false = vec![Value::Boolean(false), Value::Integer(1)];
    let row_null = vec![Value::Null, Value::Integer(1)];

    let pred = PredicateIR::Expr(ExprIR::Column("active".to_string()));

    assert!(pred.evaluate(&row_true, &table_info));
    assert!(!pred.evaluate(&row_false, &table_info));
    assert!(!pred.evaluate(&row_null, &table_info)); // Null is not Boolean(true)
}

#[test]
fn test_predicate_ir_expr_column_not_found() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row = vec![Value::Integer(1)];

    let pred = PredicateIR::Expr(ExprIR::Column("nonexistent".to_string()));
    assert!(!pred.evaluate(&row, &table_info)); // Column not found returns false
}

#[test]
fn test_predicate_ir_is_null() {
    let table_info = make_table_info(vec![("value", "INTEGER")]);

    let row_null = vec![Value::Null];
    let row_not_null = vec![Value::Integer(42)];

    let pred = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column(
        "value".to_string(),
    ))));

    assert!(pred.evaluate(&row_null, &table_info));
    assert!(!pred.evaluate(&row_not_null, &table_info));
}

#[test]
fn test_predicate_ir_is_not_null() {
    let table_info = make_table_info(vec![("value", "INTEGER")]);

    let row_null = vec![Value::Null];
    let row_not_null = vec![Value::Integer(42)];

    let pred = PredicateIR::Expr(ExprIR::IsNotNull(Box::new(ExprIR::Column(
        "value".to_string(),
    ))));

    assert!(!pred.evaluate(&row_null, &table_info));
    assert!(pred.evaluate(&row_not_null, &table_info));
}

#[test]
fn test_predicate_ir_unary_not() {
    let table_info = make_table_info(vec![("flag", "BOOLEAN")]);

    let row_true = vec![Value::Boolean(true)];
    let row_false = vec![Value::Boolean(false)];

    let pred = PredicateIR::Expr(ExprIR::Unary {
        op: "NOT".to_string(),
        expr: Box::new(ExprIR::Column("flag".to_string())),
    });

    assert!(!pred.evaluate(&row_true, &table_info));
    assert!(pred.evaluate(&row_false, &table_info));
}

#[test]
fn test_predicate_ir_unary_not_case_insensitive() {
    let table_info = make_table_info(vec![("flag", "BOOLEAN")]);
    let row = vec![Value::Boolean(true)];

    // lowercase 'not'
    let pred = PredicateIR::Expr(ExprIR::Unary {
        op: "not".to_string(),
        expr: Box::new(ExprIR::Literal(Value::Boolean(true))),
    });
    assert!(!pred.evaluate(&row, &table_info));

    // uppercase 'NOT'
    let pred = PredicateIR::Expr(ExprIR::Unary {
        op: "NOT".to_string(),
        expr: Box::new(ExprIR::Literal(Value::Boolean(true))),
    });
    assert!(!pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_unary_unknown_op() {
    let table_info = make_table_info(vec![]);
    let row = vec![];

    // Unknown unary operator should return false
    let pred = PredicateIR::Expr(ExprIR::Unary {
        op: "UNKNOWN".to_string(),
        expr: Box::new(ExprIR::Literal(Value::Boolean(true))),
    });
    assert!(!pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_and() {
    let table_info = make_table_info(vec![("a", "BOOLEAN"), ("b", "BOOLEAN")]);
    let row = vec![Value::Boolean(true), Value::Boolean(true)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "AND".to_string(),
        left: Box::new(ExprIR::Column("a".to_string())),
        right: Box::new(ExprIR::Column("b".to_string())),
    });

    assert!(pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_and_false() {
    let table_info = make_table_info(vec![("a", "BOOLEAN"), ("b", "BOOLEAN")]);
    let row = vec![Value::Boolean(true), Value::Boolean(false)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "AND".to_string(),
        left: Box::new(ExprIR::Column("a".to_string())),
        right: Box::new(ExprIR::Column("b".to_string())),
    });

    assert!(!pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_or() {
    let table_info = make_table_info(vec![("a", "BOOLEAN"), ("b", "BOOLEAN")]);
    let row = vec![Value::Boolean(false), Value::Boolean(true)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "OR".to_string(),
        left: Box::new(ExprIR::Column("a".to_string())),
        right: Box::new(ExprIR::Column("b".to_string())),
    });

    assert!(pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_or_both_false() {
    let table_info = make_table_info(vec![("a", "BOOLEAN"), ("b", "BOOLEAN")]);
    let row = vec![Value::Boolean(false), Value::Boolean(false)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "OR".to_string(),
        left: Box::new(ExprIR::Column("a".to_string())),
        right: Box::new(ExprIR::Column("b".to_string())),
    });

    assert!(!pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_eq_integer() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row_match = vec![Value::Integer(42)];
    let row_no_match = vec![Value::Integer(99)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(42))),
    });

    assert!(pred.evaluate(&row_match, &table_info));
    assert!(!pred.evaluate(&row_no_match, &table_info));
}

#[test]
fn test_predicate_ir_binary_eq_text() {
    let table_info = make_table_info(vec![("name", "TEXT")]);
    let row_match = vec![Value::Text("hello".to_string())];
    let row_no_match = vec![Value::Text("world".to_string())];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("name".to_string())),
        right: Box::new(ExprIR::Literal(Value::Text("hello".to_string()))),
    });

    assert!(pred.evaluate(&row_match, &table_info));
    assert!(!pred.evaluate(&row_no_match, &table_info));
}

#[test]
fn test_predicate_ir_binary_eq_double_eq() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row = vec![Value::Integer(42)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "==".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(42))),
    });

    assert!(pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_ne() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row = vec![Value::Integer(42)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "!=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(99))),
    });

    assert!(pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_ne_angle() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row = vec![Value::Integer(42)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "<>".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(99))),
    });

    assert!(pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_gt() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row = vec![Value::Integer(10)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: ">".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(5))),
    });

    assert!(pred.evaluate(&row, &table_info));
    assert!(!pred.evaluate(&vec![Value::Integer(5)], &table_info));
    assert!(!pred.evaluate(&vec![Value::Integer(3)], &table_info));
}

#[test]
fn test_predicate_ir_binary_gte() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: ">=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(5))),
    });

    assert!(pred.evaluate(&vec![Value::Integer(10)], &table_info));
    assert!(pred.evaluate(&vec![Value::Integer(5)], &table_info));
    assert!(!pred.evaluate(&vec![Value::Integer(3)], &table_info));
}

#[test]
fn test_predicate_ir_binary_lt() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row = vec![Value::Integer(3)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "<".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(5))),
    });

    assert!(pred.evaluate(&row, &table_info));
    assert!(!pred.evaluate(&vec![Value::Integer(5)], &table_info));
    assert!(!pred.evaluate(&vec![Value::Integer(10)], &table_info));
}

#[test]
fn test_predicate_ir_binary_lte() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "<=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(5))),
    });

    assert!(pred.evaluate(&vec![Value::Integer(3)], &table_info));
    assert!(pred.evaluate(&vec![Value::Integer(5)], &table_info));
    assert!(!pred.evaluate(&vec![Value::Integer(10)], &table_info));
}

#[test]
fn test_predicate_ir_binary_unknown_op() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row = vec![Value::Integer(42)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "UNKNOWN".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(42))),
    });

    assert!(!pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_binary_null_handling() {
    let table_info = make_table_info(vec![("id", "INTEGER")]);
    let row_null = vec![Value::Null];

    // Any comparison with Null should return false
    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "=".to_string(),
        left: Box::new(ExprIR::Column("id".to_string())),
        right: Box::new(ExprIR::Literal(Value::Integer(42))),
    });

    assert!(!pred.evaluate(&row_null, &table_info));
}

#[test]
fn test_predicate_ir_binary_float_comparison() {
    let table_info = make_table_info(vec![("price", "FLOAT")]);

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: ">".to_string(),
        left: Box::new(ExprIR::Column("price".to_string())),
        right: Box::new(ExprIR::Literal(Value::Float(3.14))),
    });

    assert!(pred.evaluate(&vec![Value::Float(3.15)], &table_info));
    assert!(pred.evaluate(&vec![Value::Float(10.0)], &table_info));
    assert!(!pred.evaluate(&vec![Value::Float(3.14)], &table_info));
    assert!(!pred.evaluate(&vec![Value::Float(1.0)], &table_info));
}

#[test]
fn test_predicate_ir_binary_text_comparison() {
    let table_info = make_table_info(vec![("name", "TEXT")]);

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: ">".to_string(),
        left: Box::new(ExprIR::Column("name".to_string())),
        right: Box::new(ExprIR::Literal(Value::Text("apple".to_string()))),
    });

    assert!(pred.evaluate(&vec![Value::Text("banana".to_string())], &table_info));
    assert!(!pred.evaluate(&vec![Value::Text("apple".to_string())], &table_info));
    assert!(!pred.evaluate(&vec![Value::Text("aaa".to_string())], &table_info));
}

#[test]
fn test_predicate_ir_eval_to_value_column() {
    let table_info = make_table_info(vec![
        ("id", "INTEGER"),
        ("name", "TEXT"),
        ("active", "BOOLEAN"),
    ]);
    let row = vec![
        Value::Integer(42),
        Value::Text("test".to_string()),
        Value::Boolean(true),
    ];

    // Column lookup: Column("active") returns true if row[active] == Boolean(true)
    let pred = PredicateIR::Expr(ExprIR::Column("active".to_string()));
    assert!(pred.evaluate(&row, &table_info)); // row["active"] is Boolean(true)

    // Column with Integer value returns false (not Boolean(true))
    let pred = PredicateIR::Expr(ExprIR::Column("id".to_string()));
    assert!(!pred.evaluate(&row, &table_info)); // row["id"] is Integer(42), not Boolean(true)
}

#[test]
fn test_predicate_ir_eval_to_value_is_null() {
    let table_info = make_table_info(vec![("value", "INTEGER")]);

    // IsNull as expression (not predicate)
    let pred = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column(
        "value".to_string(),
    ))));
    assert!(pred.evaluate(&vec![Value::Null], &table_info));
    assert!(!pred.evaluate(&vec![Value::Integer(42)], &table_info));
}

#[test]
fn test_predicate_ir_eval_to_value_is_not_null() {
    let table_info = make_table_info(vec![("value", "INTEGER")]);

    let pred = PredicateIR::Expr(ExprIR::IsNotNull(Box::new(ExprIR::Column(
        "value".to_string(),
    ))));
    assert!(!pred.evaluate(&vec![Value::Null], &table_info));
    assert!(pred.evaluate(&vec![Value::Integer(42)], &table_info));
}

#[test]
fn test_predicate_ir_eval_to_value_unary_not() {
    let table_info = make_table_info(vec![("flag", "BOOLEAN")]);

    // NOT on boolean column
    let pred = PredicateIR::Expr(ExprIR::Unary {
        op: "NOT".to_string(),
        expr: Box::new(ExprIR::Column("flag".to_string())),
    });
    assert!(!pred.evaluate(&vec![Value::Boolean(true)], &table_info));
    assert!(pred.evaluate(&vec![Value::Boolean(false)], &table_info));
}

#[test]
fn test_predicate_ir_eval_to_value_binary_and() {
    let table_info = make_table_info(vec![("a", "BOOLEAN"), ("b", "BOOLEAN")]);
    let row = vec![Value::Boolean(true), Value::Boolean(true)];

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "AND".to_string(),
        left: Box::new(ExprIR::Column("a".to_string())),
        right: Box::new(ExprIR::Column("b".to_string())),
    });
    assert!(pred.evaluate(&row, &table_info));
}

#[test]
fn test_predicate_ir_eval_to_value_binary_or() {
    let table_info = make_table_info(vec![]);

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "OR".to_string(),
        left: Box::new(ExprIR::Literal(Value::Boolean(false))),
        right: Box::new(ExprIR::Literal(Value::Boolean(true))),
    });
    assert!(pred.evaluate(&[], &table_info));

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "OR".to_string(),
        left: Box::new(ExprIR::Literal(Value::Boolean(false))),
        right: Box::new(ExprIR::Literal(Value::Boolean(false))),
    });
    assert!(!pred.evaluate(&[], &table_info));
}

#[test]
fn test_predicate_ir_cmp_values_integer() {
    let table_info = make_table_info(vec![]);

    // Test cmp_values via binary comparison
    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: ">".to_string(),
        left: Box::new(ExprIR::Literal(Value::Integer(10))),
        right: Box::new(ExprIR::Literal(Value::Integer(5))),
    });
    assert!(pred.evaluate(&[], &table_info));
}

#[test]
fn test_predicate_ir_cmp_values_float() {
    let table_info = make_table_info(vec![]);

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: ">".to_string(),
        left: Box::new(ExprIR::Literal(Value::Float(3.15))),
        right: Box::new(ExprIR::Literal(Value::Float(3.14))),
    });
    assert!(pred.evaluate(&[], &table_info));
}

#[test]
fn test_predicate_ir_cmp_values_text() {
    let table_info = make_table_info(vec![]);

    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: ">".to_string(),
        left: Box::new(ExprIR::Literal(Value::Text("zebra".to_string()))),
        right: Box::new(ExprIR::Literal(Value::Text("apple".to_string()))),
    });
    assert!(pred.evaluate(&[], &table_info));
}

// =============================================================================
// UpdatePlan Tests
// =============================================================================

#[test]
fn test_update_plan_new() {
    let assignments = vec![AssignmentIR {
        column: "id".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(100)),
    }];
    let mutation = MutationIR::new(assignments);
    let predicate = PredicateIR::All;

    let plan = UpdatePlan::new(
        "users".to_string(),
        predicate,
        mutation,
        5, // rows_affected
    );

    assert_eq!(plan.table, "users");
    assert!(matches!(plan.predicate, PredicateIR::All));
    assert_eq!(plan.trace.rows_affected(), 5);
}

#[test]
fn test_update_plan_accessors() {
    let assignments = vec![AssignmentIR {
        column: "value".to_string(),
        column_index: 1,
        expr: ExprIR::Literal(Value::Text("new".to_string())),
    }];
    let mutation = MutationIR::new(assignments.clone());
    let predicate = PredicateIR::Expr(ExprIR::Column("active".to_string()));

    let plan = UpdatePlan::new("items".to_string(), predicate.clone(), mutation.clone(), 10);

    // Test predicate accessor
    let pred = plan.predicate();
    assert!(matches!(pred, PredicateIR::Expr(_)));

    // Test mutation accessor
    let m = plan.mutation();
    assert_eq!(m.assignments().len(), 1);

    // Test trace accessor
    let trace = plan.trace();
    assert_eq!(trace.rows_affected(), 10);
    assert!(!trace.plan_id.is_empty());
}

#[test]
fn test_plan_trace_new() {
    let trace = PlanTrace::new(100, 200, 50);

    assert_eq!(trace.predicate_hash, 100);
    assert_eq!(trace.mutation_hash, 200);
    assert_eq!(trace.rows_affected, 50);
    assert!(!trace.plan_id.is_empty());
    assert!(trace.plan_id.starts_with("update_"));

    // Combined hash should be based on predicate and mutation hashes
    let mut expected_hasher = DefaultHasher::new();
    100u64.hash(&mut expected_hasher);
    200u64.hash(&mut expected_hasher);
    let expected_combined = expected_hasher.finish();

    assert_eq!(trace.combined_hash, expected_combined);
}

#[test]
fn test_plan_trace_rows_affected() {
    let trace = PlanTrace::new(0, 0, 999);
    assert_eq!(trace.rows_affected(), 999);
}

// =============================================================================
// MutationIR clone and debug
// =============================================================================

#[test]
fn test_mutation_ir_clone() {
    let assignments = vec![AssignmentIR {
        column: "x".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(1)),
    }];
    let mutation = MutationIR::new(assignments);
    let cloned = mutation.clone();

    assert_eq!(mutation.mutation_hash(), cloned.mutation_hash());
    assert_eq!(mutation.assignments().len(), cloned.assignments().len());
}

#[test]
fn test_mutation_ir_debug() {
    let assignments = vec![AssignmentIR {
        column: "x".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(1)),
    }];
    let mutation = MutationIR::new(assignments);

    let debug_str = format!("{:?}", mutation);
    assert!(debug_str.contains("MutationIR"));
}

#[test]
fn test_predicate_ir_debug() {
    let predicate = PredicateIR::All;
    let debug_str = format!("{:?}", predicate);
    assert!(debug_str.contains("All"));
}

#[test]
fn test_expr_ir_debug() {
    let expr = ExprIR::Literal(Value::Integer(42));
    let debug_str = format!("{:?}", expr);
    assert!(debug_str.contains("Literal"));
}

#[test]
fn test_update_plan_debug() {
    let assignments = vec![AssignmentIR {
        column: "x".to_string(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(1)),
    }];
    let mutation = MutationIR::new(assignments);
    let plan = UpdatePlan::new("t".to_string(), PredicateIR::All, mutation, 0);

    let debug_str = format!("{:?}", plan);
    assert!(debug_str.contains("UpdatePlan"));
}
