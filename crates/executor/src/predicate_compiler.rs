use sqlrustgo_planner::{Expr, Operator, Schema};
use sqlrustgo_storage::engine::{Record, RowFilter};

#[allow(dead_code)]
pub struct PredicateCompiler {
    schema: Schema,
}

impl PredicateCompiler {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }

    pub fn compile(expr: &Expr) -> RowFilter {
        match expr {
            Expr::Column(col) => {
                let col_name = col.name.clone();
                Box::new(move |row: &Record| -> bool {
                    if let Some(idx) = Self::find_column_index(row, &col_name) {
                        if let sqlrustgo_types::Value::Boolean(b) = &row[idx] {
                            return *b;
                        }
                    }
                    false
                })
            }
            Expr::Literal(_val) => Box::new(|_| true),
            Expr::BinaryExpr { left, op, right } => {
                let left_filter = Self::compile(left);
                let right_filter = Self::compile(right);
                let op = op.clone();
                Box::new(move |row| {
                    let l = left_filter(row);
                    let r = right_filter(row);
                    Self::eval_bool_op(&op, l, r)
                })
            }
            Expr::UnaryExpr { op, expr } => {
                let inner = Self::compile(expr);
                let op = op.clone();
                Box::new(move |row| {
                    let v = inner(row);
                    Self::eval_unary_bool_op(&op, v)
                })
            }
            Expr::Wildcard | Expr::QualifiedWildcard { .. } => Box::new(|_| true),
            _ => Box::new(|_| true),
        }
    }

    fn find_column_index(row: &Record, col_name: &str) -> Option<usize> {
        row.iter().position(|v| {
            if let sqlrustgo_types::Value::Text(s) = v {
                s == col_name
            } else {
                false
            }
        })
    }

    fn eval_bool_op(op: &Operator, left: bool, right: bool) -> bool {
        match op {
            Operator::Eq => left == right,
            Operator::NotEq => left != right,
            Operator::And => left && right,
            Operator::Or => left || right,
            _ => false,
        }
    }

    fn eval_unary_bool_op(op: &Operator, val: bool) -> bool {
        match op {
            Operator::Not => !val,
            _ => false,
        }
    }

    pub fn compile_optional(expr: Option<&Expr>) -> Option<RowFilter> {
        expr.map(|e| Self::compile(e))
    }
}
