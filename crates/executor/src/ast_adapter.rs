use sqlrustgo_parser::parser::{Expression, UpdateStatement as ParserUpdateStatement};
use sqlrustgo_storage::engine::TableInfo;
use sqlrustgo_storage::vtu_ir::{AssignmentIR, ExprIR, MutationIR, PredicateIR, UpdatePlan};
use sqlrustgo_types::{SqlError, Value};

const MAX_EXPR_DEPTH: usize = 32;

pub struct AstAdapter;

impl AstAdapter {
    pub fn to_update_plan(
        stmt: &ParserUpdateStatement,
        table_info: &TableInfo,
    ) -> Result<UpdatePlan, SqlError> {
        let table = stmt.table.clone();

        let predicate = match &stmt.where_clause {
            None => PredicateIR::All,
            Some(expr) => {
                let expr_ir = Self::convert_expr_with_depth(expr, 0)?;
                PredicateIR::Expr(expr_ir)
            }
        };

        let assignments = Self::convert_assignments(&stmt.set_clauses, table_info)?;

        let mutation = MutationIR::new(assignments);

        let update_plan = UpdatePlan::new(table, predicate, mutation, 0);
        Ok(update_plan)
    }

    fn convert_expr_with_depth(expr: &Expression, depth: usize) -> Result<ExprIR, SqlError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(SqlError::ExecutionError(
                "expression nesting too deep".into(),
            ));
        }
        Self::convert_expr(expr, depth)
    }

    fn convert_expr(expr: &Expression, depth: usize) -> Result<ExprIR, SqlError> {
        match expr {
            Expression::Identifier(name) => Ok(ExprIR::Column(name.clone())),
            Expression::Literal(s) => {
                let val = Self::parse_literal(s);
                Ok(ExprIR::Literal(val))
            }
            Expression::BinaryOp(left, op, right) => {
                let l = Self::convert_expr_with_depth(left, depth + 1)?;
                let r = Self::convert_expr_with_depth(right, depth + 1)?;
                Ok(ExprIR::Binary {
                    op: op.clone(),
                    left: Box::new(l),
                    right: Box::new(r),
                })
            }
            Expression::UnaryOp(op, inner) => {
                let e = Self::convert_expr_with_depth(inner, depth + 1)?;
                Ok(ExprIR::Unary {
                    op: op.clone(),
                    expr: Box::new(e),
                })
            }
            Expression::IsNull(inner) => {
                let e = Self::convert_expr_with_depth(inner, depth + 1)?;
                Ok(ExprIR::IsNull(Box::new(e)))
            }
            Expression::IsNotNull(inner) => {
                let e = Self::convert_expr_with_depth(inner, depth + 1)?;
                Ok(ExprIR::IsNotNull(Box::new(e)))
            }
            _ => Err(SqlError::ExecutionError(format!(
                "unsupported expression type in UPDATE: {:?}",
                expr
            ))),
        }
    }

    fn parse_literal(s: &str) -> Value {
        if s.eq_ignore_ascii_case("NULL") || s.eq_ignore_ascii_case("NULL") {
            return Value::Null;
        }
        if s.eq_ignore_ascii_case("TRUE") || s.eq_ignore_ascii_case("FALSE") {
            return Value::Boolean(s.eq_ignore_ascii_case("TRUE"));
        }
        if let Ok(i) = s.parse::<i64>() {
            return Value::Integer(i);
        }
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }
        Value::Text(s.to_string())
    }

    fn convert_assignments(
        set_clauses: &[(String, Expression)],
        table_info: &TableInfo,
    ) -> Result<Vec<AssignmentIR>, SqlError> {
        let mut assignments = Vec::new();
        for (col_name, expr) in set_clauses {
            let col_idx = table_info
                .columns
                .iter()
                .position(|c| c.name == *col_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("column not found: {}", col_name))
                })?;
            let expr_ir = Self::convert_expr_with_depth(expr, 0)?;
            assignments.push(AssignmentIR {
                column: col_name.clone(),
                column_index: col_idx,
                expr: expr_ir,
            });
        }
        Ok(assignments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_parser::Expression;
    use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
    use sqlrustgo_storage::vtu_ir::ExprIR;

    fn make_table_info() -> TableInfo {
        TableInfo {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "active".to_string(),
                    data_type: "BOOLEAN".to_string(),
                    nullable: false,
                    ..Default::default()
                },
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
        }
    }

    #[test]
    fn test_to_update_plan_no_where_clause() {
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![("name".to_string(), Expression::Literal("alice".to_string()))],
            where_clause: None,
        };
        let info = make_table_info();
        let plan = AstAdapter::to_update_plan(&stmt, &info).expect("plan ok");
        assert_eq!(plan.table, "users");
        assert!(matches!(plan.predicate, PredicateIR::All));
        assert_eq!(plan.mutation.assignments().len(), 1);
        assert_eq!(plan.mutation.assignments()[0].column, "name");
        assert_eq!(plan.mutation.assignments()[0].column_index, 1);
    }

    #[test]
    fn test_to_update_plan_with_where_clause() {
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![(
                "active".to_string(),
                Expression::Literal("FALSE".to_string()),
            )],
            where_clause: Some(Expression::BinaryOp(
                Box::new(Expression::Identifier("id".to_string())),
                "=".to_string(),
                Box::new(Expression::Literal("42".to_string())),
            )),
        };
        let info = make_table_info();
        let plan = AstAdapter::to_update_plan(&stmt, &info).expect("plan ok");
        match &plan.predicate {
            PredicateIR::Expr(ExprIR::Binary { op, left, right }) => {
                assert_eq!(op, "=");
                assert!(matches!(**left, ExprIR::Column(ref n) if n == "id"));
                assert!(matches!(**right, ExprIR::Literal(Value::Integer(42))));
            }
            _ => panic!("expected binary predicate"),
        }
    }

    #[test]
    fn test_to_update_plan_multiple_assignments() {
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![
                ("name".to_string(), Expression::Literal("bob".to_string())),
                (
                    "active".to_string(),
                    Expression::Literal("TRUE".to_string()),
                ),
            ],
            where_clause: None,
        };
        let info = make_table_info();
        let plan = AstAdapter::to_update_plan(&stmt, &info).expect("plan ok");
        assert_eq!(plan.mutation.assignments().len(), 2);
    }

    #[test]
    fn test_to_update_plan_unknown_column_errors() {
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![(
                "nonexistent".to_string(),
                Expression::Literal("x".to_string()),
            )],
            where_clause: None,
        };
        let info = make_table_info();
        let err = AstAdapter::to_update_plan(&stmt, &info).expect_err("should fail");
        assert!(format!("{:?}", err).contains("column not found"));
    }

    #[test]
    fn test_to_update_plan_unsupported_expression() {
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![("name".to_string(), Expression::Literal("x".to_string()))],
            where_clause: Some(Expression::FunctionCall("NOW".to_string(), vec![])),
        };
        let info = make_table_info();
        let err = AstAdapter::to_update_plan(&stmt, &info).expect_err("should fail");
        assert!(format!("{:?}", err).contains("unsupported expression"));
    }

    #[test]
    fn test_to_update_plan_isnull_isnotnull() {
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![(
                "name".to_string(),
                Expression::Literal("default".to_string()),
            )],
            where_clause: Some(Expression::IsNull(Box::new(Expression::Identifier(
                "name".to_string(),
            )))),
        };
        let info = make_table_info();
        let plan = AstAdapter::to_update_plan(&stmt, &info).expect("plan ok");
        match &plan.predicate {
            PredicateIR::Expr(ExprIR::IsNull(inner)) => {
                assert!(matches!(**inner, ExprIR::Column(ref n) if n == "name"));
            }
            _ => panic!("expected IsNull"),
        }

        let stmt2 = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![(
                "name".to_string(),
                Expression::Literal("default".to_string()),
            )],
            where_clause: Some(Expression::IsNotNull(Box::new(Expression::Identifier(
                "name".to_string(),
            )))),
        };
        let plan2 = AstAdapter::to_update_plan(&stmt2, &info).expect("plan ok");
        assert!(matches!(
            &plan2.predicate,
            PredicateIR::Expr(ExprIR::IsNotNull(_))
        ));
    }

    #[test]
    fn test_to_update_plan_unary_not() {
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![("name".to_string(), Expression::Literal("x".to_string()))],
            where_clause: Some(Expression::UnaryOp(
                "NOT".to_string(),
                Box::new(Expression::Identifier("active".to_string())),
            )),
        };
        let info = make_table_info();
        let plan = AstAdapter::to_update_plan(&stmt, &info).expect("plan ok");
        match &plan.predicate {
            PredicateIR::Expr(ExprIR::Unary { op, expr }) => {
                assert_eq!(op, "NOT");
                assert!(matches!(**expr, ExprIR::Column(ref n) if n == "active"));
            }
            _ => panic!("expected Unary NOT"),
        }
    }

    #[test]
    fn test_to_update_plan_nested_expression_too_deep() {
        let mut inner = Expression::Literal("1".to_string());
        for _ in 0..MAX_EXPR_DEPTH + 1 {
            inner = Expression::UnaryOp("NOT".to_string(), Box::new(inner));
        }
        let stmt = ParserUpdateStatement {
            table: "users".to_string(),
            set_clauses: vec![("name".to_string(), Expression::Literal("x".to_string()))],
            where_clause: Some(inner),
        };
        let info = make_table_info();
        let err = AstAdapter::to_update_plan(&stmt, &info).expect_err("should fail");
        assert!(format!("{:?}", err).contains("nesting too deep"));
    }

    #[test]
    fn test_parse_literal_variants() {
        assert!(matches!(AstAdapter::parse_literal("NULL"), Value::Null));
        assert!(matches!(AstAdapter::parse_literal("null"), Value::Null));
        assert!(matches!(
            AstAdapter::parse_literal("TRUE"),
            Value::Boolean(true)
        ));
        assert!(matches!(
            AstAdapter::parse_literal("false"),
            Value::Boolean(false)
        ));
        assert!(matches!(
            AstAdapter::parse_literal("42"),
            Value::Integer(42)
        ));
        assert!(matches!(
            AstAdapter::parse_literal("-7"),
            Value::Integer(-7)
        ));
        assert!(matches!(
            AstAdapter::parse_literal("3.14"),
            Value::Float(f) if (f - 3.14).abs() < 1e-9
        ));
        assert!(matches!(
            AstAdapter::parse_literal("hello"),
            Value::Text(s) if s == "hello"
        ));
    }
}
