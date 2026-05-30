use sqlrustgo_executor::mutation_compiler::{
    canonicalize_expr, Assignment, CanonicalExpr, RowMutation,
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
