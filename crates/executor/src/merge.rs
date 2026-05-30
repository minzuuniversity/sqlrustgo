//! MERGE statement executor
//!
//! Implements the SQL MERGE statement which combines INSERT, UPDATE, and DELETE
//! operations in a single statement based on a condition.
//!
//! ## VTU Enforcement
//! ALL DML operations go through ExecutionEngine::execute() - NO direct storage access.

use sqlrustgo_parser::{Expression, MergeStatement};
use sqlrustgo_storage::{StorageEngine, TableInfo};
use sqlrustgo_types::{SqlError, SqlResult, Value};
use std::sync::{Arc, Mutex, RwLock};

use crate::execution::{ExecutionEngine, QueryContext};
use crate::executor::ExecutorResult;

/// MERGE executor that handles SQL MERGE statements
/// MERGE executor that handles SQL MERGE statements
/// ALL DML operations go through ExecutionEngine (VTU enforced)
/// Storage is only used for READ operations (scan, get_table_info)
pub struct MergeExecutor {
    storage: Arc<RwLock<dyn StorageEngine>>,
    engine: Arc<Mutex<dyn ExecutionEngine>>,
}

impl MergeExecutor {
    /// Create a new MergeExecutor with VTU enforcement
    pub fn new(
        storage: Arc<RwLock<dyn StorageEngine>>,
        engine: Arc<Mutex<dyn ExecutionEngine>>,
    ) -> Self {
        Self { storage, engine }
    }

    /// Execute a MERGE statement (VTU path ONLY)
    pub fn execute_merge(&self, merge: &MergeStatement) -> SqlResult<ExecutorResult> {
        let target_table = &merge.target_table;
        let source_table = &merge.source_table;

        let source_rows = {
            let storage = self.storage.read().unwrap();
            storage.scan(source_table)?
        };

        let target_rows = {
            let storage = self.storage.read().unwrap();
            storage.scan(target_table)?
        };

        let target_table_info = {
            let storage = self.storage.read().unwrap();
            storage.get_table_info(target_table)?.clone()
        };

        let source_table_info = {
            let storage = self.storage.read().unwrap();
            storage.get_table_info(source_table)?.clone()
        };

        let target_pk_idx = target_table_info.columns.iter().position(|c| c.primary_key);

        let mut matched_count: usize = 0;
        let mut inserted_count: usize = 0;

        for source_row in source_rows.iter() {
            let matching_target_idx = target_rows.iter().position(|target_row| {
                self.eval_merge_condition(
                    &merge.on_condition,
                    source_row,
                    target_row,
                    &source_table_info,
                    &target_table_info,
                )
            });

            if let Some(idx) = matching_target_idx {
                if let Some(ref clause) = merge.matched_clause {
                    let target_row = &target_rows[idx];
                    let updates: Vec<(usize, Value)> = clause
                        .update_columns
                        .iter()
                        .zip(clause.update_values.iter())
                        .filter_map(|(col, val)| {
                            find_column_index(col, &target_table_info).map(|col_idx| {
                                let evaluated = self.eval_merge_value(
                                    val,
                                    source_row,
                                    target_row,
                                    &source_table_info,
                                    &target_table_info,
                                );
                                (col_idx, evaluated)
                            })
                        })
                        .collect();

                    let filter = target_pk_idx.and_then(|pk_idx| target_row.get(pk_idx).cloned());
                    // VTU path: execute UPDATE through ExecutionEngine
                    let update_sql = self.build_update_sql(target_table, &target_table_info, &updates, filter.as_slice());
                    let mut ctx = QueryContext::new(update_sql);
                    self.engine.lock().unwrap().execute(&mut ctx)?;
                    matched_count += 1;
                }
            } else if let Some(ref clause) = merge.not_matched_clause {
                let values: Vec<Value> = clause
                    .insert_values
                    .iter()
                    .map(|val| {
                        self.eval_merge_value(
                            val,
                            source_row,
                            &[],
                            &source_table_info,
                            &target_table_info,
                        )
                    })
                    .collect();

                // VTU path: execute INSERT through ExecutionEngine
                let insert_sql = self.build_insert_sql(target_table, &target_table_info, &values);
                let mut ctx = QueryContext::new(insert_sql);
                self.engine.lock().unwrap().execute(&mut ctx)?;
                inserted_count += 1;
            }
        }

        Ok(ExecutorResult::new(vec![], matched_count + inserted_count))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn eval_merge_condition(
        &self,
        condition: &Expression,
        source_row: &[Value],
        target_row: &[Value],
        source_table_info: &TableInfo,
        target_table_info: &TableInfo,
    ) -> bool {
        match condition {
            Expression::BinaryOp(left, op, right) if op.to_uppercase() == "AND" => {
                self.eval_merge_condition(
                    left,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                ) && self.eval_merge_condition(
                    right,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                )
            }
            Expression::BinaryOp(left, op, right) if op.to_uppercase() == "OR" => {
                self.eval_merge_condition(
                    left,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                ) || self.eval_merge_condition(
                    right,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                )
            }
            Expression::BinaryOp(left, op, right) => {
                let left_val = self.eval_merge_expr(
                    left,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                );
                let right_val = self.eval_merge_expr(
                    right,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                );
                sql_compare(op, &left_val, &right_val)
            }
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn eval_merge_expr(
        &self,
        expr: &Expression,
        source_row: &[Value],
        target_row: &[Value],
        source_table_info: &TableInfo,
        target_table_info: &TableInfo,
    ) -> Value {
        match expr {
            Expression::Literal(_) => expression_to_value(expr),
            Expression::Identifier(name) => {
                if let Some((qualifier, col)) = name.split_once('.') {
                    let qualifier_lower = qualifier.to_lowercase();
                    if qualifier_lower == source_table_info.name.to_lowercase()
                        || qualifier_lower == "source"
                    {
                        if let Some(idx) = source_table_info
                            .columns
                            .iter()
                            .position(|c| c.name.eq_ignore_ascii_case(col))
                        {
                            return source_row.get(idx).cloned().unwrap_or(Value::Null);
                        }
                    }
                    if qualifier_lower == target_table_info.name.to_lowercase()
                        || qualifier_lower == "target"
                    {
                        if let Some(idx) = target_table_info
                            .columns
                            .iter()
                            .position(|c| c.name.eq_ignore_ascii_case(col))
                        {
                            return target_row.get(idx).cloned().unwrap_or(Value::Null);
                        }
                    }
                    Value::Null
                } else if let Some(idx) = target_table_info
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(name))
                {
                    target_row.get(idx).cloned().unwrap_or(Value::Null)
                } else if let Some(idx) = source_table_info
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(name))
                {
                    source_row.get(idx).cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            Expression::BinaryOp(left, op, right) => {
                let l = self.eval_merge_expr(
                    left,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                );
                let r = self.eval_merge_expr(
                    right,
                    source_row,
                    target_row,
                    source_table_info,
                    target_table_info,
                );
                evaluate_binary_op(&l, &r, op)
            }
            _ => Value::Null,
        }
    }

    fn eval_merge_value(
        &self,
        expr: &Expression,
        source_row: &[Value],
        target_row: &[Value],
        source_table_info: &TableInfo,
        target_table_info: &TableInfo,
    ) -> Value {
        self.eval_merge_expr(
            expr,
            source_row,
            target_row,
            source_table_info,
            target_table_info,
        )
    }
}

/// Compare two values for a binary operation
fn evaluate_binary_op(left: &Value, right: &Value, op: &str) -> Value {
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
fn compare_values(left: &Value, right: &Value) -> i32 {
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

/// SQL comparison operator
fn sql_compare(op: &str, left: &Value, right: &Value) -> bool {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return false;
    }

    match op.to_uppercase().as_str() {
        "=" | "==" => left == right,
        "!=" | "<>" => left != right,
        ">" => compare_values(left, right) > 0,
        ">=" => compare_values(left, right) >= 0,
        "<" => compare_values(left, right) < 0,
        "<=" => compare_values(left, right) <= 0,
        _ => false,
    }
}

/// Find column index in table info
fn find_column_index(col_name: &str, table_info: &TableInfo) -> Option<usize> {
    if let Some((_qualifier, col)) = col_name.split_once('.') {
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

/// Convert an expression to a Value
fn expression_to_value(expr: &Expression) -> Value {
    match expr {
        Expression::Literal(s) => {
            let s = s.trim();
            if s.eq_ignore_ascii_case("NULL") {
                Value::Null
            } else if s.eq_ignore_ascii_case("TRUE") {
                Value::Boolean(true)
            } else if s.eq_ignore_ascii_case("FALSE") {
                Value::Boolean(false)
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
        Expression::Identifier(name) => Value::Text(name.clone()),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_values_integer() {
        assert_eq!(compare_values(&Value::Integer(1), &Value::Integer(1)), 0);
        assert_eq!(compare_values(&Value::Integer(1), &Value::Integer(2)), -1);
        assert_eq!(compare_values(&Value::Integer(2), &Value::Integer(1)), 1);
    }

    #[test]
    fn test_compare_values_text() {
        assert_eq!(
            compare_values(&Value::Text("a".to_string()), &Value::Text("a".to_string())),
            0
        );
        assert_eq!(
            compare_values(&Value::Text("a".to_string()), &Value::Text("b".to_string())),
            -1
        );
    }

    #[test]
    fn test_compare_values_null() {
        assert_eq!(compare_values(&Value::Null, &Value::Null), 0);
        assert_eq!(compare_values(&Value::Null, &Value::Integer(1)), -1);
        assert_eq!(compare_values(&Value::Integer(1), &Value::Null), 1);
    }

    #[test]
    fn test_evaluate_binary_op() {
        assert_eq!(
            evaluate_binary_op(&Value::Integer(1), &Value::Integer(1), "="),
            Value::Boolean(true)
        );
        assert_eq!(
            evaluate_binary_op(&Value::Integer(1), &Value::Integer(2), "<"),
            Value::Boolean(true)
        );
        assert_eq!(
            evaluate_binary_op(&Value::Boolean(true), &Value::Boolean(true), "AND"),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_expression_to_value() {
        assert_eq!(
            expression_to_value(&Expression::Literal("42".to_string())),
            Value::Integer(42)
        );
        assert_eq!(
            expression_to_value(&Expression::Literal("'hello'".to_string())),
            Value::Text("hello".to_string())
        );
        assert_eq!(
            expression_to_value(&Expression::Literal("NULL".to_string())),
            Value::Null
        );
        assert_eq!(
            expression_to_value(&Expression::Literal("TRUE".to_string())),
            Value::Boolean(true)
        );
        assert_eq!(
            expression_to_value(&Expression::Literal("FALSE".to_string())),
            Value::Boolean(false)
        );
        assert_eq!(
            expression_to_value(&Expression::Identifier("col1".to_string())),
            Value::Text("col1".to_string())
        );
        assert_eq!(
            expression_to_value(&Expression::BinaryOp(
                Box::new(Expression::Literal("1".to_string())),
                "+".to_string(),
                Box::new(Expression::Literal("2".to_string()))
            )),
            Value::Null
        );
    }

    #[test]
    fn test_expression_to_value_float() {
        assert_eq!(
            expression_to_value(&Expression::Literal("3.14".to_string())),
            Value::Float(3.14)
        );
        assert_eq!(
            expression_to_value(&Expression::Literal("'test'".to_string())),
            Value::Text("test".to_string())
        );
        // Unsupported expression returns Null
        assert_eq!(
            expression_to_value(&Expression::UnaryOp(
                "NOT".to_string(),
                Box::new(Expression::Literal("TRUE".to_string()))
            )),
            Value::Null
        );
    }

    #[test]
    fn test_find_column_index() {
        let table_info = sqlrustgo_storage::TableInfo {
            name: "test".to_string(),
            columns: vec![
                sqlrustgo_storage::ColumnDefinition {
                    name: "id".to_string(),
                    ..Default::default()
                },
                sqlrustgo_storage::ColumnDefinition {
                    name: "name".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(find_column_index("id", &table_info), Some(0));
        assert_eq!(find_column_index("name", &table_info), Some(1));
        assert_eq!(find_column_index("nonexistent", &table_info), None);
        assert_eq!(find_column_index("table.id", &table_info), Some(0));
        assert_eq!(find_column_index("table.nonexistent", &table_info), None);
    }

    #[test]
    fn test_evaluate_binary_op_comparisons() {
        // == operator
        assert_eq!(
            evaluate_binary_op(&Value::Integer(5), &Value::Integer(5), "=="),
            Value::Boolean(true)
        );
        // IS operator
        assert_eq!(
            evaluate_binary_op(&Value::Integer(5), &Value::Integer(5), "IS"),
            Value::Boolean(true)
        );
        // <> operator
        assert_eq!(
            evaluate_binary_op(&Value::Integer(5), &Value::Integer(3), "<>"),
            Value::Boolean(true)
        );
        // >= operator
        assert_eq!(
            evaluate_binary_op(&Value::Integer(5), &Value::Integer(3), ">="),
            Value::Boolean(true)
        );
        // <= operator
        assert_eq!(
            evaluate_binary_op(&Value::Integer(3), &Value::Integer(5), "<="),
            Value::Boolean(true)
        );
        // AND operator
        assert_eq!(
            evaluate_binary_op(&Value::Boolean(true), &Value::Boolean(true), "AND"),
            Value::Boolean(true)
        );
        assert_eq!(
            evaluate_binary_op(&Value::Boolean(true), &Value::Boolean(false), "AND"),
            Value::Boolean(false)
        );
        // AND with non-boolean
        assert_eq!(
            evaluate_binary_op(&Value::Integer(1), &Value::Integer(1), "AND"),
            Value::Boolean(false)
        );
        // OR operator
        assert_eq!(
            evaluate_binary_op(&Value::Boolean(true), &Value::Boolean(false), "OR"),
            Value::Boolean(true)
        );
        assert_eq!(
            evaluate_binary_op(&Value::Boolean(false), &Value::Boolean(false), "OR"),
            Value::Boolean(false)
        );
        // OR with non-boolean
        assert_eq!(
            evaluate_binary_op(&Value::Integer(1), &Value::Integer(1), "OR"),
            Value::Boolean(false)
        );
        // Unknown operator
        assert_eq!(
            evaluate_binary_op(&Value::Integer(1), &Value::Integer(1), "LIKE"),
            Value::Null
        );
    }

    #[test]
    fn test_compare_values_float_mixed() {
        assert_eq!(compare_values(&Value::Null, &Value::Float(1.0)), -1);
        assert_eq!(compare_values(&Value::Float(1.0), &Value::Null), 1);
    }

    #[test]
    fn test_sql_compare_edge_cases() {
        // != operator
        assert!(sql_compare("!=", &Value::Integer(1), &Value::Integer(2)));
        assert!(!sql_compare("!=", &Value::Integer(1), &Value::Integer(1)));
        // >= with matching values
        assert!(sql_compare(">=", &Value::Integer(5), &Value::Integer(3)));
        assert!(sql_compare(">=", &Value::Integer(5), &Value::Integer(5)));
        assert!(!sql_compare(">=", &Value::Integer(3), &Value::Integer(5)));
        // <= with matching values
        assert!(sql_compare("<=", &Value::Integer(3), &Value::Integer(5)));
        assert!(sql_compare("<=", &Value::Integer(5), &Value::Integer(5)));
        assert!(!sql_compare("<=", &Value::Integer(6), &Value::Integer(5)));
        // Unknown operator
        assert!(!sql_compare("LIKE", &Value::Integer(1), &Value::Integer(1)));
        // Null comparisons
        assert!(!sql_compare("=", &Value::Integer(1), &Value::Null));
        assert!(!sql_compare("<", &Value::Null, &Value::Null));
    }

    /// Build an INSERT SQL statement from values
    fn build_insert_sql(&self, table: &str, table_info: &TableInfo, values: &[Value]) -> String {
        let col_names: Vec<String> = table_info
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();

        let values_str = values
            .iter()
            .map(|v| self.value_to_sql(v))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            col_names.join(", "),
            values_str
        )
    }

    /// Build an UPDATE SQL statement with filters
    fn build_update_sql(
        &self,
        table: &str,
        table_info: &TableInfo,
        updates: &[(usize, Value)],
        filter: &[Value],
    ) -> String {
        let set_clauses = updates
            .iter()
            .filter_map(|(col_idx, val)| {
                table_info.columns.get(*col_idx).map(|col| {
                    format!("{} = {}", col.name, self.value_to_sql(val))
                })
            })
            .collect::<Vec<_>>()
            .join(", ");

        let where_clause = if !filter.is_empty() {
            let pk_col = table_info
                .columns
                .iter()
                .find(|c| c.primary_key)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| table_info.columns.first().map(|c| c.name.clone()).unwrap_or_default());

            let filter_str = filter
                .iter()
                .map(|v| self.value_to_sql(v))
                .collect::<Vec<_>>()
                .join(", ");

            format!(" WHERE {} IN ({})", pk_col, filter_str)
        } else {
            String::new()
        };

        format!("UPDATE {} SET {}{}", table, set_clauses, where_clause)
    }

    /// Convert a Value to SQL literal string
    fn value_to_sql(&self, value: &Value) -> String {
        match value {
            Value::Null => "NULL".to_string(),
            Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Text(s) => format!("'{}'", s.replace("'", "''")),
            Value::Blob(b) => format!("X'{:?}'", b),
        }
    }

    #[test]
    fn test_sql_compare_operators() {
        // Equality
        assert!(sql_compare("=", &Value::Integer(1), &Value::Integer(1)));
        assert!(!sql_compare("=", &Value::Integer(1), &Value::Integer(2)));
        // Not equal
        assert!(sql_compare("!=", &Value::Integer(1), &Value::Integer(2)));
        assert!(sql_compare("<>", &Value::Integer(1), &Value::Integer(2)));
        // Greater/less than
        assert!(sql_compare(">", &Value::Integer(2), &Value::Integer(1)));
        assert!(sql_compare("<", &Value::Integer(1), &Value::Integer(2)));
        assert!(sql_compare(">=", &Value::Integer(2), &Value::Integer(2)));
        assert!(sql_compare("<=", &Value::Integer(2), &Value::Integer(2)));
        // Null handling
        assert!(!sql_compare("=", &Value::Null, &Value::Integer(1)));
        assert!(!sql_compare(">", &Value::Null, &Value::Integer(1)));
    }

    #[test]
    fn test_evaluate_binary_op_or() {
        assert_eq!(
            evaluate_binary_op(&Value::Boolean(false), &Value::Boolean(true), "OR"),
            Value::Boolean(true)
        );
        assert_eq!(
            evaluate_binary_op(&Value::Boolean(false), &Value::Boolean(false), "OR"),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_evaluate_binary_op_string_eq() {
        assert_eq!(
            evaluate_binary_op(
                &Value::Text("a".to_string()),
                &Value::Text("a".to_string()),
                "="
            ),
            Value::Boolean(true)
        );
        assert_eq!(
            evaluate_binary_op(
                &Value::Text("a".to_string()),
                &Value::Text("b".to_string()),
                "!="
            ),
            Value::Boolean(true)
        );
    }
}
