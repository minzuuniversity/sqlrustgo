use sqlrustgo_executor::mutation_compiler::{Assignment, RowMutation};
use sqlrustgo_planner::Expr;
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
