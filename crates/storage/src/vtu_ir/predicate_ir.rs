use crate::engine::TableInfo;
use sqlrustgo_types::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExprIR {
    Column(String),
    Literal(Value),
    Binary {
        op: String,
        left: Box<ExprIR>,
        right: Box<ExprIR>,
    },
    Unary {
        op: String,
        expr: Box<ExprIR>,
    },
    IsNull(Box<ExprIR>),
    IsNotNull(Box<ExprIR>),
}

#[derive(Debug, Clone)]
pub enum PredicateIR {
    Expr(ExprIR),
    All,
}

impl PredicateIR {
    pub fn evaluate(&self, row: &[Value], table_info: &TableInfo) -> bool {
        match self {
            PredicateIR::All => true,
            PredicateIR::Expr(expr) => Self::eval_expr(expr, row, table_info),
        }
    }

    fn eval_expr(expr: &ExprIR, row: &[Value], table_info: &TableInfo) -> bool {
        match expr {
            ExprIR::Column(name) => {
                if let Some(idx) = Self::find_column_index(name, table_info) {
                    matches!(row.get(idx), Some(Value::Boolean(true)))
                } else {
                    false
                }
            }
            ExprIR::Literal(Value::Boolean(b)) => *b,
            ExprIR::Literal(_) => true,
            ExprIR::IsNull(inner) => {
                if let Some(idx) = Self::resolve_column_index(inner, row, table_info) {
                    matches!(row.get(idx), Some(Value::Null))
                } else {
                    false
                }
            }
            ExprIR::IsNotNull(inner) => {
                if let Some(idx) = Self::resolve_column_index(inner, row, table_info) {
                    !matches!(row.get(idx), Some(Value::Null))
                } else {
                    false
                }
            }
            ExprIR::Unary { op, expr } => {
                let op_upper = op.to_uppercase();
                if op_upper == "NOT" {
                    !Self::eval_expr(expr, row, table_info)
                } else {
                    false
                }
            }
            ExprIR::Binary { op, left, right } => {
                let op_upper = op.to_uppercase();
                match op_upper.as_str() {
                    "AND" | "&&" => {
                        Self::eval_expr(left, row, table_info)
                            && Self::eval_expr(right, row, table_info)
                    }
                    "OR" | "||" => {
                        Self::eval_expr(left, row, table_info)
                            || Self::eval_expr(right, row, table_info)
                    }
                    "=" | "==" => Self::eval_cmp(left, right, row, table_info, |a, b| a == b),
                    "!=" | "<>" => Self::eval_cmp(left, right, row, table_info, |a, b| a != b),
                    ">" => Self::eval_cmp(left, right, row, table_info, |a, b| {
                        Self::cmp_values(a, b) > 0
                    }),
                    ">=" => Self::eval_cmp(left, right, row, table_info, |a, b| {
                        Self::cmp_values(a, b) >= 0
                    }),
                    "<" => Self::eval_cmp(left, right, row, table_info, |a, b| {
                        Self::cmp_values(a, b) < 0
                    }),
                    "<=" => Self::eval_cmp(left, right, row, table_info, |a, b| {
                        Self::cmp_values(a, b) <= 0
                    }),
                    _ => false,
                }
            }
        }
    }

    fn eval_cmp<F>(
        left: &ExprIR,
        right: &ExprIR,
        row: &[Value],
        table_info: &TableInfo,
        cmp: F,
    ) -> bool
    where
        F: Fn(&Value, &Value) -> bool,
    {
        let left_val = Self::eval_to_value(left, row, table_info);
        let right_val = Self::eval_to_value(right, row, table_info);
        if matches!(left_val, Value::Null) || matches!(right_val, Value::Null) {
            return false;
        }
        cmp(&left_val, &right_val)
    }

    fn eval_to_value(expr: &ExprIR, row: &[Value], table_info: &TableInfo) -> Value {
        match expr {
            ExprIR::Column(name) => {
                if let Some(idx) = Self::find_column_index(name, table_info) {
                    row.get(idx).cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            ExprIR::Literal(val) => val.clone(),
            ExprIR::IsNull(inner) => {
                let idx = Self::resolve_column_index(inner, row, table_info);
                Value::Boolean(
                    idx.map(|i| matches!(row.get(i), Some(Value::Null)))
                        .unwrap_or(false),
                )
            }
            ExprIR::IsNotNull(inner) => {
                let idx = Self::resolve_column_index(inner, row, table_info);
                Value::Boolean(
                    !idx.map(|i| matches!(row.get(i), Some(Value::Null)))
                        .unwrap_or(false),
                )
            }
            ExprIR::Unary { op, expr } => {
                let op_upper = op.to_uppercase();
                if op_upper == "NOT" {
                    let inner = Self::eval_to_value(expr, row, table_info);
                    if let Value::Boolean(b) = inner {
                        Value::Boolean(!b)
                    } else {
                        Value::Boolean(false)
                    }
                } else {
                    Value::Null
                }
            }
            ExprIR::Binary { op, left, right } => {
                let op_upper = op.to_uppercase();
                let left_val = Self::eval_to_value(left, row, table_info);
                let right_val = Self::eval_to_value(right, row, table_info);
                match op_upper.as_str() {
                    "AND" | "&&" => {
                        if let (Value::Boolean(l), Value::Boolean(r)) = (&left_val, &right_val) {
                            Value::Boolean(*l && *r)
                        } else {
                            Value::Boolean(false)
                        }
                    }
                    "OR" | "||" => {
                        if let (Value::Boolean(l), Value::Boolean(r)) = (&left_val, &right_val) {
                            Value::Boolean(*l || *r)
                        } else {
                            Value::Boolean(false)
                        }
                    }
                    _ => Value::Null, // Comparisons go through eval_cmp
                }
            }
        }
    }

    fn find_column_index(name: &str, table_info: &TableInfo) -> Option<usize> {
        table_info.columns.iter().position(|c| c.name == name)
    }

    fn resolve_column_index(
        expr: &ExprIR,
        _row: &[Value],
        table_info: &TableInfo,
    ) -> Option<usize> {
        if let ExprIR::Column(name) = expr {
            Self::find_column_index(name, table_info)
        } else {
            None
        }
    }

    fn cmp_values(left: &Value, right: &Value) -> i32 {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => l.cmp(r) as i32,
            (Value::Float(l), Value::Float(r)) => {
                if l < r {
                    -1
                } else if l > r {
                    1
                } else {
                    0
                }
            }
            (Value::Text(l), Value::Text(r)) => l.cmp(r) as i32,
            (Value::Null, Value::Null) => 0,
            (Value::Null, _) => -1,
            (_, Value::Null) => 1,
            _ => 0,
        }
    }
}
