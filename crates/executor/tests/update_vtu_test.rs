use sqlrustgo_executor::mutation_compiler::{
    canonicalize_expr, Assignment, CanonicalExpr, MutationCompiler, RowMutation,
};
use sqlrustgo_planner::{Column, Expr, Operator};
use sqlrustgo_types::Value;

#[test]
fn test_row_mutation_creation() {
    let assignments = vec![Assignment {
        column: "age".to_string(),
        expr: Expr::Literal(Value::Integer(25)),
    }];
    let mutation = RowMutation::new(assignments, 0xABCDEF);

    assert_eq!(mutation.mutation_hash(), 0xABCDEF);
    assert_eq!(mutation.assignments().len(), 1);
}

#[test]
fn test_canonical_expr_hash_stability() {
    let expr1 = Expr::BinaryExpr {
        left: Box::new(Expr::Column(Column {
            name: "age".to_string(),
            relation: None,
        })),
        op: Operator::Plus,
        right: Box::new(Expr::Literal(Value::Integer(1))),
    };

    let canonical = canonicalize_expr(&expr1);
    assert!(matches!(canonical, CanonicalExpr::Compound { op, .. } if op == "+"));
}

#[test]
fn test_commutative_args_sorted() {
    let expr_a_plus_b = Expr::BinaryExpr {
        left: Box::new(Expr::Column(Column {
            name: "a".to_string(),
            relation: None,
        })),
        op: Operator::Plus,
        right: Box::new(Expr::Column(Column {
            name: "b".to_string(),
            relation: None,
        })),
    };

    let expr_b_plus_a = Expr::BinaryExpr {
        left: Box::new(Expr::Column(Column {
            name: "b".to_string(),
            relation: None,
        })),
        op: Operator::Plus,
        right: Box::new(Expr::Column(Column {
            name: "a".to_string(),
            relation: None,
        })),
    };

    let canonical_ab = canonicalize_expr(&expr_a_plus_b);
    let canonical_ba = canonicalize_expr(&expr_b_plus_a);
    assert_eq!(canonical_ab, canonical_ba, "a + b and b + a should canonicalize to same form");
}

#[test]
fn test_mutation_compiler_basic() {
    let assignments = vec![
        Assignment {
            column: "age".to_string(),
            expr: Expr::Literal(Value::Integer(25)),
        },
    ];

    let mutation = MutationCompiler::compile(assignments);
    assert!(mutation.mutation_hash() != 0); // Hash should be computed
}
