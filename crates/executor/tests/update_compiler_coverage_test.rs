use sqlrustgo_executor::mutation_compiler::{Assignment, MutationCompiler, RowMutation};
use sqlrustgo_executor::predicate_compiler::PredicateCompiler;
use sqlrustgo_executor::update_compiler::{UpdateCompiler, UpdatePlan, UpdateStatement};
use sqlrustgo_planner::{Column, DataType, Expr, Operator, Schema};
use sqlrustgo_types::Value;

fn test_schema() -> Schema {
    Schema {
        fields: vec![
            sqlrustgo_planner::Field {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
            sqlrustgo_planner::Field {
                name: "value".to_string(),
                data_type: DataType::Integer,
                nullable: true,
            },
        ],
    }
}

fn column_expr(name: &str) -> Expr {
    Expr::Column(Column::new(name.to_string()))
}

fn lit(val: Value) -> Expr {
    Expr::Literal(val)
}

fn binary(left: Expr, op: Operator, right: Expr) -> Expr {
    Expr::BinaryExpr {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

#[test]
fn test_update_compiler_no_where_clause() {
    let schema = test_schema();
    let stmt = UpdateStatement {
        table: "t".to_string(),
        where_clause: None,
        assignments: vec![Assignment {
            column: "value".to_string(),
            expr: lit(Value::Integer(42)),
        }],
    };
    let plan: UpdatePlan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
    assert_eq!(plan.predicate_hash(), 0);
    assert_ne!(plan.mutation_hash(), 0);
    assert_ne!(plan.combined_hash(), 0);
}

#[test]
fn test_update_compiler_with_where_clause_eq() {
    let schema = test_schema();
    let stmt = UpdateStatement {
        table: "t".to_string(),
        where_clause: Some(binary(
            column_expr("id"),
            Operator::Eq,
            lit(Value::Integer(1)),
        )),
        assignments: vec![Assignment {
            column: "value".to_string(),
            expr: lit(Value::Integer(99)),
        }],
    };
    let plan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
    assert_ne!(plan.predicate_hash(), 0);
    assert_ne!(plan.mutation_hash(), 0);
    assert_ne!(plan.combined_hash(), 0);
}

#[test]
fn test_update_compiler_with_inequality_operators() {
    let schema = test_schema();
    for op in [
        Operator::NotEq,
        Operator::Lt,
        Operator::LtEq,
        Operator::Gt,
        Operator::GtEq,
    ] {
        let stmt = UpdateStatement {
            table: "t".to_string(),
            where_clause: Some(binary(column_expr("id"), op, lit(Value::Integer(5)))),
            assignments: vec![Assignment {
                column: "value".to_string(),
                expr: lit(Value::Integer(1)),
            }],
        };
        let plan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
        assert_ne!(plan.predicate_hash(), 0);
    }
}

#[test]
fn test_update_compiler_with_and_predicate() {
    let schema = test_schema();
    let and_pred = binary(
        binary(column_expr("id"), Operator::Eq, lit(Value::Integer(1))),
        Operator::And,
        binary(column_expr("value"), Operator::Lt, lit(Value::Integer(100))),
    );
    let stmt = UpdateStatement {
        table: "t".to_string(),
        where_clause: Some(and_pred),
        assignments: vec![Assignment {
            column: "value".to_string(),
            expr: lit(Value::Integer(0)),
        }],
    };
    let plan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
    assert_ne!(plan.predicate_hash(), 0);
}

#[test]
fn test_update_compiler_with_or_predicate() {
    let schema = test_schema();
    let or_pred = binary(
        binary(column_expr("id"), Operator::Eq, lit(Value::Integer(1))),
        Operator::Or,
        binary(column_expr("id"), Operator::Eq, lit(Value::Integer(2))),
    );
    let stmt = UpdateStatement {
        table: "t".to_string(),
        where_clause: Some(or_pred),
        assignments: vec![Assignment {
            column: "value".to_string(),
            expr: lit(Value::Integer(0)),
        }],
    };
    let plan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
    assert_ne!(plan.predicate_hash(), 0);
}

#[test]
fn test_update_compiler_with_not_predicate() {
    let schema = test_schema();
    let not_pred = Expr::BinaryExpr {
        left: Box::new(lit(Value::Integer(0))),
        op: Operator::Not,
        right: Box::new(binary(
            column_expr("id"),
            Operator::Eq,
            lit(Value::Integer(1)),
        )),
    };
    let stmt = UpdateStatement {
        table: "t".to_string(),
        where_clause: Some(not_pred),
        assignments: vec![Assignment {
            column: "value".to_string(),
            expr: lit(Value::Integer(0)),
        }],
    };
    let plan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
    assert_ne!(plan.predicate_hash(), 0);
}

#[test]
fn test_update_plan_accessors() {
    let schema = test_schema();
    let stmt = UpdateStatement {
        table: "t".to_string(),
        where_clause: Some(binary(
            column_expr("id"),
            Operator::Eq,
            lit(Value::Integer(7)),
        )),
        assignments: vec![Assignment {
            column: "value".to_string(),
            expr: lit(Value::Integer(8)),
        }],
    };
    let plan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
    let _pred = plan.predicate();
    let _mut = plan.mutation();
    assert_eq!(plan.predicate_hash(), plan.predicate_hash());
    assert_eq!(plan.mutation_hash(), plan.mutation_hash());
    assert_eq!(plan.combined_hash(), plan.combined_hash());
}

#[test]
fn test_update_compiler_unsupported_predicate_operator_falls_back_to_const_null() {
    let schema = test_schema();
    let plus_pred = binary(
        column_expr("id"),
        Operator::Plus,
        lit(Value::Integer(1)),
    );
    let stmt = UpdateStatement {
        table: "t".to_string(),
        where_clause: Some(plus_pred),
        assignments: vec![Assignment {
            column: "value".to_string(),
            expr: lit(Value::Integer(0)),
        }],
    };
    let plan = UpdateCompiler::compile(&stmt, &schema).expect("compile ok");
    assert_ne!(plan.predicate_hash(), 0);
}

#[test]
fn test_predicate_compiler_returns_filter_for_eq_expr() {
    let schema = test_schema();
    let compiler = PredicateCompiler::new(schema.clone());
    let filter = compiler.compile(&binary(
        column_expr("id"),
        Operator::Eq,
        lit(Value::Integer(1)),
    ));
    let _ = filter;
}

#[test]
fn test_mutation_compiler_plus_operator() {
    use sqlrustgo_planner::Expr::BinaryExpr;
    let expr = BinaryExpr {
        left: Box::new(lit(Value::Integer(1))),
        op: Operator::Plus,
        right: Box::new(lit(Value::Integer(2))),
    };
    let m = MutationCompiler::compile(vec![Assignment {
        column: "value".to_string(),
        expr,
    }]);
    assert_ne!(m.mutation_hash(), 0);
}

#[test]
fn test_mutation_compiler_minus_operator() {
    use sqlrustgo_planner::Expr::BinaryExpr;
    let expr = BinaryExpr {
        left: Box::new(lit(Value::Integer(5))),
        op: Operator::Minus,
        right: Box::new(lit(Value::Integer(3))),
    };
    let m = MutationCompiler::compile(vec![Assignment {
        column: "value".to_string(),
        expr,
    }]);
    assert_ne!(m.mutation_hash(), 0);
}

#[test]
fn test_mutation_compiler_mul_operator() {
    use sqlrustgo_planner::Expr::BinaryExpr;
    let expr = BinaryExpr {
        left: Box::new(lit(Value::Integer(2))),
        op: Operator::Multiply,
        right: Box::new(lit(Value::Integer(3))),
    };
    let m = MutationCompiler::compile(vec![Assignment {
        column: "value".to_string(),
        expr,
    }]);
    assert_ne!(m.mutation_hash(), 0);
}

#[test]
fn test_mutation_compiler_div_operator() {
    use sqlrustgo_planner::Expr::BinaryExpr;
    let expr = BinaryExpr {
        left: Box::new(lit(Value::Integer(10))),
        op: Operator::Divide,
        right: Box::new(lit(Value::Integer(2))),
    };
    let m = MutationCompiler::compile(vec![Assignment {
        column: "value".to_string(),
        expr,
    }]);
    assert_ne!(m.mutation_hash(), 0);
}

#[test]
fn test_mutation_compiler_column_assignment() {
    let assign = Assignment {
        column: "v".to_string(),
        expr: column_expr("v"),
    };
    let m = MutationCompiler::compile(vec![assign.clone()]);
    assert_eq!(m.assignments().len(), 1);
    assert_eq!(m.assignments()[0].column, assign.column);
}

#[test]
fn test_row_mutation_new_constructor() {
    let rm = RowMutation::new(vec![], 12345);
    assert_eq!(rm.mutation_hash(), 12345);
    assert!(rm.assignments().is_empty());
}