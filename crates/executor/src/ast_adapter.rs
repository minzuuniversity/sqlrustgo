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
