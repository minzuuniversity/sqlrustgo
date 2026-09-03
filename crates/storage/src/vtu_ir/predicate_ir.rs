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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ColumnDefinition;

    fn make_table() -> TableInfo {
        TableInfo {
            name: "t".to_string(),
            columns: vec![
                ColumnDefinition::new("a", "INTEGER"),
                ColumnDefinition::new("b", "TEXT"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],

            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        original_sql: String::new(),
        }
    }

    #[test]
    fn test_predicate_all_matches_any_row() {
        let pred = PredicateIR::All;
        let table = make_table();
        let row = vec![Value::Integer(1), Value::Text("x".into())];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_expr_column_match_true() {
        let pred = PredicateIR::Expr(ExprIR::Column("a".to_string()));
        let table = make_table();
        let row = vec![Value::Boolean(true), Value::Null];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_expr_column_match_false() {
        let pred = PredicateIR::Expr(ExprIR::Column("a".to_string()));
        let table = make_table();
        let row = vec![Value::Boolean(false), Value::Null];
        assert!(!pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_expr_literal_true() {
        let pred = PredicateIR::Expr(ExprIR::Literal(Value::Boolean(true)));
        let table = make_table();
        let row = vec![Value::Null, Value::Null];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_expr_literal_false() {
        let pred = PredicateIR::Expr(ExprIR::Literal(Value::Boolean(false)));
        let table = make_table();
        let row = vec![Value::Null, Value::Null];
        assert!(!pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_expr_literal_non_bool() {
        let pred = PredicateIR::Expr(ExprIR::Literal(Value::Integer(42)));
        let table = make_table();
        let row = vec![Value::Null, Value::Null];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_is_null_true() {
        let pred = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column("a".to_string()))));
        let table = make_table();
        let row = vec![Value::Null, Value::Null];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_is_null_false() {
        let pred = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column("a".to_string()))));
        let table = make_table();
        let row = vec![Value::Integer(42), Value::Null];
        assert!(!pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_is_not_null_true() {
        let pred = PredicateIR::Expr(ExprIR::IsNotNull(Box::new(ExprIR::Column("a".to_string()))));
        let table = make_table();
        let row = vec![Value::Integer(42), Value::Null];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_is_not_null_false() {
        let pred = PredicateIR::Expr(ExprIR::IsNotNull(Box::new(ExprIR::Column("a".to_string()))));
        let table = make_table();
        let row = vec![Value::Null, Value::Null];
        assert!(!pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_unary_not() {
        let pred = PredicateIR::Expr(ExprIR::Unary {
            op: "NOT".to_string(),
            expr: Box::new(ExprIR::Literal(Value::Boolean(true))),
        });
        let table = make_table();
        let row = vec![];
        assert!(!pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_binary_and() {
        let pred = PredicateIR::Expr(ExprIR::Binary {
            op: "AND".to_string(),
            left: Box::new(ExprIR::Literal(Value::Boolean(true))),
            right: Box::new(ExprIR::Literal(Value::Boolean(false))),
        });
        let table = make_table();
        let row = vec![];
        assert!(!pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_binary_or() {
        let pred = PredicateIR::Expr(ExprIR::Binary {
            op: "OR".to_string(),
            left: Box::new(ExprIR::Literal(Value::Boolean(false))),
            right: Box::new(ExprIR::Literal(Value::Boolean(true))),
        });
        let table = make_table();
        let row = vec![];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_binary_eq() {
        let pred = PredicateIR::Expr(ExprIR::Binary {
            op: "=".to_string(),
            left: Box::new(ExprIR::Literal(Value::Integer(5))),
            right: Box::new(ExprIR::Literal(Value::Integer(5))),
        });
        let table = make_table();
        let row = vec![];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_binary_gt() {
        let pred = PredicateIR::Expr(ExprIR::Binary {
            op: ">".to_string(),
            left: Box::new(ExprIR::Literal(Value::Integer(10))),
            right: Box::new(ExprIR::Literal(Value::Integer(5))),
        });
        let table = make_table();
        let row = vec![];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_binary_lt() {
        let pred = PredicateIR::Expr(ExprIR::Binary {
            op: "<".to_string(),
            left: Box::new(ExprIR::Literal(Value::Integer(5))),
            right: Box::new(ExprIR::Literal(Value::Integer(10))),
        });
        let table = make_table();
        let row = vec![];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_binary_neq() {
        let pred = PredicateIR::Expr(ExprIR::Binary {
            op: "!=".to_string(),
            left: Box::new(ExprIR::Literal(Value::Integer(5))),
            right: Box::new(ExprIR::Literal(Value::Integer(10))),
        });
        let table = make_table();
        let row = vec![];
        assert!(pred.evaluate(&row, &table));
    }

    #[test]
    fn test_predicate_unknown_op_returns_false() {
        let pred = PredicateIR::Expr(ExprIR::Binary {
            op: "BOGUS".to_string(),
            left: Box::new(ExprIR::Literal(Value::Boolean(true))),
            right: Box::new(ExprIR::Literal(Value::Boolean(true))),
        });
        let table = make_table();
        let row = vec![];
        assert!(!pred.evaluate(&row, &table));
    }

    #[test]
    fn test_cmp_values() {
        assert_eq!(
            super::PredicateIR::cmp_values(&Value::Integer(1), &Value::Integer(2)),
            -1
        );
        assert_eq!(
            super::PredicateIR::cmp_values(&Value::Integer(2), &Value::Integer(1)),
            1
        );
        assert_eq!(
            super::PredicateIR::cmp_values(&Value::Integer(1), &Value::Integer(1)),
            0
        );
    }
}
