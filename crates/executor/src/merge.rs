//! MERGE statement executor
//!
//! Implements the SQL MERGE statement which combines INSERT, UPDATE, and DELETE
//! operations in a single statement based on a condition.
//!
//! ## VTU Enforcement
//! ALL DML operations go through ExecutionEngine::execute() - NO direct storage access.

use sqlrustgo_planner::{Expr, MergeStatement, Operator};
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
                                let evaluated = self.eval_merge_expr(
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
                        self.eval_merge_expr(
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
        condition: &Expr,
        source_row: &[Value],
        target_row: &[Value],
        source_table_info: &TableInfo,
        target_table_info: &TableInfo,
    ) -> bool {
        match condition {
            Expr::BinaryExpr {
                left,
                op: Operator::And,
                right,
            } => {
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
            Expr::BinaryExpr {
                left,
                op: Operator::Or,
                right,
            } => {
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
            Expr::BinaryExpr { left, op, right } => {
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
                op_compare(op, &left_val, &right_val)
            }
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn eval_merge_expr(
        &self,
        expr: &Expr,
        source_row: &[Value],
        target_row: &[Value],
        source_table_info: &TableInfo,
        target_table_info: &TableInfo,
    ) -> Value {
        match expr {
            Expr::Literal(v) => v.clone(),
            Expr::Column(col) => {
                if let Some(ref qualifier) = col.relation {
                    let qualifier_lower = qualifier.to_lowercase();
                    let col_name = &col.name;
                    if qualifier_lower == source_table_info.name.to_lowercase()
                        || qualifier_lower == "source"
                    {
                        if let Some(idx) = source_table_info
                            .columns
                            .iter()
                            .position(|c| c.name.eq_ignore_ascii_case(col_name))
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
                            .position(|c| c.name.eq_ignore_ascii_case(col_name))
                        {
                            return target_row.get(idx).cloned().unwrap_or(Value::Null);
                        }
                    }
                    Value::Null
                } else if let Some(idx) = target_table_info
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(&col.name))
                {
                    target_row.get(idx).cloned().unwrap_or(Value::Null)
                } else if let Some(idx) = source_table_info
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(&col.name))
                {
                    source_row.get(idx).cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            Expr::BinaryExpr { left, op, right } => {
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
                eval_binary_op(&l, &r, op)
            }
            _ => Value::Null,
        }
    }

    fn build_insert_sql(&self, table: &str, table_info: &TableInfo, values: &[Value]) -> String {
        let col_names: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();
        let values_str = values.iter().map(|v| self.value_to_sql(v)).collect::<Vec<_>>().join(", ");
        format!("INSERT INTO {} ({}) VALUES ({})", table, col_names.join(", "), values_str)
    }

    fn build_update_sql(&self, table: &str, table_info: &TableInfo, updates: &[(usize, Value)], filter: &[Value]) -> String {
        let set_clauses = updates.iter().filter_map(|(col_idx, val)| {
            table_info.columns.get(*col_idx).map(|col| format!("{} = {}", col.name, self.value_to_sql(val)))
        }).collect::<Vec<_>>().join(", ");
        let where_clause = if !filter.is_empty() {
            let pk_col = table_info.columns.iter().find(|c| c.primary_key).map(|c| c.name.clone())
                .unwrap_or_else(|| table_info.columns.first().map(|c| c.name.clone()).unwrap_or_default());
            let conditions = filter.iter().enumerate().map(|(i, v)| format!("{} = {}", table_info.columns.get(i).map(|c| c.name.as_str()).unwrap_or("id"), self.value_to_sql(v))).collect::<Vec<_>>().join(" AND ");
            format!(" WHERE {} = {} AND {}", pk_col, filter.first().map(|v| self.value_to_sql(v)).unwrap_or_default(), conditions)
        } else { String::new() };
        format!("UPDATE {} SET {}{}", table, set_clauses, where_clause)
    }

    fn value_to_sql(&self, val: &Value) -> String {
        match val {
            Value::Null => "NULL".to_string(),
            Value::Integer(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Text(s) => format!("'{}'", s.replace('\'', "''")),
            Value::Boolean(true) => "TRUE".to_string(),
            Value::Boolean(false) => "FALSE".to_string(),
            Value::Blob(_) => "NULL".to_string(),
        }
    }
}

/// Compare two values for a binary operation
fn eval_binary_op(left: &Value, right: &Value, op: &Operator) -> Value {
    match op {
        Operator::Eq | Operator::NotEq => {
            if matches!(left, Value::Null) || matches!(right, Value::Null) {
                return Value::Boolean(false);
            }
            Value::Boolean(match op {
                Operator::Eq => left == right,
                Operator::NotEq => left != right,
                _ => unreachable!(),
            })
        }
        Operator::Gt => Value::Boolean(compare_values(left, right) > 0),
        Operator::GtEq => Value::Boolean(compare_values(left, right) >= 0),
        Operator::Lt => Value::Boolean(compare_values(left, right) < 0),
        Operator::LtEq => Value::Boolean(compare_values(left, right) <= 0),
        Operator::And => {
            if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                Value::Boolean(*l && *r)
            } else {
                Value::Boolean(false)
            }
        }
        Operator::Or => {
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

/// SQL comparison operator using planner Operator enum
fn op_compare(op: &Operator, left: &Value, right: &Value) -> bool {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return false;
    }

    match op {
        Operator::Eq => left == right,
        Operator::NotEq => left != right,
        Operator::Gt => compare_values(left, right) > 0,
        Operator::GtEq => compare_values(left, right) >= 0,
        Operator::Lt => compare_values(left, right) < 0,
        Operator::LtEq => compare_values(left, right) <= 0,
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
    fn test_eval_binary_op() {
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Integer(1), &Operator::Eq),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Integer(2), &Operator::Lt),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(&Value::Boolean(true), &Value::Boolean(true), &Operator::And),
            Value::Boolean(true)
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
    fn test_eval_binary_op_comparisons() {
        // Eq operator
        assert_eq!(
            eval_binary_op(&Value::Integer(5), &Value::Integer(5), &Operator::Eq),
            Value::Boolean(true)
        );
        // NotEq operator
        assert_eq!(
            eval_binary_op(&Value::Integer(5), &Value::Integer(3), &Operator::NotEq),
            Value::Boolean(true)
        );
        // GtEq operator
        assert_eq!(
            eval_binary_op(&Value::Integer(5), &Value::Integer(3), &Operator::GtEq),
            Value::Boolean(true)
        );
        // LtEq operator
        assert_eq!(
            eval_binary_op(&Value::Integer(3), &Value::Integer(5), &Operator::LtEq),
            Value::Boolean(true)
        );
        // And operator
        assert_eq!(
            eval_binary_op(&Value::Boolean(true), &Value::Boolean(true), &Operator::And),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(
                &Value::Boolean(true),
                &Value::Boolean(false),
                &Operator::And
            ),
            Value::Boolean(false)
        );
        // And with non-boolean
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Integer(1), &Operator::And),
            Value::Boolean(false)
        );
        // Or operator
        assert_eq!(
            eval_binary_op(&Value::Boolean(true), &Value::Boolean(false), &Operator::Or),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(
                &Value::Boolean(false),
                &Value::Boolean(false),
                &Operator::Or
            ),
            Value::Boolean(false)
        );
        // Or with non-boolean
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Integer(1), &Operator::Or),
            Value::Boolean(false)
        );
        // Unknown operator
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Integer(1), &Operator::Like),
            Value::Null
        );
        // Null comparisons with Eq
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Null, &Operator::Eq),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_compare_values_float_mixed() {
        assert_eq!(compare_values(&Value::Null, &Value::Float(1.0)), -1);
        assert_eq!(compare_values(&Value::Float(1.0), &Value::Null), 1);
    }

    #[test]
    fn test_op_compare_edge_cases() {
        // NotEq operator
        assert!(op_compare(
            &Operator::NotEq,
            &Value::Integer(1),
            &Value::Integer(2)
        ));
        assert!(!op_compare(
            &Operator::NotEq,
            &Value::Integer(1),
            &Value::Integer(1)
        ));
        // GtEq with matching values
        assert!(op_compare(
            &Operator::GtEq,
            &Value::Integer(5),
            &Value::Integer(3)
        ));
        assert!(op_compare(
            &Operator::GtEq,
            &Value::Integer(5),
            &Value::Integer(5)
        ));
        assert!(!op_compare(
            &Operator::GtEq,
            &Value::Integer(3),
            &Value::Integer(5)
        ));
        // LtEq with matching values
        assert!(op_compare(
            &Operator::LtEq,
            &Value::Integer(3),
            &Value::Integer(5)
        ));
        assert!(op_compare(
            &Operator::LtEq,
            &Value::Integer(5),
            &Value::Integer(5)
        ));
        assert!(!op_compare(
            &Operator::LtEq,
            &Value::Integer(6),
            &Value::Integer(5)
        ));
        // Unknown operator
        assert!(!op_compare(
            &Operator::Like,
            &Value::Integer(1),
            &Value::Integer(1)
        ));
        // Null comparisons
        assert!(!op_compare(&Operator::Eq, &Value::Integer(1), &Value::Null));
        assert!(!op_compare(&Operator::Lt, &Value::Null, &Value::Null));
    }


    #[test]
    fn test_eval_binary_op_or() {
        assert_eq!(
            eval_binary_op(&Value::Boolean(false), &Value::Boolean(true), &Operator::Or),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(
                &Value::Boolean(false),
                &Value::Boolean(false),
                &Operator::Or
            ),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_eval_binary_op_string_eq() {
        assert_eq!(
            eval_binary_op(
                &Value::Text("a".to_string()),
                &Value::Text("a".to_string()),
                &Operator::Eq
            ),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(
                &Value::Text("a".to_string()),
                &Value::Text("b".to_string()),
                &Operator::NotEq
            ),
            Value::Boolean(true)
        );
    }
}
