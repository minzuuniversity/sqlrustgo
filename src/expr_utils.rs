//! Expression utility functions extracted from execution_engine.rs.
//!
//! Pure helper functions for evaluating SQL expressions against rows.
//! These functions do not depend on the `ExecutionEngine` struct and can be
//! used independently by any module needing expression evaluation.

use sqlrustgo_parser::Expression;
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;

pub fn expression_to_string(expr: &sqlrustgo_parser::Expression) -> String {
    match expr {
        sqlrustgo_parser::Expression::Literal(s) => s.clone(),
        sqlrustgo_parser::Expression::Identifier(name) => name.clone(),
        sqlrustgo_parser::Expression::BinaryOp(left, op, right) => {
            format!(
                "({} {} {})",
                expression_to_string(left),
                op,
                expression_to_string(right)
            )
        }
        sqlrustgo_parser::Expression::IsNull(inner) => {
            format!("{} IS NULL", expression_to_string(inner))
        }
        sqlrustgo_parser::Expression::IsNotNull(inner) => {
            format!("{} IS NOT NULL", expression_to_string(inner))
        }
        sqlrustgo_parser::Expression::Aggregate(agg) => match agg.func {
            sqlrustgo_parser::AggregateFunction::Count => {
                if agg.args.is_empty() {
                    "COUNT(*)".to_string()
                } else {
                    format!(
                        "COUNT({})",
                        agg.args
                            .iter()
                            .map(expression_to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            sqlrustgo_parser::AggregateFunction::Sum => {
                format!(
                    "SUM({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::Avg => {
                format!(
                    "AVG({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::Min => {
                format!(
                    "MIN({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::Max => {
                format!(
                    "MAX({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        },
        _ => "?".to_string(),
    }
}

/// Convert a parser Expression to a Value (simple literal evaluation)
pub fn expression_to_value(expr: &sqlrustgo_parser::Expression) -> Value {
    match expr {
        sqlrustgo_parser::Expression::Literal(s) => {
            let s = s.trim();
            if s.eq_ignore_ascii_case("NULL") {
                Value::Null
            } else if let Ok(n) = s.parse::<i64>() {
                Value::Integer(n)
            } else if let Ok(f) = s.parse::<f64>() {
                Value::Float(f)
            } else if s.starts_with('\'') && s.ends_with('\'') {
                Value::Text(s[1..s.len() - 1].to_string())
            } else {
                Value::Text(s.to_string())
            }
        }
        sqlrustgo_parser::Expression::Identifier(name) => Value::Text(name.clone()),
        _ => Value::Null,
    }
}

/// Convert a string argument to a Value (for CALL arguments)
pub fn expression_to_value_from_string(s: &str) -> Value {
    let s = s.trim();
    if s.eq_ignore_ascii_case("NULL") {
        Value::Null
    } else if let Ok(n) = s.parse::<i64>() {
        Value::Integer(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::Float(f)
    } else if s.starts_with('\'') && s.ends_with('\'') {
        Value::Text(s[1..s.len() - 1].to_string())
    } else {
        Value::Text(s.to_string())
    }
}

/// Evaluate an expression and return a Value
pub fn evaluate_expression(
    expr: &Expression,
    row: &[Value],
    table_info: &TableInfo,
) -> Result<Value, String> {
    match expr {
        Expression::Literal(_) => Ok(expression_to_value(expr)),
        Expression::Identifier(name) => {
            if let Some(col_idx) = find_column_index(name, table_info) {
                Ok(row.get(col_idx).cloned().unwrap_or(Value::Null))
            } else {
                Ok(expression_to_value(expr))
            }
        }
        Expression::BinaryOp(left, op, right) => {
            let left_val = evaluate_expression(left, row, table_info).unwrap_or(Value::Null);
            let right_val = evaluate_expression(right, row, table_info).unwrap_or(Value::Null);
            Ok(evaluate_binary_op(&left_val, &right_val, op))
        }
        Expression::IsNull(inner) => {
            let val = evaluate_expression(inner, row, table_info)?;
            Ok(Value::Boolean(matches!(val, Value::Null)))
        }
        Expression::Aggregate(agg) => {
            let agg_name = expression_to_string(&Expression::Aggregate(agg.clone()));
            if let Some(col_idx) = find_column_index(&agg_name, table_info) {
                Ok(row.get(col_idx).cloned().unwrap_or(Value::Null))
            } else {
                Err(format!("Aggregate not found in schema: {}", agg_name))
            }
        }
        _ => Ok(Value::Null),
    }
}

/// Evaluate a binary operation and return a boolean Value
pub fn evaluate_binary_op(left: &Value, right: &Value, op: &str) -> Value {
    match op.to_uppercase().as_str() {
        "=" | "==" | "IS" => Value::Boolean(left == right),
        "!=" | "<>" => Value::Boolean(left != right),
        ">" => Value::Boolean(compare_values(left, right) > 0),
        ">=" => Value::Boolean(compare_values(left, right) >= 0),
        "<" => Value::Boolean(compare_values(left, right) < 0),
        "<=" => Value::Boolean(compare_values(left, right) <= 0),
        "AND" | "&&" => {
            if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                Value::Boolean(*l && *r)
            } else {
                Value::Boolean(false)
            }
        }
        "OR" | "||" => {
            if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                Value::Boolean(*l || *r)
            } else {
                Value::Boolean(false)
            }
        }
        _ => Value::Null,
    }
}

/// Compare two values and return -1, 0, or 1
pub fn compare_values(left: &Value, right: &Value) -> i32 {
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

/// Evaluate expression to string (for GROUP BY key)
pub fn evaluate_expr_to_string(expr: &Expression, row: &[Value], table_info: &TableInfo) -> String {
    let val = evaluate_expression(expr, row, table_info).unwrap_or(Value::Null);
    match val {
        Value::Null => "NULL".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(s) => s,
        Value::Boolean(b) => b.to_string(),
        _ => "?".to_string(),
    }
}

/// Find the index of a column in the table info
/// For JOIN queries with combined tables, handles qualified names like "t2.id"
/// by routing to the correct portion of the combined schema.
/// Combined table naming: left_table.col, right_table.col
pub(crate) fn find_column_index(col_name: &str, table_info: &TableInfo) -> Option<usize> {
    if let Some((_qualifier, col)) = col_name.split_once('.') {
        for (i, c) in table_info.columns.iter().enumerate() {
            if c.name.eq_ignore_ascii_case(col_name) {
                return Some(i);
            }
        }
        table_info
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col))
    } else {
        table_info
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col_name))
    }
}
