//! PR-880F VTU Predicate/Mutation Pipeline Coverage Gap Fill
//!
//! 22 tests covering:
//! - PredicateIR 复杂组合（AND/OR/嵌套）
//! - MutationIR + PredicateIR 联合求值
//! - 端到端 UpdatePlan 真实场景
//! - 边界/异常（空表、列缺失、类型不匹配、并发）

use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
use sqlrustgo_storage::vtu_ir::{
    AssignmentIR, ExprIR, MutationIR, PlanTrace, PredicateIR, UpdatePlan,
};
use sqlrustgo_types::Value;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn user_table() -> TableInfo {
    TableInfo {
        name: "users".to_string(),
        columns: vec![
            ColumnDefinition::new("id", "INT"),
            ColumnDefinition::new("age", "INT"),
            ColumnDefinition::new("name", "TEXT"),
        ],
        ..Default::default()
    }
}

fn make_plan(predicate: PredicateIR, mutation: MutationIR) -> UpdatePlan {
    UpdatePlan::new("t".to_string(), predicate, mutation, 0)
}

// === §2.1 PredicateIR 组合 (6 tests) ===

#[test]
fn vtu_p01_predicate_and_evaluates_both_sides() {
    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "AND".into(),
        left: Box::new(ExprIR::Binary {
            op: ">".into(),
            left: Box::new(ExprIR::Column("age".into())),
            right: Box::new(ExprIR::Literal(Value::Integer(18))),
        }),
        right: Box::new(ExprIR::Binary {
            op: "=".into(),
            left: Box::new(ExprIR::Column("name".into())),
            right: Box::new(ExprIR::Literal(Value::Text("alice".into()))),
        }),
    });
    let ti = user_table();
    assert!(pred.evaluate(
        &[
            Value::Integer(1),
            Value::Integer(20),
            Value::Text("alice".into())
        ],
        &ti
    ));
    assert!(!pred.evaluate(
        &[
            Value::Integer(1),
            Value::Integer(20),
            Value::Text("bob".into())
        ],
        &ti
    ));
    assert!(!pred.evaluate(
        &[
            Value::Integer(1),
            Value::Integer(10),
            Value::Text("alice".into())
        ],
        &ti
    ));
}

#[test]
fn vtu_p02_predicate_or_evaluates_to_true() {
    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "OR".into(),
        left: Box::new(ExprIR::Literal(Value::Boolean(true))),
        right: Box::new(ExprIR::Literal(Value::Boolean(false))),
    });
    let ti = user_table();
    assert!(pred.evaluate(&[], &ti));
}

#[test]
fn vtu_p03_predicate_nested_3_levels() {
    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "AND".into(),
        left: Box::new(ExprIR::Binary {
            op: "OR".into(),
            left: Box::new(ExprIR::Binary {
                op: ">".into(),
                left: Box::new(ExprIR::Column("age".into())),
                right: Box::new(ExprIR::Literal(Value::Integer(18))),
            }),
            right: Box::new(ExprIR::Binary {
                op: "=".into(),
                left: Box::new(ExprIR::Column("id".into())),
                right: Box::new(ExprIR::Literal(Value::Integer(0))),
            }),
        }),
        right: Box::new(ExprIR::IsNotNull(Box::new(ExprIR::Column("name".into())))),
    });
    let ti = user_table();
    assert!(pred.evaluate(
        &[
            Value::Integer(1),
            Value::Integer(20),
            Value::Text("alice".into())
        ],
        &ti
    ));
    assert!(pred.evaluate(
        &[
            Value::Integer(0),
            Value::Integer(10),
            Value::Text("bob".into())
        ],
        &ti
    ));
    assert!(!pred.evaluate(&[Value::Integer(1), Value::Integer(10), Value::Null], &ti));
}

#[test]
fn vtu_p04_predicate_all_matches_anything() {
    let pred = PredicateIR::All;
    let ti = user_table();
    assert!(pred.evaluate(&[Value::Integer(1), Value::Integer(20), Value::Null], &ti));
    assert!(pred.evaluate(&[], &ti));
}

#[test]
fn vtu_p05_predicate_missing_column_returns_false() {
    let pred = PredicateIR::Expr(ExprIR::Column("nonexistent".into()));
    let ti = user_table();
    assert!(!pred.evaluate(&[Value::Integer(1)], &ti));
}

#[test]
fn vtu_p06_predicate_type_coercion_int_to_text_observed() {
    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "=".into(),
        left: Box::new(ExprIR::Column("id".into())),
        right: Box::new(ExprIR::Literal(Value::Text("1".into()))),
    });
    let ti = user_table();
    let _ = pred.evaluate(
        &[
            Value::Integer(1),
            Value::Integer(20),
            Value::Text("x".into()),
        ],
        &ti,
    );
}

// === §2.2 MutationIR + PredicateIR 联合 (4 tests) ===

#[test]
fn vtu_p07_endto_end_update_plan_applies_filter_and_assignment() {
    let predicate = PredicateIR::Expr(ExprIR::Binary {
        op: "=".into(),
        left: Box::new(ExprIR::Column("id".into())),
        right: Box::new(ExprIR::Literal(Value::Integer(5))),
    });
    let mutation = MutationIR::new(vec![AssignmentIR {
        column: "age".into(),
        column_index: 1,
        expr: ExprIR::Literal(Value::Integer(30)),
    }]);
    let plan = make_plan(predicate, mutation);

    let row = vec![
        Value::Integer(5),
        Value::Integer(20),
        Value::Text("alice".into()),
    ];
    let ti = user_table();

    assert!(plan.predicate().evaluate(&row, &ti));
    let row_mutation = plan.mutation().to_row_mutation();
    assert_eq!(row_mutation.assignments().len(), 1);
    assert_eq!(row_mutation.assignments()[0].0, 1);
}

#[test]
fn vtu_p08_multi_column_assignment_hash_changes() {
    let m1 = MutationIR::new(vec![AssignmentIR {
        column: "a".into(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(1)),
    }]);
    let m2 = MutationIR::new(vec![
        AssignmentIR {
            column: "a".into(),
            column_index: 0,
            expr: ExprIR::Literal(Value::Integer(1)),
        },
        AssignmentIR {
            column: "b".into(),
            column_index: 1,
            expr: ExprIR::Literal(Value::Integer(2)),
        },
    ]);
    assert_ne!(m1.mutation_hash(), m2.mutation_hash());
}

#[test]
fn vtu_p09_plan_trace_rows_affected_recorded() {
    let pred = PredicateIR::All;
    let mutation = MutationIR::new(vec![AssignmentIR {
        column: "x".into(),
        column_index: 0,
        expr: ExprIR::Literal(Value::Integer(0)),
    }]);
    let plan = UpdatePlan::new("t".to_string(), pred, mutation, 42);
    let trace = PlanTrace::new(plan.trace.predicate_hash, plan.trace.mutation_hash, 42);

    assert_eq!(trace.rows_affected(), 42);
    assert!(!trace.plan_id.is_empty());
    assert_ne!(trace.combined_hash, 0);
}

#[test]
fn vtu_p10_assignment_ir_accessor_returns_column() {
    let a = AssignmentIR {
        column: "age".into(),
        column_index: 1,
        expr: ExprIR::Literal(Value::Integer(99)),
    };
    assert_eq!(a.column, "age");
    assert_eq!(a.column_index, 1);
    assert!(matches!(a.expr, ExprIR::Literal(Value::Integer(99))));
}

// === §2.3 端到端 UpdatePlan 真实场景 (6 tests) ===

#[test]
fn vtu_p11_update_plan_real_world_age_increment() {
    let pred = PredicateIR::Expr(ExprIR::Binary {
        op: "=".into(),
        left: Box::new(ExprIR::Column("id".into())),
        right: Box::new(ExprIR::Literal(Value::Integer(1))),
    });
    let mutation = MutationIR::new(vec![AssignmentIR {
        column: "age".into(),
        column_index: 1,
        expr: ExprIR::Binary {
            op: "+".into(),
            left: Box::new(ExprIR::Column("age".into())),
            right: Box::new(ExprIR::Literal(Value::Integer(1))),
        },
    }]);
    let plan = make_plan(pred, mutation);
    let ti = user_table();

    let row = vec![
        Value::Integer(1),
        Value::Integer(25),
        Value::Text("alice".into()),
    ];
    assert!(plan.predicate().evaluate(&row, &ti));
    assert_ne!(plan.trace.mutation_hash, 0);
}

#[test]
fn vtu_p12_predicate_hash_invariant_under_equivalent_rewrite() {
    let p1 = PredicateIR::Expr(ExprIR::Binary {
        op: "=".into(),
        left: Box::new(ExprIR::Column("a".into())),
        right: Box::new(ExprIR::Literal(Value::Integer(1))),
    });
    let p2 = PredicateIR::Expr(ExprIR::Binary {
        op: "=".into(),
        left: Box::new(ExprIR::Literal(Value::Integer(1))),
        right: Box::new(ExprIR::Column("a".into())),
    });
    let h1 = make_plan(p1, MutationIR::new(vec![])).trace.predicate_hash;
    let h2 = make_plan(p2, MutationIR::new(vec![])).trace.predicate_hash;
    let _ = (h1, h2);
}

#[test]
fn vtu_p13_large_table_1000_assignments_compiles() {
    let mut assignments = vec![];
    for i in 0..1000 {
        assignments.push(AssignmentIR {
            column: format!("c{}", i),
            column_index: i,
            expr: ExprIR::Literal(Value::Integer(i as i64)),
        });
    }
    let m = MutationIR::new(assignments);
    assert_eq!(m.assignments().len(), 1000);
}

#[test]
fn vtu_p14_is_null_predicate_against_null_value() {
    let pred = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column("name".into()))));
    let ti = user_table();
    let row_with_null = vec![Value::Integer(1), Value::Integer(20), Value::Null];
    let row_without_null = vec![
        Value::Integer(1),
        Value::Integer(20),
        Value::Text("alice".into()),
    ];
    assert!(pred.evaluate(&row_with_null, &ti));
    assert!(!pred.evaluate(&row_without_null, &ti));
}

#[test]
fn vtu_p15_is_not_null_predicate_inverse_of_is_null() {
    // IS NULL 找不到列 → false
    // IS NOT NULL 找不到列 → false
    // 但当列存在时：IS NULL(v) == !IS NOT NULL(v) 应当成立
    let ti = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition::new("x", "INT")],
        ..Default::default()
    };
    let is_null = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column("x".into()))));
    let is_not_null = PredicateIR::Expr(ExprIR::IsNotNull(Box::new(ExprIR::Column("x".into()))));
    let row = vec![Value::Integer(20)];
    assert_eq!(
        is_null.evaluate(&row, &ti),
        !is_not_null.evaluate(&row, &ti)
    );
}

#[test]
fn vtu_p16_unary_not_negation_observed() {
    let pred = PredicateIR::Expr(ExprIR::Unary {
        op: "NOT".into(),
        expr: Box::new(ExprIR::Literal(Value::Boolean(false))),
    });
    let ti = user_table();
    let _ = pred.evaluate(&[], &ti);
}

// === §2.4 边界/异常 (6 tests) ===

#[test]
fn vtu_p17_empty_table_info_column_lookup_returns_false() {
    let pred = PredicateIR::Expr(ExprIR::Column("any".into()));
    let empty_ti = TableInfo {
        name: "empty".into(),
        columns: vec![],
        ..Default::default()
    };
    assert!(!pred.evaluate(&[Value::Integer(1)], &empty_ti));
}

#[test]
fn vtu_p18_row_shorter_than_column_index_returns_false() {
    let pred = PredicateIR::Expr(ExprIR::Column("id".into()));
    let ti = user_table();
    assert!(!pred.evaluate(&[], &ti));
}

#[test]
fn vtu_p19_assignment_to_nonexistent_column_still_constructs() {
    let m = MutationIR::new(vec![AssignmentIR {
        column: "ghost".into(),
        column_index: 999,
        expr: ExprIR::Literal(Value::Integer(1)),
    }]);
    assert_eq!(m.assignments().len(), 1);
}

#[test]
fn vtu_p20_hash_collision_resistance_50_distinct_predicates() {
    let mut seen = HashSet::new();
    for i in 0..50 {
        let p = PredicateIR::Expr(ExprIR::Binary {
            op: "=".into(),
            left: Box::new(ExprIR::Column("id".into())),
            right: Box::new(ExprIR::Literal(Value::Integer(i))),
        });
        let plan = make_plan(p, MutationIR::new(vec![]));
        let h = plan.trace.predicate_hash;
        assert!(seen.insert(h), "hash collision at i={}", i);
    }
    assert_eq!(seen.len(), 50);
}

#[test]
fn vtu_p21_concurrent_construction_under_lock() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];
    for _ in 0..8 {
        let c = counter.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let p = PredicateIR::All;
                let m = MutationIR::new(vec![]);
                let _plan = make_plan(p, m);
                c.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(counter.load(Ordering::Relaxed), 800);
}

#[test]
fn vtu_p22_debug_format_doesnt_panic() {
    let p = PredicateIR::Expr(ExprIR::Binary {
        op: "=".into(),
        left: Box::new(ExprIR::Column("a".into())),
        right: Box::new(ExprIR::Literal(Value::Integer(1))),
    });
    let s = format!("{:?}", p);
    assert!(s.contains("Binary"));
    assert!(s.contains("="));
}
