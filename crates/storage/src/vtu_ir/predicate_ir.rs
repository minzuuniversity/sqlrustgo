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
    use crate::engine::{ColumnDefinition, TableInfo};

    fn table() -> TableInfo {
        TableInfo {
            name: "t".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "active".to_string(),
                    data_type: "BOOLEAN".to_string(),
                    nullable: false,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: true,
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
    fn test_predicate_all_matches_any_row() {
        let p = PredicateIR::All;
        assert!(p.evaluate(&[Value::Integer(1)], &table()));
        assert!(p.evaluate(&[], &table()));
    }

    #[test]
    fn test_eval_column_boolean_value() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::Column("active".to_string()));
        assert!(p.evaluate(&[Value::Integer(1), Value::Boolean(true), Value::Text("a".into())], &info));
        assert!(!p.evaluate(&[Value::Integer(1), Value::Boolean(false), Value::Text("a".into())], &info));
    }

    #[test]
    fn test_eval_column_missing_returns_false() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::Column("missing".to_string()));
        assert!(!p.evaluate(&[Value::Integer(1)], &info));
    }

    #[test]
    fn test_eval_literal_boolean() {
        let p = PredicateIR::Expr(ExprIR::Literal(Value::Boolean(true)));
        assert!(p.evaluate(&[], &table()));
        let p2 = PredicateIR::Expr(ExprIR::Literal(Value::Boolean(false)));
        assert!(!p2.evaluate(&[], &table()));
    }

    #[test]
    fn test_eval_literal_non_boolean_is_true() {
        let p = PredicateIR::Expr(ExprIR::Literal(Value::Integer(42)));
        assert!(p.evaluate(&[], &table()));
    }

    #[test]
    fn test_eval_isnull_and_isnotnull() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Column("name".to_string()))));
        assert!(p.evaluate(&[Value::Integer(1), Value::Boolean(false), Value::Null], &info));
        assert!(!p.evaluate(&[Value::Integer(1), Value::Boolean(false), Value::Text("a".into())], &info));

        let p2 = PredicateIR::Expr(ExprIR::IsNotNull(Box::new(ExprIR::Column("name".to_string()))));
        assert!(p2.evaluate(&[Value::Integer(1), Value::Boolean(false), Value::Text("a".into())], &info));
        assert!(!p2.evaluate(&[Value::Integer(1), Value::Boolean(false), Value::Null], &info));
    }

    #[test]
    fn test_eval_isnull_on_non_column_returns_false() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::IsNull(Box::new(ExprIR::Literal(Value::Null))));
        assert!(!p.evaluate(&[Value::Integer(1)], &info));
    }

    #[test]
    fn test_eval_unary_not() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::Unary {
            op: "NOT".to_string(),
            expr: Box::new(ExprIR::Column("active".to_string())),
        });
        assert!(!p.evaluate(&[Value::Integer(1), Value::Boolean(true), Value::Null], &info));
        assert!(p.evaluate(&[Value::Integer(1), Value::Boolean(false), Value::Null], &info));
    }

    #[test]
    fn test_eval_unary_other_returns_false() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::Unary {
            op: "ABS".to_string(),
            expr: Box::new(ExprIR::Column("active".to_string())),
        });
        assert!(!p.evaluate(&[Value::Integer(1), Value::Boolean(true), Value::Null], &info));
    }

    #[test]
    fn test_eval_binary_logical_operators() {
        let info = table();
        let row_active_true = &[Value::Integer(1), Value::Boolean(true), Value::Null];
        let row_active_false = &[Value::Integer(1), Value::Boolean(false), Value::Null];

        for op in ["AND", "OR", "&&", "||"] {
            let p = PredicateIR::Expr(ExprIR::Binary {
                op: op.to_string(),
                left: Box::new(ExprIR::Column("active".to_string())),
                right: Box::new(ExprIR::Column("active".to_string())),
            });
            assert_eq!(p.evaluate(row_active_true, &info), true, "op={}", op);
            assert_eq!(p.evaluate(row_active_false, &info), false, "op={}", op);
        }
    }

    #[test]
    fn test_eval_binary_comparisons() {
        let info = table();
        let row = &[Value::Integer(5), Value::Boolean(true), Value::Text("hello".into())];

        let p = PredicateIR::Expr(ExprIR::Binary {
            op: "=".to_string(),
            left: Box::new(ExprIR::Column("id".to_string())),
            right: Box::new(ExprIR::Literal(Value::Integer(5))),
        });
        assert!(p.evaluate(row, &info));

        let p2 = PredicateIR::Expr(ExprIR::Binary {
            op: "!=".to_string(),
            left: Box::new(ExprIR::Column("id".to_string())),
            right: Box::new(ExprIR::Literal(Value::Integer(5))),
        });
        assert!(!p2.evaluate(row, &info));
    }

    #[test]
    fn test_eval_binary_greater_less() {
        let info = table();
        let row = &[Value::Integer(5), Value::Boolean(true), Value::Null];

        for (op, expected) in [(">", false), (">=", true), ("<", false), ("<=", true)] {
            let p = PredicateIR::Expr(ExprIR::Binary {
                op: op.to_string(),
                left: Box::new(ExprIR::Column("id".to_string())),
                right: Box::new(ExprIR::Literal(Value::Integer(5))),
            });
            assert_eq!(p.evaluate(row, &info), expected, "op={}", op);
        }
    }

    #[test]
    fn test_eval_binary_null_returns_false() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::Binary {
            op: "=".to_string(),
            left: Box::new(ExprIR::Column("name".to_string())),
            right: Box::new(ExprIR::Literal(Value::Text("x".into()))),
        });
        assert!(!p.evaluate(&[Value::Integer(1), Value::Boolean(true), Value::Null], &info));
    }

    #[test]
    fn test_eval_binary_unknown_op_returns_false() {
        let info = table();
        let p = PredicateIR::Expr(ExprIR::Binary {
            op: "POW".to_string(),
            left: Box::new(ExprIR::Literal(Value::Integer(2))),
            right: Box::new(ExprIR::Literal(Value::Integer(3))),
        });
        assert!(!p.evaluate(&[], &info));
    }

    #[test]
    fn test_eval_to_value_column() {
        let info = table();
        let row = &[Value::Integer(1), Value::Boolean(true), Value::Text("a".into())];
        assert_eq!(
            PredicateIR::eval_to_value(&ExprIR::Column("id".to_string()), row, &info),
            Value::Integer(1)
        );
        assert_eq!(
            PredicateIR::eval_to_value(&ExprIR::Column("missing".to_string()), row, &info),
            Value::Null
        );
    }

    #[test]
    fn test_eval_to_value_literal() {
        let info = table();
        assert_eq!(
            PredicateIR::eval_to_value(&ExprIR::Literal(Value::Integer(42)), &[], &info),
            Value::Integer(42)
        );
    }

    #[test]
    fn test_eval_to_value_isnull() {
        let info = table();
        let row_null = &[Value::Integer(1), Value::Boolean(true), Value::Null];
        let row_text = &[Value::Integer(1), Value::Boolean(true), Value::Text("a".into())];
        assert_eq!(
            PredicateIR::eval_to_value(
                &ExprIR::IsNull(Box::new(ExprIR::Column("name".to_string()))),
                row_null,
                &info
            ),
            Value::Boolean(true)
        );
        assert_eq!(
            PredicateIR::eval_to_value(
                &ExprIR::IsNull(Box::new(ExprIR::Column("name".to_string()))),
                row_text,
                &info
            ),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_eval_to_value_isnotnull() {
        let info = table();
        let row = &[Value::Integer(1), Value::Boolean(true), Value::Null];
        assert_eq!(
            PredicateIR::eval_to_value(
                &ExprIR::IsNotNull(Box::new(ExprIR::Column("name".to_string()))),
                row,
                &info
            ),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_eval_to_value_unary_not() {
        let info = table();
        assert_eq!(
            PredicateIR::eval_to_value(
                &ExprIR::Unary {
                    op: "NOT".to_string(),
                    expr: Box::new(ExprIR::Literal(Value::Boolean(true))),
                },
                &[],
                &info
            ),
            Value::Boolean(false)
        );
        assert_eq!(
            PredicateIR::eval_to_value(
                &ExprIR::Unary {
                    op: "NOT".to_string(),
                    expr: Box::new(ExprIR::Literal(Value::Boolean(false))),
                },
                &[],
                &info
            ),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_eval_to_value_unary_not_on_non_boolean() {
        let info = table();
        assert_eq!(
            PredicateIR::eval_to_value(
                &ExprIR::Unary {
                    op: "NOT".to_string(),
                    expr: Box::new(ExprIR::Literal(Value::Integer(42))),
                },
                &[],
                &info
            ),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_eval_to_value_unary_other_op() {
        let info = table();
        assert_eq!(
            PredicateIR::eval_to_value(
                &ExprIR::Unary {
                    op: "ABS".to_string(),
                    expr: Box::new(ExprIR::Literal(Value::Boolean(true))),
                },
                &[],
                &info
            ),
            Value::Null
        );
    }

    #[test]
    fn test_eval_to_value_binary_logical() {
        let info = table();
        let and = PredicateIR::eval_to_value(
            &ExprIR::Binary {
                op: "AND".to_string(),
                left: Box::new(ExprIR::Literal(Value::Boolean(true))),
                right: Box::new(ExprIR::Literal(Value::Boolean(false))),
            },
            &[],
            &info,
        );
        assert_eq!(and, Value::Boolean(false));
    }

    #[test]
    fn test_eval_to_value_binary_non_boolean_operands() {
        let info = table();
        let r = PredicateIR::eval_to_value(
            &ExprIR::Binary {
                op: "AND".to_string(),
                left: Box::new(ExprIR::Literal(Value::Integer(1))),
                right: Box::new(ExprIR::Literal(Value::Boolean(true))),
            },
            &[],
            &info,
        );
        assert_eq!(r, Value::Boolean(false));
    }

    #[test]
    fn test_eval_to_value_binary_non_logical_op() {
        let info = table();
        let r = PredicateIR::eval_to_value(
            &ExprIR::Binary {
                op: "=".to_string(),
                left: Box::new(ExprIR::Literal(Value::Integer(1))),
                right: Box::new(ExprIR::Literal(Value::Integer(1))),
            },
            &[],
            &info,
        );
        assert_eq!(r, Value::Null);
    }

    #[test]
    fn test_cmp_values() {
        assert_eq!(PredicateIR::cmp_values(&Value::Integer(1), &Value::Integer(2)), -1);
        assert_eq!(PredicateIR::cmp_values(&Value::Integer(2), &Value::Integer(2)), 0);
        assert_eq!(PredicateIR::cmp_values(&Value::Integer(2), &Value::Integer(1)), 1);
        assert_eq!(PredicateIR::cmp_values(&Value::Float(1.0), &Value::Float(2.0)), -1);
        assert_eq!(PredicateIR::cmp_values(&Value::Text("a".into()), &Value::Text("b".into())), -1);
        assert_eq!(PredicateIR::cmp_values(&Value::Null, &Value::Null), 0);
        assert_eq!(PredicateIR::cmp_values(&Value::Null, &Value::Integer(1)), -1);
        assert_eq!(PredicateIR::cmp_values(&Value::Integer(1), &Value::Null), 1);
    }

    #[test]
    fn test_find_column_index() {
        let info = table();
        assert_eq!(PredicateIR::find_column_index("id", &info), Some(0));
        assert_eq!(PredicateIR::find_column_index("active", &info), Some(1));
        assert_eq!(PredicateIR::find_column_index("missing", &info), None);
    }

    #[test]
    fn test_resolve_column_index() {
        let info = table();
        let row = &[Value::Integer(1), Value::Boolean(true), Value::Null];
        assert_eq!(
            PredicateIR::resolve_column_index(&ExprIR::Column("id".to_string()), row, &info),
            Some(0)
        );
        assert_eq!(
            PredicateIR::resolve_column_index(&ExprIR::Literal(Value::Integer(1)), row, &info),
            None
        );
    }

    #[test]
    fn test_expr_ir_equality() {
        let a = ExprIR::Column("x".to_string());
        let b = ExprIR::Column("x".to_string());
        let c = ExprIR::Column("y".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
