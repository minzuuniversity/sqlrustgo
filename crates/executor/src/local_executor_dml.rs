//! LocalExecutorDML - DML execution engine with mutable storage access
//!
//! This executor handles INSERT/UPDATE/DELETE operations that require
//! mutable access to the storage engine.

use sqlrustgo_planner::{Expr, InsertExec, UpdateExec, DeleteExec};
use sqlrustgo_storage::{Record, StorageEngine, TableInfo};
use sqlrustgo_types::{SqlResult, Value};
use std::sync::Arc;
use std::sync::RwLock;

use crate::executor::ExecutorResult;

/// DML Executor that supports mutable storage operations
pub struct LocalExecutorDML<'a> {
    storage: &'a mut dyn StorageEngine,
}

impl<'a> LocalExecutorDML<'a> {
    /// Create a new DML executor with the given storage
    pub fn new(storage: &'a mut dyn StorageEngine) -> Self {
        Self { storage }
    }

    /// Execute an INSERT statement
    pub fn execute_insert(&mut self, plan: &InsertExec) -> SqlResult<ExecutorResult> {
        let table_name = plan.table_name();
        let columns = plan.columns();
        let values = plan.values();

        // Get table info for column mapping and AUTO_INCREMENT handling
        let table_info = {
            let storage = self.storage.get_table_info(table_name)?;
            storage.clone()
        };

        // Determine which columns are specified
        let specified_cols: Vec<usize> = if columns.is_empty() {
            (0..table_info.columns.len()).collect()
        } else {
            table_info
                .columns
                .iter()
                .enumerate()
                .filter(|(_, c)| columns.contains(&c.name))
                .map(|(i, _)| i)
                .collect()
        };

        // Find AUTO_INCREMENT columns
        let auto_increment_cols: Vec<usize> = table_info
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.auto_increment)
            .map(|(i, _)| i)
            .collect();

        // Convert expressions to records
        let mut records: Vec<Record> = Vec::with_capacity(values.len());

        for row_exprs in values {
            let mut record = Vec::with_capacity(table_info.columns.len());

            for col_idx in 0..table_info.columns.len() {
                if specified_cols.contains(&col_idx) {
                    // User specified this column - find its value
                    let col_name = &table_info.columns[col_idx].name;
                    let value_idx = columns
                        .iter()
                        .position(|c| c == col_name)
                        .unwrap_or(col_idx);
                    if let Some(expr) = row_exprs.get(value_idx) {
                        let val = self.evaluate_expression(expr);
                        record.push(val);
                    } else {
                        record.push(Value::Null);
                    }
                } else {
                    // Column not specified - check if AUTO_INCREMENT
                    if auto_increment_cols.contains(&col_idx) {
                        record.push(Value::Null); // Will be auto-generated
                    } else {
                        record.push(Value::Null);
                    }
                }
            }

            // Generate AUTO_INCREMENT values where needed
            for &auto_inc_idx in &auto_increment_cols {
                if record[auto_inc_idx] == Value::Null {
                    let next_id = self.storage.get_next_auto_increment(table_name)?;
                    record[auto_inc_idx] = Value::Integer(next_id);
                }
            }

            records.push(record);
        }

        // Insert the records
        self.storage.insert(table_name, records)?;

        let row_count = values.len();
        Ok(ExecutorResult::new(vec![], row_count))
    }

    /// Execute an UPDATE statement
    pub fn execute_update(&mut self, plan: &UpdateExec) -> SqlResult<ExecutorResult> {
        let table_name = plan.table_name();
        let updates = plan.updates();
        let predicate = plan.predicate();

        // Get table info
        let table_info = {
            let info = self.storage.get_table_info(table_name)?;
            info.clone()
        };

        // Find column indices for SET clauses
        let set_col_indices: Vec<(usize, Expr)> = updates
            .iter()
            .filter_map(|(col_name, expr)| {
                table_info
                    .columns
                    .iter()
                    .position(|c| c.name == *col_name)
                    .map(|idx| (idx, expr.clone()))
            })
            .collect();

        if set_col_indices.is_empty() {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        // Get all rows (read-only)
        let all_rows = self.storage.scan(table_name)?;
        let count = all_rows.len();

        if count == 0 {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        // Determine which rows match the predicate and compute updated rows
        let mut updated_count = 0;
        let updated_rows: Vec<Record> = all_rows
            .into_iter()
            .map(|row| {
                let matches = predicate.map_or(true, |pred| {
                    self.evaluate_predicate(pred, &row, &table_info)
                });

                if matches {
                    let mut new_row = row.clone();
                    for &(col_idx, ref set_expr) in &set_col_indices {
                        // Resolve column references first, then evaluate
                        let resolved_expr = self.resolve_row_expr(set_expr, &row, &table_info);
                        let new_val = self.evaluate_expression(&resolved_expr);
                        if col_idx < new_row.len() {
                            new_row[col_idx] = new_val;
                        }
                    }
                    // Count rows that actually changed
                    if new_row != row {
                        updated_count += 1;
                    }
                    new_row
                } else {
                    row
                }
            })
            .collect();

        // Write back all rows (replace entire table)
        self.storage.delete(table_name, &[])?;
        self.storage.insert(table_name, updated_rows)?;

        // If no predicate, all rows were "updated" (matched and potentially changed)
        if predicate.is_none() {
            updated_count = count;
        }

        Ok(ExecutorResult::new(vec![], updated_count))
    }

    /// Execute a DELETE statement
    pub fn execute_delete(&mut self, plan: &DeleteExec) -> SqlResult<ExecutorResult> {
        let table_name = plan.table_name();
        let predicate = plan.predicate();

        // If no predicate, delete all rows
        if predicate.is_none() {
            let count = self.storage.delete(table_name, &[])?;
            return Ok(ExecutorResult::new(vec![], count));
        }

        // Get table info
        let table_info = {
            let info = self.storage.get_table_info(table_name)?;
            info.clone()
        };

        // Get all rows
        let all_rows = self.storage.scan(table_name)?;
        let total_count = all_rows.len();

        // Find rows to keep (not delete)
        let rows_to_keep: Vec<Record> = all_rows
            .into_iter()
            .filter(|row| {
                !self.evaluate_predicate(predicate.as_ref().unwrap(), row, &table_info)
            })
            .collect();

        let deleted_count = total_count - rows_to_keep.len();

        // Write back remaining rows
        self.storage.delete(table_name, &[])?;
        if !rows_to_keep.is_empty() {
            self.storage.insert(table_name, rows_to_keep)?;
        }

        Ok(ExecutorResult::new(vec![], deleted_count))
    }

    /// Evaluate an expression to a Value
    fn evaluate_expression(&self, expr: &Expr) -> Value {
        match expr {
            Expr::Literal(v) => v.clone(),
            Expr::Column(_) => Value::Null, // TODO: resolve column references
            Expr::BinaryExpr { left, op, right } => {
                let left_val = self.evaluate_expression(left);
                let right_val = self.evaluate_expression(right);
                self.apply_binary_op(&left_val, op, &right_val)
            }
            Expr::UnaryExpr { op, expr } => {
                let val = self.evaluate_expression(expr);
                self.apply_unary_op(op, &val)
            }
            _ => Value::Null,
        }
    }

    /// Apply a binary operator
    fn apply_binary_op(&self, left: &Value, op: &sqlrustgo_planner::Operator, right: &Value) -> Value {
        use sqlrustgo_planner::Operator;
        match op {
            Operator::Eq => Value::Boolean(left == right),
            Operator::NotEq => Value::Boolean(left != right),
            Operator::Lt => Value::Boolean(left < right),
            Operator::LtEq => Value::Boolean(left <= right),
            Operator::Gt => Value::Boolean(left > right),
            Operator::GtEq => Value::Boolean(left >= right),
            Operator::And => {
                if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                    Value::Boolean(*l && *r)
                } else {
                    Value::Null
                }
            }
            Operator::Or => {
                if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                    Value::Boolean(*l || *r)
                } else {
                    Value::Null
                }
            }
            Operator::Plus => {
                if let (Value::Integer(l), Value::Integer(r)) = (left, right) {
                    Value::Integer(l + r)
                } else if let (Value::Float(l), Value::Float(r)) = (left, right) {
                    Value::Float(l + r)
                } else {
                    Value::Null
                }
            }
            Operator::Minus => {
                if let (Value::Integer(l), Value::Integer(r)) = (left, right) {
                    Value::Integer(l - r)
                } else if let (Value::Float(l), Value::Float(r)) = (left, right) {
                    Value::Float(l - r)
                } else {
                    Value::Null
                }
            }
            Operator::Multiply => {
                if let (Value::Integer(l), Value::Integer(r)) = (left, right) {
                    Value::Integer(l * r)
                } else if let (Value::Float(l), Value::Float(r)) = (left, right) {
                    Value::Float(l * r)
                } else {
                    Value::Null
                }
            }
            Operator::Divide => {
                if let (Value::Integer(l), Value::Integer(r)) = (left, right) {
                    if *r != 0 {
                        Value::Integer(l / r)
                    } else {
                        Value::Null
                    }
                } else if let (Value::Float(l), Value::Float(r)) = (left, right) {
                    if *r != 0.0 {
                        Value::Float(l / r)
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                }
            }
            Operator::Like => {
                // Simple LIKE implementation
                if let (Value::Text(pattern), Value::Text(text)) = (left, right) {
                    let pattern = pattern.replace('%', ".*").replace('_', ".");
                    Value::Boolean(text.matches(&pattern).count() > 0)
                } else {
                    Value::Null
                }
            }
            _ => Value::Null,
        }
    }

    /// Apply a unary operator
    fn apply_unary_op(&self, op: &sqlrustgo_planner::Operator, val: &Value) -> Value {
        use sqlrustgo_planner::Operator;
        match op {
            Operator::Not => {
                if let Value::Boolean(b) = val {
                    Value::Boolean(!*b)
                } else {
                    Value::Null
                }
            }
            Operator::Minus => {
                // Unary negation
                if let Value::Integer(i) = val {
                    Value::Integer(-*i)
                } else if let Value::Float(f) = val {
                    Value::Float(-*f)
                } else {
                    Value::Null
                }
            }
            _ => Value::Null,
        }
    }

    /// Evaluate a predicate against a row
    fn evaluate_predicate(&self, predicate: &Expr, row: &Record, table_info: &TableInfo) -> bool {
        // Resolve column references in the expression against the row
        let resolved_expr = self.resolve_row_expr(predicate, row, table_info);
        let val = self.evaluate_expression(&resolved_expr);
        if let Value::Boolean(b) = val {
            b
        } else {
            false
        }
    }

    /// Resolve column references in an expression against a row
    fn resolve_row_expr(&self, expr: &Expr, row: &Record, table_info: &TableInfo) -> Expr {
        match expr {
            Expr::Column(col) => {
                if let Some(idx) = table_info.columns.iter().position(|c| c.name == col.name) {
                    if idx < row.len() {
                        Expr::Literal(row[idx].clone())
                    } else {
                        Expr::Literal(Value::Null)
                    }
                } else {
                    Expr::Literal(Value::Null)
                }
            }
            Expr::BinaryExpr { left, op, right } => {
                Expr::BinaryExpr {
                    left: Box::new(self.resolve_row_expr(left, row, table_info)),
                    op: op.clone(),
                    right: Box::new(self.resolve_row_expr(right, row, table_info)),
                }
            }
            Expr::UnaryExpr { op, expr } => {
                Expr::UnaryExpr {
                    op: op.clone(),
                    expr: Box::new(self.resolve_row_expr(expr, row, table_info)),
                }
            }
            _ => expr.clone(),
        }
    }
}
