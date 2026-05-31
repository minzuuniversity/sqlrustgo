use crate::engine_utils::*;
use crate::expr_utils::*;
use crate::{ExecutionEngine, ExecutorResult, SqlError, SqlResult, Value};
use sqlrustgo_parser::{AggregateCall, AggregateFunction, Expression, JoinType, SelectStatement};
use sqlrustgo_storage::{StorageEngine, TableInfo};

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    pub(crate) fn execute_select(&self, select: &SelectStatement) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read().unwrap();

        if select.table.is_empty() {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        // Step 1: FROM/JOIN - get initial rows and schema
        let (mut rows, table_info) = if select.join_clause.is_some() {
            self.execute_join(select)?
        } else {
            let rows = storage.scan(&select.table)?;
            let table_info = storage.get_table_info(&select.table)?;
            (rows, table_info)
        };

        // Step 2: WHERE
        if let Some(ref where_expr) = select.where_clause {
            rows.retain(|row| eval_predicate(where_expr, row, &table_info));
        }

        // Step 3: GROUP BY + AGGREGATE
        if !select.aggregates.is_empty() {
            let group_exprs = &select.group_by;
            if group_exprs.is_empty() {
                let mut agg_values =
                    self.compute_aggregates(&select.aggregates, &rows, &table_info)?;

                if let Some(ref having_expr) = select.having {
                    let having_schema = build_aggregate_schema(&[], &select.aggregates)?;
                    if !eval_predicate(having_expr, &agg_values, &having_schema) {
                        return Ok(ExecutorResult::new(vec![], 0));
                    }
                }

                return Ok(ExecutorResult::new(vec![agg_values], 1));
            } else {
                let mut groups: std::collections::HashMap<String, Vec<Vec<Value>>> =
                    std::collections::HashMap::new();
                for row in &rows {
                    let key = group_exprs
                        .iter()
                        .map(|expr| evaluate_expr_to_string(expr, row, &table_info))
                        .collect::<Vec<_>>()
                        .join("\x00");
                    groups.entry(key).or_default().push(row.clone());
                }

                let mut agg_result_rows: Vec<Vec<Value>> = Vec::new();

                for (key, group_rows) in groups.iter() {
                    let key_values: Vec<Value> = key
                        .split('\x00')
                        .map(|s| {
                            if s == "NULL" {
                                Value::Null
                            } else if let Ok(n) = s.parse::<i64>() {
                                Value::Integer(n)
                            } else {
                                Value::Text(s.to_string())
                            }
                        })
                        .collect();
                    let agg_values =
                        self.compute_aggregates(&select.aggregates, group_rows, &table_info)?;
                    let mut combined = key_values;
                    combined.extend(agg_values);
                    agg_result_rows.push(combined);
                }

                if let Some(ref having_expr) = select.having {
                    let having_schema = build_aggregate_schema(group_exprs, &select.aggregates)?;
                    agg_result_rows.retain(|row| eval_predicate(having_expr, row, &having_schema));
                }

                let row_count = agg_result_rows.len();
                return Ok(ExecutorResult::new(agg_result_rows, row_count));
            }
        }

        // Step 4: LIMIT / OFFSET
        let limited_rows = if let Some(limit) = select.limit {
            let offset = select.offset.unwrap_or(0);
            if offset as usize >= rows.len() {
                vec![]
            } else {
                rows.into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect()
            }
        } else {
            rows
        };

        let row_count = limited_rows.len();
        Ok(ExecutorResult::new(limited_rows, row_count))
    }

    fn compute_aggregates(
        &self,
        aggregates: &[AggregateCall],
        rows: &[Vec<Value>],
        table_info: &TableInfo,
    ) -> SqlResult<Vec<Value>> {
        let mut results = Vec::with_capacity(aggregates.len());
        for agg in aggregates {
            let values: Vec<Value> = if let Some(arg) = agg.args.first() {
                rows.iter()
                    .map(|row| evaluate_expression(arg, row, table_info).unwrap_or(Value::Null))
                    .collect()
            } else {
                vec![Value::Integer(rows.len() as i64)]
            };

            let result = match agg.func {
                AggregateFunction::Count => {
                    if agg.args.is_empty() {
                        // COUNT(*) - count all rows
                        Value::Integer(rows.len() as i64)
                    } else {
                        // COUNT(col) - count non-NULL values
                        let non_null_count =
                            values.iter().filter(|v| !matches!(v, Value::Null)).count();
                        Value::Integer(non_null_count as i64)
                    }
                }
                AggregateFunction::Sum => {
                    let int_values: Vec<i64> = values
                        .iter()
                        .filter_map(|v| {
                            if let Value::Integer(n) = v {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .collect();
                    if int_values.is_empty() {
                        Value::Null
                    } else {
                        Value::Integer(int_values.iter().sum())
                    }
                }
                AggregateFunction::Avg => {
                    let sum: i64 = values
                        .iter()
                        .filter_map(|v| {
                            if let Value::Integer(n) = v {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .sum();
                    let count = values
                        .iter()
                        .filter(|v| matches!(v, Value::Integer(_)))
                        .count();
                    if count > 0 {
                        Value::Integer(sum / count as i64)
                    } else {
                        Value::Null
                    }
                }
                AggregateFunction::Min => {
                    let min = values
                        .iter()
                        .filter_map(|v| {
                            if let Value::Integer(n) = v {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .min();
                    min.map(Value::Integer).unwrap_or(Value::Null)
                }
                AggregateFunction::Max => {
                    let max = values
                        .iter()
                        .filter_map(|v| {
                            if let Value::Integer(n) = v {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .max();
                    max.map(Value::Integer).unwrap_or(Value::Null)
                }
            };
            results.push(result);
        }
        Ok(results)
    }

    /// Execute JOIN and return (rows, combined_schema)
    /// This function only generates joined rows, does NOT apply WHERE/AGG/HAVING
    fn execute_join(&self, select: &SelectStatement) -> SqlResult<(Vec<Vec<Value>>, TableInfo)> {
        use sqlrustgo_parser::JoinType as ParserJoinType;
        use std::collections::HashMap;

        let join_clause = select.join_clause.as_ref().unwrap();
        let left_table_name = select.table.clone();
        let right_table_name = join_clause.table.clone();

        let storage = self.storage.read().unwrap();

        // Scan both tables
        let left_rows = storage.scan(&left_table_name)?;
        let right_rows = storage.scan(&right_table_name)?;

        // Get table info for column indices
        let left_table_info = storage.get_table_info(&left_table_name)?;
        let right_table_info = storage.get_table_info(&right_table_name)?;

        // Extract join key column index from ON clause
        // For "t1.id = t2.id", we need to find which column "id" refers to in each table
        let left_key_idx =
            self.find_join_key_index(&join_clause.on_clause, &left_table_info, &select.table)?;
        let right_key_idx =
            self.find_join_key_index(&join_clause.on_clause, &right_table_info, &right_table_name)?;

        // Determine join type
        let join_type = match join_clause.join_type {
            ParserJoinType::Inner => JoinType::Inner,
            ParserJoinType::Left => JoinType::Left,
            ParserJoinType::Right => JoinType::Right,
            ParserJoinType::Full => JoinType::Full,
            ParserJoinType::Cross => JoinType::Cross,
        };

        let left_col_count = left_table_info.columns.len();
        let right_col_count = right_table_info.columns.len();

        let mut matched_results = match join_type {
            JoinType::Inner | JoinType::Left | JoinType::Right | JoinType::Full => {
                // Hash-based matching
                // SQL semantics: NULL = NULL is UNKNOWN (not a match), so skip NULL keys
                let mut right_hash: HashMap<String, Vec<Vec<Value>>> = HashMap::new();
                for right_row in &right_rows {
                    if matches!(right_row[right_key_idx], Value::Null) {
                        // NULL keys can never match in a join
                        continue;
                    }
                    let key = format!("{:?}", right_row[right_key_idx]);
                    right_hash.entry(key).or_default().push(right_row.clone());
                }

                let mut matched: Vec<Vec<Value>> = Vec::new();
                let mut left_matched: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();
                let mut right_matched: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();

                // Match left rows to right
                for (li, left_row) in left_rows.iter().enumerate() {
                    // SQL semantics: NULL keys never match
                    if matches!(left_row[left_key_idx], Value::Null) {
                        // For LEFT JOIN, this row will be added as unmatched later
                        continue;
                    }
                    let key = format!("{:?}", left_row[left_key_idx]);
                    if let Some(right_match_rows) = right_hash.get(&key) {
                        left_matched.insert(li);
                        for right_row in right_match_rows {
                            // Find the original right row index
                            if let Some(ri) = right_rows.iter().position(|r| r == right_row) {
                                right_matched.insert(ri);
                            }
                            let mut combined = left_row.clone();
                            combined.extend(right_row.clone());
                            matched.push(combined);
                        }
                    }
                }

                // For LEFT/RIGHT/FULL, add unmatched rows
                if matches!(join_type, JoinType::Left | JoinType::Full) {
                    for (li, left_row) in left_rows.iter().enumerate() {
                        if !left_matched.contains(&li) {
                            let mut combined = left_row.clone();
                            combined.extend(vec![Value::Null; right_col_count]);
                            matched.push(combined);
                        }
                    }
                }

                if matches!(join_type, JoinType::Right | JoinType::Full) {
                    for (ri, right_row) in right_rows.iter().enumerate() {
                        if !right_matched.contains(&ri) {
                            let mut combined = vec![Value::Null; left_col_count];
                            combined.extend(right_row.clone());
                            matched.push(combined);
                        }
                    }
                }

                matched
            }
            JoinType::Cross => {
                let mut results = Vec::new();
                for left_row in &left_rows {
                    for right_row in &right_rows {
                        let mut combined = left_row.clone();
                        combined.extend(right_row.clone());
                        results.push(combined);
                    }
                }
                results
            }
        };

        let combined_schema =
            build_combined_schema(&left_table_info, &right_table_name, &right_table_info)?;
        Ok((matched_results, combined_schema))
    }

    /// Find the column index for a join key in a table
    /// Handles both simple column names and qualified names (e.g., "t1.id")
    fn find_join_key_index(
        &self,
        expr: &Expression,
        table_info: &TableInfo,
        table_name: &str,
    ) -> SqlResult<usize> {
        match expr {
            Expression::Identifier(name) => {
                // Check if it's a qualified name like "t1.id"
                if let Some((qualifier, col_name)) = name.split_once('.') {
                    // If qualifier matches our table name, use the column name part
                    if qualifier == table_name {
                        table_info
                            .columns
                            .iter()
                            .position(|c| c.name.as_str() == col_name)
                            .ok_or_else(|| {
                                SqlError::ExecutionError(format!(
                                    "Column '{}.{}' not found in {}",
                                    qualifier, col_name, table_name
                                ))
                            })
                    } else {
                        // Qualifier doesn't match this table - column not in this table
                        Err(SqlError::ExecutionError(format!(
                            "Column '{}' not found in {}",
                            name, table_name
                        )))
                    }
                } else {
                    // Simple column name - find its index
                    table_info
                        .columns
                        .iter()
                        .position(|c| c.name.as_str() == name.as_str())
                        .ok_or_else(|| {
                            SqlError::ExecutionError(format!(
                                "Column '{}' not found in {}",
                                name, table_name
                            ))
                        })
                }
            }
            Expression::BinaryOp(left, _, right) => {
                // Try left side first
                let left_result = self.find_join_key_index(left, table_info, table_name);
                if left_result.is_ok() {
                    return left_result;
                }
                // Try right side
                self.find_join_key_index(right, table_info, table_name)
            }
            _ => Err(SqlError::ExecutionError(
                "Unsupported join condition expression".to_string(),
            )),
        }
    }
}
