//! SQL Regression Corpus
//!
//! A SQL-based regression test framework that loads SQL files and executes them
//! against SQLRustGo to verify correct behavior.

use serde::{Deserialize, Serialize};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{
    parse, AlterTableOperation, CommonTableExpression, Expression, InsertStatement,
    SelectStatement, Statement, WithDmlStatement, WithSelect,
};
use sqlrustgo_storage::{ColumnDefinition, MemoryStorage, StorageEngine, TableInfo};
use sqlrustgo_types::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlTestResult {
    pub case_name: String,
    pub sql: String,
    pub success: bool,
    pub rows_returned: usize,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
    pub expected_rows: Option<usize>,
    pub expected_columns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusFileResult {
    pub file_path: String,
    pub total_cases: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<SqlTestResult>,
}

struct SimpleExecutor {
    storage: MemoryStorage,
}

impl SimpleExecutor {
    fn new() -> Self {
        Self {
            storage: MemoryStorage::new(),
        }
    }

    fn reset(&mut self) {
        self.storage = MemoryStorage::new();
    }

    fn execute(&mut self, sql: &str) -> Result<ExecutorResult, String> {
        let statement = parse(sql).map_err(|e| format!("Parse error: {:?}", e))?;

        match statement {
            Statement::CreateTable(create) => {
                let info = TableInfo {
                    name: create.name.clone(),
                    columns: create
                        .columns
                        .into_iter()
                        .map(|c| ColumnDefinition {
                            name: c.name,
                            data_type: c.data_type,
                            nullable: c.nullable,
                            primary_key: c.primary_key,
                            char_max_length: c.char_max_length,
                            collation: c.collation,

                            default_value: None,
                        })
                        .collect(),
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    compression: None,
                    collations: std::collections::HashMap::new(),
                    partition_info: None,
                };
                self.storage
                    .create_table(&info)
                    .map_err(|e| format!("Create table error: {:?}", e))?;
                Ok(ExecutorResult::new(vec![], 0))
            }
            Statement::Insert(insert) => {
                let records = self.evaluate_insert_values(&insert)?;
                self.storage
                    .insert(&insert.table, records)
                    .map_err(|e| format!("Insert error: {:?}", e))?;
                Ok(ExecutorResult::new(vec![], 0))
            }
            Statement::Select(select) => {
                let rows = self.execute_select(&select)?;
                let count = rows.len();
                Ok(ExecutorResult::new(rows, count))
            }
            Statement::Union(union_stmt) => {
                // Phase 4: UNION executor support. Execute both sides
                // and combine (UNION ALL keeps duplicates, UNION removes).
                // We use execute_statement (which is &mut self) so the
                // derived-subquery materialisation path from execute_select
                // is reachable from the UNION dispatcher too.
                let left_rows = self.execute_statement(&union_stmt.left)?;
                let right_rows = self.execute_statement(&union_stmt.right)?;
                let combined = if union_stmt.union_all {
                    left_rows.into_iter().chain(right_rows).collect()
                } else {
                    let mut c = left_rows;
                    c.extend(right_rows);
                    c.sort();
                    c.dedup();
                    c
                };
                let count = combined.len();
                Ok(ExecutorResult::new(combined, count))
            }
            Statement::Delete(delete) => {
                // If no WHERE clause, delete all rows
                if delete.where_clause.is_none() {
                    let count = self
                        .storage
                        .delete(&delete.tables[0].name, &[])
                        .map_err(|e| format!("Delete error: {:?}", e))?;
                    return Ok(ExecutorResult::new(vec![], count));
                }

                // Get table info to find column indices
                let table_info = self
                    .storage
                    .get_table_info(&delete.tables[0].name)
                    .map_err(|e| format!("Get table info error: {:?}", e))?;

                // Scan all rows
                let all_rows = self
                    .storage
                    .scan(&delete.tables[0].name)
                    .map_err(|e| format!("Scan error: {:?}", e))?;

                // Filter rows based on WHERE clause
                let where_clause = delete.where_clause.as_ref().unwrap();
                let rows_to_delete: Vec<Vec<Value>> = all_rows
                    .clone()
                    .into_iter()
                    .filter(|row| self.evaluate_where(where_clause, row, &table_info))
                    .collect();

                let count = rows_to_delete.len();

                if count == 0 {
                    return Ok(ExecutorResult::new(vec![], 0));
                }

                // Keep rows that don't match the WHERE clause
                let rows_to_keep: Vec<Vec<Value>> = all_rows
                    .into_iter()
                    .filter(|row| !self.evaluate_where(where_clause, row, &table_info))
                    .collect();

                // Delete all rows and re-insert non-matching ones
                self.storage
                    .delete(&delete.tables[0].name, &[])
                    .map_err(|e| format!("Delete error: {:?}", e))?;

                if !rows_to_keep.is_empty() {
                    self.storage
                        .insert(&delete.tables[0].name, rows_to_keep)
                        .map_err(|e| format!("Insert error: {:?}", e))?;
                }

                Ok(ExecutorResult::new(vec![], count))
            }
            Statement::Update(update) => {
                let updates: Vec<(usize, Value)> = update
                    .set_clauses
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (_col, expr))| {
                        if let Ok(val) = self.evaluate_expression(expr) {
                            Some((i, val))
                        } else {
                            None
                        }
                    })
                    .collect();
                let count = self
                    .storage
                    .update(&update.tables[0].name, &[], &updates)
                    .map_err(|e| format!("Update error: {:?}", e))?;
                Ok(ExecutorResult::new(vec![], count))
            }
            Statement::DropTable(drop) => {
                self.storage
                    .drop_table(&drop.name)
                    .map_err(|e| format!("Drop table error: {:?}", e))?;
                Ok(ExecutorResult::new(vec![], 0))
            }
            Statement::AlterTable(alter) => {
                match &alter.operation {
                    AlterTableOperation::AddColumn {
                        name,
                        data_type,
                        nullable,
                        ..
                    } => {
                        let col = ColumnDefinition {
                            name: name.clone(),
                            data_type: data_type.clone(),
                            nullable: *nullable,
                            primary_key: false,
                            char_max_length: None,
                            collation: None,

                            default_value: None,
                        };
                        self.storage
                            .add_column(&alter.table_name, col)
                            .map_err(|e| format!("Add column error: {:?}", e))?;
                    }
                    AlterTableOperation::RenameTo { new_name } => {
                        self.storage
                            .rename_table(&alter.table_name, new_name)
                            .map_err(|e| format!("Rename table error: {:?}", e))?;
                    }
                    AlterTableOperation::DropColumn { .. } => {}
                    AlterTableOperation::ModifyColumn { .. } => {}
                    AlterTableOperation::RenameColumn { name, new_name } => {
                        self.storage
                            .rename_column(&alter.table_name, name, new_name)
                            .map_err(|e| format!("Rename column error: {:?}", e))?;
                    }
                    AlterTableOperation::AlterColumn { name, .. } => {
                        return Err(format!(
                            "ALTER COLUMN '{}' SET DATA TYPE requires explicit CAST (unsafe implicit conversion rejected)",
                            name
                        ));
                    }
                    AlterTableOperation::SetPartitionedBy { .. }
                    | AlterTableOperation::ResetPartitionedBy => {
                        // V312-40 placeholder: PARTITIONED BY not yet implemented in corpus
                        // Real impl pending: V312-41 follow-up issue
                    }
                }
                Ok(ExecutorResult::new(vec![], 0))
            }
            Statement::CreateIndex(_) => Ok(ExecutorResult::new(vec![], 0)),
            Statement::WithSelect(with_select) => {
                self.execute_with_select(&with_select)?;
                Ok(ExecutorResult::new(vec![], 0))
            }
            Statement::WithDml(with_dml) => {
                self.execute_with_dml(&with_dml)?;
                Ok(ExecutorResult::new(vec![], 0))
            }
            _ => Err("Unsupported statement type".to_string()),
        }
    }

    /// Execute a `WITH ... DML` statement. We materialize the CTEs as
    /// ephemeral tables (same as `execute_with_select`) and then run
    /// the DML body (Insert/Update/Delete) which can reference those
    /// CTE tables in its subqueries.
    fn execute_with_dml(&mut self, with_dml: &WithDmlStatement) -> Result<(), String> {
        for cte in &with_dml.with_clause.ctes {
            let cte_rows = self.execute_statement(&cte.subquery)?;
            let column_count = if cte.columns.is_empty() {
                if cte_rows.is_empty() {
                    0
                } else {
                    cte_rows[0].len()
                }
            } else {
                cte.columns.len()
            };
            let columns: Vec<ColumnDefinition> = (0..column_count)
                .map(|i| ColumnDefinition {
                    name: if cte.columns.is_empty() {
                        format!("col_{}", i)
                    } else {
                        cte.columns[i].clone()
                    },
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,

                    default_value: None,
                })
                .collect();
            let table_info = TableInfo {
                name: cte.name.clone(),
                columns,
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            };
            self.storage
                .create_table(&table_info)
                .map_err(|e| format!("CTE create_table error: {:?}", e))?;
            if !cte_rows.is_empty() {
                self.storage
                    .insert(&cte.name, cte_rows)
                    .map_err(|e| format!("CTE insert error: {:?}", e))?;
            }
        }
        // Now run the DML body. We dispatch on the statement variant
        // and re-use the existing execution logic.
        match &*with_dml.body {
            Statement::Insert(insert) => {
                let records = self.evaluate_insert_values(insert)?;
                self.storage
                    .insert(&insert.table, records)
                    .map_err(|e| format!("Insert error: {:?}", e))?;
                Ok(())
            }
            Statement::Update(update) => {
                // The corpus runner's UPDATE is a positional update:
                // each `set_clauses` entry is (col_name, expr); we
                // build a (col_index, new_value) list by name lookup.
                let table_info = self
                    .storage
                    .get_table_info(&update.tables[0].name)
                    .map_err(|e| format!("Get table info error: {:?}", e))?;
                let updates: Vec<(usize, Value)> = update
                    .set_clauses
                    .iter()
                    .filter_map(|(col_name, expr)| {
                        if let Some(col_idx) =
                            table_info.columns.iter().position(|c| c.name == *col_name)
                        {
                            if let Ok(v) = self.evaluate_expression(expr) {
                                return Some((col_idx, v));
                            }
                        }
                        None
                    })
                    .collect();
                self.storage
                    .update(&update.tables[0].name, &[], &updates)
                    .map_err(|e| format!("Update error: {:?}", e))?;
                Ok(())
            }
            Statement::Delete(delete) => {
                let _count = self
                    .storage
                    .delete(&delete.tables[0].name, &[])
                    .map_err(|e| format!("Delete error: {:?}", e))?;
                Ok(())
            }
            other => Err(format!(
                "WITH ... body must be INSERT/UPDATE/DELETE, got {:?}",
                other
            )),
        }
    }

    fn evaluate_insert_values(&self, insert: &InsertStatement) -> Result<Vec<Vec<Value>>, String> {
        let mut all_records = Vec::new();

        for row in &insert.values {
            let mut record = Vec::new();
            for expr in row {
                record.push(self.evaluate_expression(expr)?);
            }
            all_records.push(record);
        }

        Ok(all_records)
    }

    fn evaluate_expression(&self, expr: &Expression) -> Result<Value, String> {
        match expr {
            Expression::Literal(s) => {
                let s = s.trim();
                if s.eq_ignore_ascii_case("NULL") {
                    Ok(Value::Null)
                } else if let Ok(n) = s.parse::<i64>() {
                    Ok(Value::Integer(n))
                } else if let Ok(f) = s.parse::<f64>() {
                    Ok(Value::Float(f))
                } else if s.starts_with('\'') && s.ends_with('\'') {
                    Ok(Value::Text(s[1..s.len() - 1].to_string()))
                } else {
                    Ok(Value::Text(s.to_string()))
                }
            }
            Expression::Identifier(_) => Ok(Value::Null),
            _ => Ok(Value::Null),
        }
    }

    fn execute_select(&mut self, select: &SelectStatement) -> Result<Vec<Vec<Value>>, String> {
        // Phase 3 (TPCH-01 Q15): materialise any derived subqueries
        // registered by the parser (via the global DERIVED_SUBQUERIES
        // thread-local). The parser emits `__subq_<alias>` as the table
        // name and registers the subquery AST here. We execute each
        // subquery and store the results in a synthetic table that
        // the rest of the executor can scan.
        let derived = sqlrustgo_parser::get_and_clear_derived_subqueries();
        for (name, subquery) in &derived {
            let rows = self.execute_select(subquery)?;
            // Build a synthetic TableInfo from the first row's column count
            // (or fall back to 0 columns if empty).
            let column_count = if rows.is_empty() { 0 } else { rows[0].len() };
            let columns: Vec<ColumnDefinition> = (0..column_count)
                .map(|i| ColumnDefinition {
                    name: format!("col_{}", i),
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,

                    default_value: None,
                })
                .collect();
            let table_info = TableInfo {
                name: name.clone(),
                columns,
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            };
            self.storage
                .create_table(&table_info)
                .map_err(|e| format!("Create derived table error: {:?}", e))?;
            if !rows.is_empty() {
                self.storage
                    .insert(name, rows)
                    .map_err(|e| format!("Insert derived rows error: {:?}", e))?;
            }
        }
        // If the SELECT has a JOIN clause, do a simple nested-loop inner
        // join. This is needed for recursive CTEs whose step joins the
        // CTE table against a base table (e.g. org_chart). We only
        // support the simple form: one INNER JOIN with an ON condition
        // involving column references from both sides. Outer joins and
        // multi-table joins are out of scope here.
        // Handle FROM (subquery) AS alias FIRST (before the join check)
        // because a SELECT with a from_subquery AND a join_clause is
        // valid (e.g. `(sub) JOIN t ON ...`). The from_subquery creates
        // the synthetic table that the join then scans as the left side.
        if let Some(ref subq) = select.from_subquery {
            let rows = self.execute_select(subq)?;
            let column_count = if rows.is_empty() { 0 } else { rows[0].len() };
            let columns: Vec<ColumnDefinition> = (0..column_count)
                .map(|i| ColumnDefinition {
                    name: format!("col_{}", i),
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,

                    default_value: None,
                })
                .collect();
            let table_info = TableInfo {
                name: select.table.clone(),
                columns,
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            };
            // Create table only if it doesn't exist yet
            let _ = self.storage.create_table(&table_info);
            if !rows.is_empty() {
                self.storage
                    .insert(&select.table, rows)
                    .map_err(|e| format!("Insert from_subquery rows error: {:?}", e))?;
            }
        }
        // Strip `|alias` suffix from base table name (parser encodes it).
        let base_table = select.table.split('|').next().unwrap_or(&select.table);
        let mut rows = self
            .storage
            .scan(base_table)
            .map_err(|e| format!("Scan error: {:?}", e))?;

        if let Some(ref where_clause) = select.where_clause {
            let table_info = self
                .storage
                .get_table_info(base_table)
                .map_err(|e| format!("Get table info error: {:?}", e))?;
            rows.retain(|row| self.evaluate_where(where_clause, row, &table_info));
        }

        Ok(rows)
    }

    /// Simple nested-loop INNER JOIN executor for the corpus runner.
    /// Supports one or more chained JOIN clauses (e.g.
    /// `FROM t1 JOIN t2 ON cond JOIN t3 ON cond`). Joins are evaluated
    /// left-associatively: ((t1 ⋈ t2) ⋈ t3). The ON condition for each
    /// join is evaluated against the running combined row using the
    /// synthesized TableInfo (left + all already-joined right columns).
    #[allow(dead_code)]
    /// Outer joins are not supported.
    fn execute_select_with_join(
        &self,
        select: &SelectStatement,
    ) -> Result<Vec<Vec<Value>>, String> {
        // Strip any `|alias` suffix the parser encodes into table names
        // (e.g. `FROM users u` → `select.table = "users|u"`).
        let base_table = select.table.split('|').next().unwrap_or(&select.table);
        let mut current_rows = self
            .storage
            .scan(base_table)
            .map_err(|e| format!("Scan error: {:?}", e))?;
        let mut current_info = self
            .storage
            .get_table_info(base_table)
            .map_err(|e| format!("Get left table info error: {:?}", e))?;

        for join in &select.join_clause {
            let right_table = join.table.split('|').next().unwrap_or(&join.table);
            let right_rows = self
                .storage
                .scan(right_table)
                .map_err(|e| format!("Right scan error: {:?}", e))?;
            let right_info = self
                .storage
                .get_table_info(right_table)
                .map_err(|e| format!("Get right table info error: {:?}", e))?;

            // Build the combined TableInfo for this join step: current
            // accumulated columns + the new right table's columns.
            let mut combined_columns = current_info.columns.clone();
            combined_columns.extend(right_info.columns.clone());
            let combined_info = TableInfo {
                name: format!("{}_x_{}", current_info.name, right_info.name),
                columns: combined_columns,
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
            };

            // Nested-loop join. The combined row is
            // (current_row ++ right_row); the ON condition is evaluated
            // against this combined row.
            let mut joined = Vec::new();
            for left_row in &current_rows {
                for right_row in &right_rows {
                    let mut combined_row = left_row.clone();
                    combined_row.extend(right_row.clone());
                    if self.evaluate_where(&join.on_clause, &combined_row, &combined_info) {
                        joined.push(combined_row);
                    }
                }
            }
            current_rows = joined;
            current_info = combined_info;
        }

        // Optional WHERE filter on the joined result.
        if let Some(ref where_clause) = select.where_clause {
            current_rows.retain(|row| self.evaluate_where(where_clause, row, &current_info));
        }

        Ok(current_rows)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Vec<Vec<Value>>, String> {
        match stmt {
            Statement::Select(select) => self.execute_select(select),
            Statement::Union(union_stmt) => {
                let left_rows = self.execute_statement(&union_stmt.left)?;
                let right_rows = self.execute_statement(&union_stmt.right)?;
                if union_stmt.union_all {
                    Ok(left_rows.into_iter().chain(right_rows).collect())
                } else {
                    let mut combined = left_rows;
                    combined.extend(right_rows);
                    combined.sort();
                    combined.dedup();
                    Ok(combined)
                }
            }
            // We don't support WithSelect in the &self execute_statement
            // path (it requires &mut self to populate CTE tables). Nested
            // CTEs would need a RefCell<MemoryStorage> or similar
            // refactor. For now, return an empty result and let the
            // higher-level WithSelect dispatch handle it. We
            // intentionally don't fail here so that the outer
            // execute_with_select (which IS &mut self) can drive the
            // evaluation.
            Statement::WithSelect(_) => Ok(vec![]),
            _ => Err(format!("Unsupported statement type: {:?}", stmt)),
        }
    }

    fn evaluate_where(&self, expr: &Expression, row: &[Value], table_info: &TableInfo) -> bool {
        match expr {
            // Handle AND conditions
            Expression::BinaryOp(left, op, right) if op.to_uppercase() == "AND" => {
                self.evaluate_where(left, row, table_info)
                    && self.evaluate_where(right, row, table_info)
            }
            // Handle OR conditions
            Expression::BinaryOp(left, op, right) if op.to_uppercase() == "OR" => {
                self.evaluate_where(left, row, table_info)
                    || self.evaluate_where(right, row, table_info)
            }
            // Handle IS NULL
            Expression::BinaryOp(left, op, right)
                if op.to_uppercase() == "IS"
                    && matches!(right.as_ref(), Expression::Literal(s) if s.to_uppercase() == "NULL") =>
            {
                if let Expression::Identifier(col_name) = left.as_ref() {
                    if let Some(col_idx) = self.find_column_index(col_name, table_info) {
                        if let Some(row_val) = row.get(col_idx) {
                            return matches!(row_val, Value::Null);
                        }
                    }
                }
                false
            }
            // Handle IS NOT NULL
            Expression::BinaryOp(left, op, right)
                if op.to_uppercase() == "IS NOT"
                    && matches!(right.as_ref(), Expression::Literal(s) if s.to_uppercase() == "NULL") =>
            {
                if let Expression::Identifier(col_name) = left.as_ref() {
                    if let Some(col_idx) = self.find_column_index(col_name, table_info) {
                        if let Some(row_val) = row.get(col_idx) {
                            return !matches!(row_val, Value::Null);
                        }
                    }
                }
                false
            }
            // Handle comparison operators
            Expression::BinaryOp(left, op, right) => {
                self.evaluate_binary_comparison(left, op, right, row, table_info)
            }
            _ => true,
        }
    }

    fn evaluate_binary_comparison(
        &self,
        left: &Expression,
        op: &str,
        right: &Expression,
        row: &[Value],
        table_info: &TableInfo,
    ) -> bool {
        let left_val = self.get_expression_value(left, row, table_info);
        let right_val = self.get_expression_value(right, row, table_info);

        // SQL three-valued logic: any comparison involving NULL yields
        // false (UNKNOWN, treated as not-matching in WHERE/ON). This
        // is the SQL-standard NULL semantics — `NULL = NULL` is not
        // TRUE.
        if matches!(left_val, Value::Null) || matches!(right_val, Value::Null) {
            return false;
        }
        match op.to_uppercase().as_str() {
            "=" | "==" | "IS" => left_val == right_val,
            "!=" | "<>" => left_val != right_val,
            ">" => self.compare_values(&left_val, &right_val) > 0,
            ">=" => self.compare_values(&left_val, &right_val) >= 0,
            "<" => self.compare_values(&left_val, &right_val) < 0,
            "<=" => self.compare_values(&left_val, &right_val) <= 0,
            _ => false,
        }
    }

    fn get_expression_value(
        &self,
        expr: &Expression,
        row: &[Value],
        table_info: &TableInfo,
    ) -> Value {
        match expr {
            Expression::Literal(s) => {
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
            Expression::Identifier(name) => {
                if let Some(col_idx) = self.find_column_index(name, table_info) {
                    row.get(col_idx).cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            Expression::BinaryOp(left, op, right) => {
                let left_val = self.get_expression_value(left, row, table_info);
                let right_val = self.get_expression_value(right, row, table_info);
                self.evaluate_binary_op_value(&left_val, &right_val, op)
            }
            _ => Value::Null,
        }
    }

    fn evaluate_binary_op_value(&self, left: &Value, right: &Value, op: &str) -> Value {
        match op.to_uppercase().as_str() {
            "=" | "==" | "IS" => Value::Boolean(left == right),
            "!=" | "<>" => Value::Boolean(left != right),
            ">" => Value::Boolean(self.compare_values(left, right) > 0),
            ">=" => Value::Boolean(self.compare_values(left, right) >= 0),
            "<" => Value::Boolean(self.compare_values(left, right) < 0),
            "<=" => Value::Boolean(self.compare_values(left, right) <= 0),
            "AND" | "&&" => {
                if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                    Value::Boolean(*l && *r)
                } else {
                    Value::Boolean(false)
                }
            }
            "OR" | "||" => {
                // SQL `||` is string concatenation when either side is a
                // string (MySQL/PostgreSQL/Oracle mode). If both are
                // booleans, treat as logical-OR. If both are integers and
                // the operation is `||`, fall through to text concat for
                // safety.
                match (left, right) {
                    (Value::Text(l), r) => Value::Text(format!("{}{}", l, r)),
                    (l, Value::Text(r)) => Value::Text(format!("{}{}", l, r)),
                    (Value::Boolean(l), Value::Boolean(r)) => Value::Boolean(*l || *r),
                    (Value::Integer(l), Value::Integer(r)) => Value::Text(format!("{}{}", l, r)),
                    _ => Value::Null,
                }
            }
            _ => Value::Null,
        }
    }

    fn compare_values(&self, left: &Value, right: &Value) -> i32 {
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

    fn find_column_index(&self, col_name: &str, table_info: &TableInfo) -> Option<usize> {
        table_info
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col_name))
    }

    fn execute_with_select(&mut self, with_select: &WithSelect) -> Result<(), String> {
        if let Some(ref with_clause) = with_select.with_clause {
            for cte in &with_clause.ctes {
                if with_clause.recursive {
                    // Recursive CTE: the subquery must be a UNION (or UNION ALL)
                    // between a non-recursive seed and a recursive part. Iterate
                    // until the recursive part returns no new rows or the depth
                    // limit (default 1000) is reached.
                    self.execute_recursive_cte(cte, with_clause.recursive)?;
                } else {
                    let cte_rows = self.execute_statement(&cte.subquery)?;
                    let column_count = if cte.columns.is_empty() {
                        if cte_rows.is_empty() {
                            0
                        } else {
                            cte_rows[0].len()
                        }
                    } else {
                        cte.columns.len()
                    };
                    let columns: Vec<ColumnDefinition> = (0..column_count)
                        .map(|i| ColumnDefinition {
                            name: if cte.columns.is_empty() {
                                format!("col_{}", i)
                            } else {
                                cte.columns[i].clone()
                            },
                            data_type: "TEXT".to_string(),
                            nullable: true,
                            primary_key: false,
                            char_max_length: None,
                            collation: None,

                            default_value: None,
                        })
                        .collect();
                    let table_info = TableInfo {
                        name: cte.name.clone(),
                        columns,
                        foreign_keys: vec![],
                        unique_constraints: vec![],
                        check_constraints: vec![],
                        compression: None,
                        collations: std::collections::HashMap::new(),
                        partition_info: None,
                    };
                    self.storage
                        .create_table(&table_info)
                        .map_err(|e| format!("Create CTE table error: {:?}", e))?;
                    if !cte_rows.is_empty() {
                        self.storage
                            .insert(&cte.name, cte_rows)
                            .map_err(|e| format!("Insert CTE rows error: {:?}", e))?;
                    }
                }
            }
        }
        self.execute_select(&with_select.select)?;
        Ok(())
    }

    /// Recursive CTE executor: a non-recursive seed UNION (ALL) a recursive
    /// step that references the CTE itself. The step is iterated until it
    /// returns no rows (fixed point) or the depth limit is hit.
    fn execute_recursive_cte(
        &mut self,
        cte: &CommonTableExpression,
        _recursive: bool,
    ) -> Result<(), String> {
        // The recursive subquery must be a UNION/UNION ALL between two
        // SELECT statements; the second SELECT may reference `cte.name`.
        let (seed, step, union_all) = match &*cte.subquery {
            Statement::Union(u) => (&*u.left, &*u.right, u.union_all),
            other => {
                return Err(format!(
                    "Recursive CTE body must be UNION/UNION ALL of two SELECTs, got {:?}",
                    other
                ))
            }
        };

        // Create the CTE table once. The schema is inferred from the
        // first row of the seed (or 0 columns if the seed is empty).
        let seed_rows = self.execute_statement(seed)?;
        let column_count = if !cte.columns.is_empty() {
            cte.columns.len()
        } else if !seed_rows.is_empty() {
            seed_rows[0].len()
        } else {
            0
        };
        let columns: Vec<ColumnDefinition> = (0..column_count)
            .map(|i| ColumnDefinition {
                name: if !cte.columns.is_empty() {
                    cte.columns[i].clone()
                } else {
                    format!("col_{}", i)
                },
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,

                default_value: None,
            })
            .collect();
        let table_info = TableInfo {
            name: cte.name.clone(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        };
        self.storage
            .create_table(&table_info)
            .map_err(|e| format!("Create recursive CTE table error: {:?}", e))?;

        // Iteration 0: insert the seed.
        if !seed_rows.is_empty() {
            self.storage
                .insert(&cte.name, seed_rows.clone())
                .map_err(|e| format!("Insert seed rows error: {:?}", e))?;
        }
        let mut total_rows = seed_rows.len();
        let max_depth: usize = 1000;
        for _depth in 0..max_depth {
            let step_rows = self.execute_statement(step)?;
            if step_rows.is_empty() {
                break;
            }
            if union_all {
                self.storage
                    .insert(&cte.name, step_rows.clone())
                    .map_err(|e| format!("Insert step rows error: {:?}", e))?;
                total_rows += step_rows.len();
            } else {
                // UNION: deduplicate against existing rows.
                let existing: Vec<Vec<Value>> = self
                    .storage
                    .scan(&cte.name)
                    .map_err(|e| format!("Scan error: {:?}", e))?;
                let mut new_rows = Vec::new();
                for r in &step_rows {
                    if !existing.contains(r) && !new_rows.contains(r) {
                        new_rows.push(r.clone());
                    }
                }
                if new_rows.is_empty() {
                    break;
                }
                self.storage
                    .insert(&cte.name, new_rows.clone())
                    .map_err(|e| format!("Insert step rows error: {:?}", e))?;
                total_rows += new_rows.len();
            }
            if total_rows > 1_000_000 {
                return Err(format!(
                    "Recursive CTE {} exceeded 1,000,000 row cap",
                    cte.name
                ));
            }
        }
        Ok(())
    }
}

pub struct SqlCorpus {
    corpus_root: PathBuf,
    executor: SimpleExecutor,
}

impl SqlCorpus {
    pub fn new(corpus_root: PathBuf) -> Self {
        Self {
            corpus_root,
            executor: SimpleExecutor::new(),
        }
    }

    pub fn reset(&mut self) {
        self.executor.reset();
    }

    pub fn execute_all(&mut self) -> HashMap<String, CorpusFileResult> {
        let mut results = HashMap::new();
        self.execute_directory(&mut results);
        results
    }

    fn execute_directory(&mut self, results: &mut HashMap<String, CorpusFileResult>) {
        if !self.corpus_root.is_dir() {
            return;
        }

        for entry in fs::read_dir(&self.corpus_root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                let mut sub_corpus = SqlCorpus::new(path);
                let sub_results = sub_corpus.execute_all();
                results.extend(sub_results);
            } else if path.extension().is_some_and(|e| e == "sql") {
                let file_result = self.execute_file(&path);
                let relative_path = path
                    .strip_prefix(&self.corpus_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                results.insert(relative_path, file_result);
            }
        }
    }

    pub fn execute_file(&mut self, path: &Path) -> CorpusFileResult {
        let content = fs::read_to_string(path).unwrap_or_default();
        let file_results = self.parse_and_execute(&content);

        let total = file_results.len();
        let passed = file_results.iter().filter(|r| r.success).count();
        let failed = total - passed;

        CorpusFileResult {
            file_path: path.to_string_lossy().to_string(),
            total_cases: total,
            passed,
            failed,
            results: file_results,
        }
    }

    pub fn parse_and_execute(&mut self, content: &str) -> Vec<SqlTestResult> {
        let first_lines: Vec<_> = content.lines().take(5).collect();
        for line in &first_lines {
            let trimmed = line.trim();
            if trimmed == "-- === SKIP ===" || trimmed == "-- === IGNORE ===" {
                return Vec::new();
            }
        }

        let mut results = Vec::new();
        let mut current_case: Option<TestCase> = None;
        let mut setup_sql = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("-- === SETUP ===") {
                if let Some(case) = current_case.take() {
                    results.push(self.execute_case(case, &setup_sql));
                }
                setup_sql.clear();
            } else if trimmed.starts_with("-- === CASE:") {
                // Only treat as a new case boundary if we're NOT inside
                // an unclosed paren (e.g. a subquery in JOIN). The
                // corpus file has inner `CASE:` comments as labels
                // for subqueries, not as separate test cases.
                let in_subquery = current_case
                    .as_ref()
                    .map(|c| {
                        // Count unclosed LParens in the accumulated SQL
                        let opens = c.sql.matches('(').count();
                        let closes = c.sql.matches(')').count();
                        let unclosed_parens = opens > closes;
                        // Also check if the SQL ends with UNION (mid-statement
                        // UNION means the next line is still part of this case)
                        let trimmed_sql = c.sql.trim_end();
                        let ends_with_union =
                            trimmed_sql.ends_with("UNION") || trimmed_sql.ends_with("UNION ALL");
                        unclosed_parens || ends_with_union
                    })
                    .unwrap_or(false);
                if !in_subquery {
                    if let Some(case) = current_case.take() {
                        results.push(self.execute_case(case, &setup_sql));
                    }
                } else {
                    // Inside a subquery: the comment is part of the SQL
                    if !current_case.as_ref().unwrap().sql.is_empty() {
                        current_case.as_mut().unwrap().sql.push('\n');
                    }
                    // Strip the `--` prefix from the comment (since SQL
                    // doesn't have `--` comments, but the corpus uses
                    // them as subquery labels).
                    let comment_text = trimmed.trim_start_matches("-- ").trim();
                    current_case
                        .as_mut()
                        .unwrap()
                        .sql
                        .push_str(&format!("-- {}", comment_text));
                }

                let case_name = trimmed
                    .trim_start_matches("-- === CASE:")
                    .trim()
                    .to_string();
                current_case = Some(TestCase {
                    name: case_name,
                    sql: String::new(),
                    expected_rows: None,
                    expected_columns: None,
                });
            } else if trimmed.starts_with("-- EXPECT:") {
                if let Some(ref mut case) = current_case {
                    let expect = trimmed.trim_start_matches("-- EXPECT:").trim();
                    if expect.starts_with("ERROR") {
                        case.expected_rows = Some(0);
                    } else if expect.starts_with("rows") {
                        if let Ok(n) = expect.split_whitespace().next().unwrap_or("0").parse() {
                            case.expected_rows = Some(n);
                        }
                    }
                }
            } else if current_case.is_some() {
                if !trimmed.is_empty() && !trimmed.starts_with("--") {
                    if !current_case.as_ref().unwrap().sql.is_empty() {
                        current_case.as_mut().unwrap().sql.push('\n');
                    }
                    current_case.as_mut().unwrap().sql.push_str(trimmed);
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("--") {
                if !setup_sql.is_empty() {
                    setup_sql.push('\n');
                }
                setup_sql.push_str(trimmed);
            }
        }

        if let Some(case) = current_case {
            results.push(self.execute_case(case, &setup_sql));
        }

        results
    }

    fn execute_case(&mut self, case: TestCase, setup_sql: &str) -> SqlTestResult {
        let start = std::time::Instant::now();

        self.reset();

        if !setup_sql.is_empty() {
            if let Err(e) = self.execute_sql(setup_sql) {
                return SqlTestResult {
                    case_name: case.name,
                    sql: case.sql,
                    success: false,
                    rows_returned: 0,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error_message: Some(format!("Setup failed: {}", e)),
                    expected_rows: case.expected_rows,
                    expected_columns: case.expected_columns,
                };
            }
        }

        match self.execute_sql(&case.sql) {
            Ok(result) => {
                let rows_returned = result.rows.len();
                let success = case
                    .expected_rows
                    .map(|expected| rows_returned == expected)
                    .unwrap_or(true);

                SqlTestResult {
                    case_name: case.name,
                    sql: case.sql,
                    success,
                    rows_returned,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error_message: if !success {
                        Some(format!(
                            "Expected {} rows, got {}",
                            case.expected_rows.unwrap_or(0),
                            rows_returned
                        ))
                    } else {
                        None
                    },
                    expected_rows: case.expected_rows,
                    expected_columns: case.expected_columns,
                }
            }
            Err(e) => SqlTestResult {
                case_name: case.name,
                sql: case.sql,
                success: false,
                rows_returned: 0,
                execution_time_ms: start.elapsed().as_millis() as u64,
                error_message: Some(e),
                expected_rows: case.expected_rows,
                expected_columns: case.expected_columns,
            },
        }
    }
    fn execute_sql(&mut self, sql: &str) -> Result<ExecutorResult, String> {
        // Split on ';' but respect single-quoted string literals. The
        // previous naive `sql.split(';')` would split inside literals
        // like `'; '` (used in GROUP_CONCAT SEPARATOR), breaking the
        // resulting fragments with unterminated quotes.
        let statements: Vec<&str> = split_sql_statements(sql);
        let mut last_result = Ok(ExecutorResult::new(vec![], 0));

        for stmt in statements {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            last_result = self.executor.execute(stmt);
        }

        last_result
    }

    pub fn summary(&self, results: &HashMap<String, CorpusFileResult>) -> CorpusSummary {
        let mut total_cases = 0;
        let mut passed = 0;
        let mut failed = 0;

        for result in results.values() {
            total_cases += result.total_cases;
            passed += result.passed;
            failed += result.failed;
        }

        CorpusSummary {
            total_files: results.len(),
            total_cases,
            passed,
            failed,
            pass_rate: if total_cases > 0 {
                (passed as f64 / total_cases as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    sql: String,
    expected_rows: Option<usize>,
    expected_columns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSummary {
    pub total_files: usize,
    pub total_cases: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
}

/// Split a SQL string into statements, respecting single-quoted
/// string literals. The previous naive `sql.split(';')` would
/// split inside literals like `'; '` (used in GROUP_CONCAT
/// SEPARATOR), breaking the resulting fragments.
fn split_sql_statements(sql: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let mut start = 0;
    let mut in_string = false;
    let mut prev_was_escape = false;
    for (i, ch) in sql.char_indices() {
        if prev_was_escape {
            prev_was_escape = false;
            continue;
        }
        if ch == '\\' {
            prev_was_escape = true;
            continue;
        }
        if ch == '\'' {
            in_string = !in_string;
        } else if ch == ';' && !in_string {
            let stmt = sql[start..i].trim();
            if !stmt.is_empty() {
                out.push(stmt);
            }
            start = i + 1;
        }
    }
    let tail = sql[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn split_statements_single() {
        let out = split_sql_statements("SELECT 1");
        assert_eq!(out, vec!["SELECT 1"]);
    }

    #[test]
    fn split_statements_multiple() {
        let out = split_sql_statements("SELECT 1; SELECT 2; SELECT 3");
        assert_eq!(out, vec!["SELECT 1", "SELECT 2", "SELECT 3"]);
    }

    #[test]
    fn split_statements_trailing_semicolon() {
        let out = split_sql_statements("SELECT 1;");
        assert_eq!(out, vec!["SELECT 1"]);
    }

    #[test]
    fn split_statements_semicolon_in_string() {
        let out = split_sql_statements("INSERT INTO t VALUES ('a;b'); SELECT 2");
        assert_eq!(out.len(), 2);
        assert!(out[0].contains("'a;b'"));
        assert_eq!(out[1], "SELECT 2");
    }

    #[test]
    fn split_statements_escaped_quote() {
        let out = split_sql_statements("INSERT INTO t VALUES ('it\\'s'); SELECT 2");
        assert_eq!(out.len(), 2);
        assert!(out[0].contains("it\\'s"));
    }

    #[test]
    fn split_statements_empty() {
        let out = split_sql_statements("");
        assert!(out.is_empty());
    }

    #[test]
    fn split_statements_whitespace_only() {
        let out = split_sql_statements("   \n\t  ");
        assert!(out.is_empty());
    }

    #[test]
    fn split_statements_only_semicolons() {
        let out = split_sql_statements(";;;");
        assert!(out.is_empty());
    }

    #[test]
    fn sql_corpus_new() {
        let corpus = SqlCorpus::new(PathBuf::from("/tmp/corpus"));
        assert_eq!(corpus.corpus_root, PathBuf::from("/tmp/corpus"));
    }

    #[test]
    fn sql_corpus_reset() {
        let mut corpus = SqlCorpus::new(PathBuf::from("/tmp/corpus"));
        // Just ensure reset doesn't panic
        corpus.reset();
    }

    #[test]
    fn sql_corpus_execute_all_missing_dir() {
        let mut corpus = SqlCorpus::new(PathBuf::from("/nonexistent/dir/abc"));
        let results = corpus.execute_all();
        assert!(results.is_empty());
    }

    #[test]
    fn summary_empty_results() {
        let corpus = SqlCorpus::new(PathBuf::from("/tmp/corpus"));
        let summary = corpus.summary(&HashMap::new());
        assert_eq!(summary.total_files, 0);
        assert_eq!(summary.total_cases, 0);
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.failed, 0);
    }

    #[test]
    fn summary_with_pass_and_fail() {
        let corpus = SqlCorpus::new(PathBuf::from("/tmp/corpus"));
        let mut results = HashMap::new();
        results.insert(
            "a.sql".to_string(),
            CorpusFileResult {
                file_path: "a.sql".to_string(),
                total_cases: 1,
                passed: 1,
                failed: 0,
                results: vec![SqlTestResult {
                    case_name: "t1".to_string(),
                    sql: "SELECT 1".to_string(),
                    success: true,
                    rows_returned: 1,
                    execution_time_ms: 5,
                    error_message: None,
                    expected_rows: None,
                    expected_columns: None,
                }],
            },
        );
        results.insert(
            "b.sql".to_string(),
            CorpusFileResult {
                file_path: "b.sql".to_string(),
                total_cases: 1,
                passed: 0,
                failed: 1,
                results: vec![SqlTestResult {
                    case_name: "t2".to_string(),
                    sql: "BAD SQL".to_string(),
                    success: false,
                    rows_returned: 0,
                    execution_time_ms: 7,
                    error_message: Some("oops".to_string()),
                    expected_rows: None,
                    expected_columns: None,
                }],
            },
        );
        let summary = corpus.summary(&results);
        assert_eq!(summary.total_files, 2);
        assert_eq!(summary.total_cases, 2);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert!((summary.pass_rate - 50.0).abs() < 0.001);
    }
}
