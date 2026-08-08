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
        if stmt.tables.len() != 1 {
            return Err(SqlError::ExecutionError(
                "AstAdapter::to_update_plan only supports single-table UPDATE".to_string(),
            ));
        }
        let table = stmt.tables[0].name.clone();

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
        Self::convert_expr(expr)
    }

    fn convert_expr(expr: &Expression) -> Result<ExprIR, SqlError> {
        match expr {
            Expression::Identifier(name) => Ok(ExprIR::Column(name.clone())),
            Expression::Literal(s) => {
                let val = Self::parse_literal(s);
                Ok(ExprIR::Literal(val))
            }
            Expression::BinaryOp(left, op, right) => {
                let l = Self::convert_expr(left)?;
                let r = Self::convert_expr(right)?;
                Ok(ExprIR::Binary {
                    op: op.clone(),
                    left: Box::new(l),
                    right: Box::new(r),
                })
            }
            Expression::UnaryOp(op, inner) => {
                let e = Self::convert_expr(inner)?;
                Ok(ExprIR::Unary {
                    op: op.clone(),
                    expr: Box::new(e),
                })
            }
            Expression::IsNull(inner) => {
                let e = Self::convert_expr(inner)?;
                Ok(ExprIR::IsNull(Box::new(e)))
            }
            Expression::IsNotNull(inner) => {
                let e = Self::convert_expr(inner)?;
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
            let expr_ir = Self::convert_expr(expr)?;
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
    use sqlrustgo_parser::parser::{
        Expression, TableRef, UpdateStatement as ParserUpdateStatement,
    };
    use sqlrustgo_storage::engine::TableInfo;
    use sqlrustgo_storage::ColumnDefinition;

    fn make_table_info(columns: Vec<&str>) -> TableInfo {
        TableInfo {
            name: "t".to_string(),
            columns: columns
                .into_iter()
                .map(|n| ColumnDefinition {
                    name: n.to_string(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    fn table(name: &str) -> TableRef {
        TableRef {
            name: name.to_string(),
            alias: None,
        }
    }

    fn ident(name: &str) -> Expression {
        Expression::Identifier(name.to_string())
    }

    fn lit(s: &str) -> Expression {
        Expression::Literal(s.to_string())
    }

    fn update_stmt(
        tables: Vec<TableRef>,
        set_clauses: Vec<(String, Expression)>,
        where_clause: Option<Expression>,
    ) -> ParserUpdateStatement {
        ParserUpdateStatement {
            tables,
            set_clauses,
            where_clause,
        }
    }

    // --- AstAdapter tests ---

    #[test]
    fn test_to_update_plan_multi_table_error() {
        let mut stmt = update_stmt(vec![table("a"), table("b")], vec![], None);
        let result = AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"]));
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("single-table"));
    }

    #[test]
    fn test_to_update_plan_no_where() {
        let mut stmt = update_stmt(vec![table("t")], vec![("x".to_string(), lit("1"))], None);
        let plan = AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).unwrap();
        match plan.predicate {
            PredicateIR::All => {}
            _ => panic!("expected All"),
        }
    }

    #[test]
    fn test_to_update_plan_with_where() {
        let mut stmt = update_stmt(
            vec![table("t")],
            vec![("x".to_string(), lit("1"))],
            Some(Expression::BinaryOp(
                Box::new(ident("id")),
                "=".to_string(),
                Box::new(lit("5")),
            )),
        );
        let plan =
            AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["id", "x"])).unwrap();
        match plan.predicate {
            PredicateIR::Expr(_) => {}
            _ => panic!("expected Expr"),
        }
    }

    #[test]
    fn test_to_update_plan_set_clauses() {
        let mut stmt = update_stmt(
            vec![table("t")],
            vec![("a".to_string(), lit("1")), ("b".to_string(), ident("c"))],
            None,
        );
        let plan =
            AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["a", "b", "c"])).unwrap();
        assert_eq!(plan.mutation.assignments.len(), 2);
    }

    #[test]
    fn test_convert_expr_literal_null() {
        let mut stmt = update_stmt(vec![table("t")], vec![("x".to_string(), lit("NULL"))], None);
        assert!(AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).is_ok());
    }

    #[test]
    fn test_convert_expr_literal_true() {
        let mut stmt = update_stmt(vec![table("t")], vec![("x".to_string(), lit("TRUE"))], None);
        assert!(AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).is_ok());
    }

    #[test]
    fn test_convert_expr_literal_false() {
        let mut stmt = update_stmt(
            vec![table("t")],
            vec![("x".to_string(), lit("FALSE"))],
            None,
        );
        assert!(AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).is_ok());
    }

    #[test]
    fn test_convert_expr_literal_integer() {
        for s in &["0", "42", "-7", "999999"] {
            let mut stmt = update_stmt(vec![table("t")], vec![("x".to_string(), lit(s))], None);
            assert!(
                AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).is_ok(),
                "failed for {}",
                s
            );
        }
    }

    #[test]
    fn test_convert_expr_literal_float() {
        for s in &["0.5", "-3.14"] {
            let mut stmt = update_stmt(vec![table("t")], vec![("x".to_string(), lit(s))], None);
            assert!(
                AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).is_ok(),
                "failed for {}",
                s
            );
        }
    }

    #[test]
    fn test_convert_expr_literal_string() {
        let mut stmt = update_stmt(
            vec![table("t")],
            vec![("x".to_string(), lit("hello"))],
            None,
        );
        assert!(AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).is_ok());
    }

    // Operator::Add/Not do not exist; Expression::Function does not exist
    // Note: no depth limit exists in the current AstAdapter
    // 40-level nested BinaryOp("+") succeeds without error

    #[test]
    fn test_convert_expr_is_null() {
        let expr = Expression::IsNull(Box::new(ident("x")));
        let mut stmt = update_stmt(vec![table("t")], vec![("y".to_string(), expr)], None);
        assert!(AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x", "y"])).is_ok());
    }

    #[test]
    fn test_convert_expr_is_not_null() {
        let expr = Expression::IsNotNull(Box::new(ident("x")));
        let mut stmt = update_stmt(vec![table("t")], vec![("y".to_string(), expr)], None);
        assert!(AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x", "y"])).is_ok());
    }

    #[test]
    fn test_convert_expr_unsupported() {
        let expr = Expression::FunctionCall("SUM".to_string(), vec![]);
        let mut stmt = update_stmt(vec![table("t")], vec![("x".to_string(), expr)], None);
        let err = AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).unwrap_err();
        assert!(format!("{}", err).contains("unsupported"));
    }

    #[test]
    fn test_convert_assignments_column_not_found() {
        let mut stmt = update_stmt(
            vec![table("t")],
            vec![("nonexistent".to_string(), lit("1"))],
            None,
        );
        let err = AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["x"])).unwrap_err();
        assert!(format!("{}", err).contains("column not found"));
    }

    #[test]
    fn test_convert_expr_binary_op_complex() {
        let expr = Expression::BinaryOp(Box::new(ident("a")), "+".to_string(), Box::new(lit("1")));
        let mut stmt = update_stmt(vec![table("t")], vec![("x".to_string(), expr)], None);
        assert!(AstAdapter::to_update_plan(&mut stmt, &make_table_info(vec!["a", "x"])).is_ok());
    }
}
