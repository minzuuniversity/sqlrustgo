//! ExecutionEngine - high-level SQL execution API
//! Provides a simple interface for executing SQL statements against a storage backend.

#![allow(unused_variables, unused_imports)]

use crate::engine_utils::{
    build_aggregate_schema, build_combined_schema, eval_predicate, evaluate_where_clause,
    find_column_index, sql_compare, validate_foreign_keys,
};
use crate::expr_utils::{
    compare_values, evaluate_binary_op, evaluate_expr_to_string, evaluate_expression,
    expression_to_string, expression_to_value, expression_to_value_from_string,
};
use crate::{parse, SqlError, SqlResult, Value};
use sqlrustgo_catalog::stored_proc::{ParamMode, StoredProcParam, StoredProcStatement};
use sqlrustgo_catalog::{auth::UserIdentity, Catalog, StoredProcedure};
use sqlrustgo_executor::ast_adapter::AstAdapter;
use sqlrustgo_executor::stored_proc::StoredProcExecutor;
use sqlrustgo_executor::trigger::{
    TriggerEvent as ExecTriggerEvent, TriggerExecutor, TriggerTiming as ExecTriggerTiming,
};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{
    AggregateCall, AggregateFunction, AlterTableOperation, AlterTableStatement, CallStatement,
    CreateIndexStatement, CreateProcedureStatement, CreateRoleStatement, CreateTableStatement,
    CreateTriggerStatement, DropRoleStatement, DropTableStatement, GrantRoleStatement,
    GrantStatement, InsertStatement, ObjectType as ParserObjectType, Privilege as ParserPrivilege,
    RevokeRoleStatement, RevokeStatement, SelectStatement, SetRoleStatement, ShowStatement,
    StoredProcParam as ParserStoredProcParam, StoredProcParamMode as ParserParamMode,
    StoredProcStatement as ParserStatement, TruncateStatement,
};
use sqlrustgo_parser::transaction::IsolationLevel as ParserIsolationLevel;
use sqlrustgo_parser::JoinType;
use sqlrustgo_parser::{
    DeleteStatement, Expression, Statement, TransactionStatement, UpdateStatement,
};
use sqlrustgo_storage::checkpoint::{CheckpointManager, CheckpointMetadata};
use sqlrustgo_storage::{
    recovery_engine::{RecoveryEngine, RecoveryEngineImpl},
    ColumnDefinition, FileBackedWalManager, FileStorage, MemoryStorage, StorageEngine, TableInfo,
    WalStorage,
};
use sqlrustgo_transaction::{IsolationLevel as TmIsolationLevel, TransactionManager, TxId};
use sqlrustgo_types::Value as SqlValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Execution engine for SQL statements
pub struct ExecutionEngine<S: StorageEngine> {
    pub(crate) storage: Arc<RwLock<S>>,
    pub(crate) catalog: Option<Arc<RwLock<Catalog>>>,
    pub(crate) stats: Arc<RwLock<ExecutionStats>>,
    pub(crate) cbo_enabled: bool,
    pub(crate) transaction_manager: TransactionManager,
    pub(crate) current_tx_id: Option<TxId>,
    pub(crate) tx_status: TxStatus,
    pub(crate) default_isolation: TmIsolationLevel,
    pub(crate) current_role: Option<String>,
    /// CheckpointManager field — reserved for future PR-830F WAL lifecycle
    /// integration (currently set to None in all engine builders).
    /// PR-830F lifecycle methods were removed in SPEC-002; the field is
    /// kept for future re-introduction without changing the public struct layout.
    #[allow(dead_code)]
    pub(crate) checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>,
}

/// Transaction status for lifecycle enforcement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    Idle,      // No transaction started
    Active,    // Transaction in progress
    Committed, // Transaction committed (terminal)
    Aborted,   // Transaction rolled back (terminal)
}

/// Execution statistics for CBO
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    pub table_stats: HashMap<String, TableStatistics>,
}

/// Table-level statistics for query optimization
#[derive(Debug, Clone)]
pub struct TableStatistics {
    pub row_count: u64,
    pub column_stats: HashMap<String, ColumnStatistics>,
}

/// Column-level statistics
#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub null_count: u64,
    pub distinct_count: u64,
    pub min_value: Option<SqlValue>,
    pub max_value: Option<SqlValue>,
}

/// Type alias for MemoryStorage-backed execution engine
pub type MemoryExecutionEngine = ExecutionEngine<MemoryStorage>;

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    /// Create a new execution engine with CBO enabled by default
    pub fn new(storage: Arc<RwLock<S>>) -> Self {
        Self {
            storage,
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            checkpoint_manager: None,
        }
    }

    /// Create a new execution engine with CBO configuration
    pub fn with_cbo(storage: Arc<RwLock<S>>, cbo_enabled: bool) -> Self {
        Self {
            storage,
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            checkpoint_manager: None,
        }
    }

    /// Create a new execution engine with a catalog
    pub fn with_catalog(storage: Arc<RwLock<S>>, catalog: Arc<RwLock<Catalog>>) -> Self {
        Self {
            storage,
            catalog: Some(catalog),
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            checkpoint_manager: None,
        }
    }

    /// Check if CBO is enabled
    pub fn is_cbo_enabled(&self) -> bool {
        self.cbo_enabled
    }

    /// Enable or disable CBO
    pub fn set_cbo_enabled(&mut self, enabled: bool) {
        self.cbo_enabled = enabled;
    }

    /// Get table statistics for CBO
    pub fn get_table_stats(&self) -> Arc<RwLock<ExecutionStats>> {
        self.stats.clone()
    }

    // CBO estimation methods extracted to cbo_estimator.rs (SPEC-012).
    // Thin forwarder methods retained for backwards-compatible public API.

    /// Estimate the number of rows returned by a query based on statistics
    pub fn estimate_row_count(&self, table_name: &str) -> u64 {
        crate::cbo_estimator::estimate_row_count(&self.stats, table_name)
    }

    /// Estimate the selectivity of a predicate
    pub fn estimate_selectivity(&self, table_name: &str, column_name: &str) -> f64 {
        crate::cbo_estimator::estimate_selectivity(&self.stats, table_name, column_name)
    }

    /// Estimate the cost of a sequential scan
    pub fn estimate_seq_scan_cost(&self, table_name: &str) -> f64 {
        crate::cbo_estimator::estimate_seq_scan_cost(&self.stats, table_name)
    }

    /// Estimate the cost of an index scan
    pub fn estimate_index_scan_cost(&self, table_name: &str, selectivity: f64) -> f64 {
        crate::cbo_estimator::estimate_index_scan_cost(&self.stats, table_name, selectivity)
    }

    /// Estimate the benefit of using an index vs sequential scan
    pub fn estimate_index_benefit(&self, table_name: &str, selectivity: f64) -> f64 {
        crate::cbo_estimator::estimate_index_benefit(&self.stats, table_name, selectivity)
    }

    /// Decide whether to use index scan
    pub fn should_use_index(&self, table_name: &str, column_name: &str) -> bool {
        crate::cbo_estimator::should_use_index(&self.stats, table_name, column_name)
    }

    /// Estimate the cost of a join between two tables
    pub fn estimate_join_cost(&self, left_table: &str, right_table: &str, join_type: &str) -> f64 {
        crate::cbo_estimator::estimate_join_cost(&self.stats, left_table, right_table, join_type)
    }

    /// Find the optimal join order
    pub fn optimize_join_order<'a>(&self, tables: &'a [&str]) -> Vec<&'a str> {
        crate::cbo_estimator::optimize_join_order(&self.stats, tables)
    }

    // collect_table_stats extracted to cbo_estimator.rs (SPEC-012).
    // Forwarder retained for backwards-compatible call sites.
    fn collect_table_stats(&self, table: &str) -> SqlResult<TableStatistics> {
        let storage = self.storage.read().unwrap();
        crate::cbo_estimator::collect_table_stats(&*storage, table)
    }

    /// Execute a SQL statement and return results
    pub fn execute(&mut self, sql: &str) -> SqlResult<ExecutorResult> {
        let statement = parse(sql).map_err(|e| SqlError::ParseError(e.to_string()))?;

        match statement {
            Statement::Select(ref select) => self.execute_select(select),
            Statement::Insert(ref insert) => self.execute_insert(insert),
            Statement::Update(ref update) => self.execute_update(update),
            Statement::Delete(ref delete) => self.execute_delete(delete),
            Statement::CreateTable(ref create) => self.execute_create_table(create),
            Statement::DropTable(ref drop) => self.execute_drop_table(drop),
            Statement::Truncate(ref truncate) => self.execute_truncate(truncate),
            Statement::CreateIndex(ref idx) => self.execute_create_index(idx),
            Statement::Analyze(ref analyze) => {
                let table_name = analyze.table_name.as_ref().ok_or_else(|| {
                    SqlError::ExecutionError("ANALYZE: table name is required".to_string())
                })?;
                let stats = self.collect_table_stats(table_name)?;
                let row_count = stats.row_count;

                let mut stats_guard = self.stats.write().unwrap();
                stats_guard.table_stats.insert(table_name.clone(), stats);

                Ok(ExecutorResult::new(
                    vec![vec![Value::Integer(row_count as i64)]],
                    1,
                ))
            }
            Statement::Union(ref union_stmt) => {
                // Extract left and right SelectStatements from the Union
                let left_select = match union_stmt.left.as_ref() {
                    Statement::Select(s) => s,
                    _ => {
                        return Err(SqlError::ExecutionError(
                            "UNION left side must be a SELECT".to_string(),
                        ))
                    }
                };
                let right_select = match union_stmt.right.as_ref() {
                    Statement::Select(s) => s,
                    _ => {
                        return Err(SqlError::ExecutionError(
                            "UNION right side must be a SELECT".to_string(),
                        ))
                    }
                };

                let mut left_result = self.execute_select(left_select)?;
                let right_result = self.execute_select(right_select)?;

                // Append rows from right to left
                left_result.rows.extend(right_result.rows);

                // If not UNION ALL, deduplicate
                if !union_stmt.union_all {
                    left_result.rows.sort();
                    left_result.rows.dedup();
                }

                left_result.affected_rows = left_result.rows.len();
                Ok(left_result)
            }
            Statement::CreateTrigger(ref create_trigger) => {
                self.execute_create_trigger(create_trigger)
            }
            Statement::Call(ref call) => self.execute_call(call),
            Statement::CreateProcedure(ref create_proc) => {
                self.execute_create_procedure(create_proc)
            }
            Statement::Transaction(ref txn) => self.execute_transaction(txn),
            Statement::Grant(ref grant) => self.execute_grant(grant),
            Statement::Revoke(ref revoke) => self.execute_revoke(revoke),
            Statement::CreateRole(ref stmt) => self.execute_create_role(stmt),
            Statement::DropRole(ref stmt) => self.execute_drop_role(stmt),
            Statement::GrantRole(ref stmt) => self.execute_grant_role(stmt),
            Statement::RevokeRole(ref stmt) => self.execute_revoke_role(stmt),
            Statement::SetRole(ref stmt) => self.execute_set_role(stmt),
            Statement::ShowRoles => self.execute_show_roles(),
            Statement::ShowGrantsFor(ref user) => self.execute_show_grants_for(user),
            Statement::Show(ref show) => self.execute_show(show),
            Statement::AlterTable(ref alter) => self.execute_alter_table(alter),
            _ => Err(SqlError::ExecutionError(
                "Unsupported statement type".to_string(),
            )),
        }
    }

    fn execute_insert(&mut self, insert: &InsertStatement) -> SqlResult<ExecutorResult> {
        // IMPL-001 & IMPL-004: TX lifecycle enforcement
        // IDLE/Active with no current_tx_id = implicit autocommit TX (allowed)
        // Committed/Aborted state = no new implicit TX (error)
        match self.tx_status {
            TxStatus::Committed => {
                return Err(SqlError::ExecutionError(
                    "transaction already committed".to_string(),
                ));
            }
            TxStatus::Aborted => {
                return Err(SqlError::ExecutionError(
                    "transaction already aborted".to_string(),
                ));
            }
            TxStatus::Idle | TxStatus::Active => {
                // Autocommit: allow DML without explicit BEGIN
                // New implicit TX started implicitly when current_tx_id is None
            }
        }
        // INT-1: Force DML to go through TransactionManager.
        // Begin an implicit transaction if none is active, and ensure it
        // commits when the DML finishes (autocommit semantics).
        // This call site now matches the documented DML contract: every
        // INSERT/UPDATE/DELETE must be wrapped by TM.begin_transaction() / TM.commit().
        let tm_tx_id = if self.current_tx_id.is_none() {
            let tx_id = self
                .transaction_manager
                .begin_transaction(self.default_isolation)
                .map_err(|e| SqlError::ExecutionError(format!("TM.begin failed: {:?}", e)))?;
            self.current_tx_id = Some(tx_id);
            self.tx_status = TxStatus::Active;
            Some(tx_id)
        } else {
            self.current_tx_id
        };
        let table_name = insert.table.clone();

        // Get table info first (need it for triggers and FK validation)
        let table_info = {
            let storage = self.storage.read().unwrap();
            storage.get_table_info(&table_name)?.clone()
        };

        // Convert expressions to records
        let mut all_records: Vec<Vec<Value>> = Vec::new();
        for row_exprs in &insert.values {
            let mut record = Vec::with_capacity(row_exprs.len());
            for (_i, expr) in row_exprs.iter().enumerate() {
                let val = expression_to_value(expr);
                record.push(val);
            }
            all_records.push(record);
        }

        // For REPLACE INTO: if insert.values has a unique/key conflict, delete old row first
        if insert.is_replace {
            {
                let mut storage = self.storage.write().unwrap();
                for record in &all_records {
                    // Find existing rows with matching unique key (primary key or unique index)
                    let existing_rows = storage.scan(&table_name)?;
                    for existing_row in existing_rows {
                        if self.record_matches_unique_key(&existing_row, record, &table_info) {
                            // Delete the existing row
                            storage.delete(&table_name, &[])?;
                            break;
                        }
                    }
                }
            }
        }

        // Execute BEFORE INSERT triggers
        let trigger_executor = TriggerExecutor::new(self.storage.clone());
        let before_triggers = trigger_executor.get_triggers_for_operation(
            &table_name,
            ExecTriggerTiming::Before,
            ExecTriggerEvent::Insert,
        );

        let processed_records: Vec<Vec<Value>> = if !before_triggers.is_empty() {
            let mut processed = Vec::new();
            for record in &all_records {
                let modified = trigger_executor.execute_before_insert(&table_name, record)?;
                processed.push(modified);
            }
            processed
        } else {
            all_records.clone()
        };

        // Validate FK and CHECK constraints, then insert
        {
            let mut storage = self.storage.write().unwrap();
            let col_names: Vec<String> =
                table_info.columns.iter().map(|c| c.name.clone()).collect();
            for record in &processed_records {
                if !table_info.foreign_keys.is_empty() {
                    validate_foreign_keys(&*storage, &table_info, record, &insert.columns)?;
                }
                // Validate CHECK constraints
                if !table_info.check_constraints.is_empty() {
                    for constraint in &table_info.check_constraints {
                        let valid = sqlrustgo_storage::evaluate_check_constraint(
                            constraint, &col_names, record,
                        )?;
                        if !valid {
                            return Err(format!(
                                "CHECK constraint '{}' violated: {}",
                                constraint.name.as_deref().unwrap_or("unnamed"),
                                constraint.expression
                            )
                            .into());
                        }
                    }
                }
            }
            storage.insert(&table_name, processed_records)?;
        }

        // Execute AFTER INSERT triggers
        let after_triggers = trigger_executor.get_triggers_for_operation(
            &table_name,
            ExecTriggerTiming::After,
            ExecTriggerEvent::Insert,
        );

        if !after_triggers.is_empty() {
            for record in &all_records {
                trigger_executor.execute_after_insert(&table_name, record)?;
            }
        }

        // INT-1: For autocommit (no explicit transaction), commit the
        // implicit transaction so WAL/MVCC receive the changes. If the
        // user already started a transaction via BEGIN, leave the
        // current_tx_id intact so they can COMMIT/ROLLBACK explicitly.
        // `tm_tx_id == Some(_)` only when we just opened a new implicit
        // transaction in this function (the if-branch above); the
        // else-branch returns the existing TxId for an already-open
        // transaction, in which case the user controls commit/rollback.
        if self.current_tx_id.is_some() && self.current_tx_id == tm_tx_id {
            let tx_id = self.current_tx_id.unwrap();
            let _ = self.transaction_manager.commit(tx_id);
            self.current_tx_id = None;
            self.tx_status = TxStatus::Idle;
        }

        Ok(ExecutorResult::new(vec![], insert.values.len()))
    }

    /// Check if a new record matches an existing row based on primary key or unique constraints
    fn record_matches_unique_key(
        &self,
        existing: &[Value],
        new: &[Value],
        table_info: &TableInfo,
    ) -> bool {
        // Find primary key column(s)
        for (col_idx, col) in table_info.columns.iter().enumerate() {
            if col.primary_key {
                // Compare by primary key column index
                if col_idx < existing.len() && col_idx < new.len() {
                    if existing[col_idx] != new[col_idx] {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }

    fn execute_update(&mut self, update: &UpdateStatement) -> SqlResult<ExecutorResult> {
        // IMPL-001 & IMPL-004: TX lifecycle enforcement
        match self.tx_status {
            TxStatus::Committed => {
                return Err(SqlError::ExecutionError(
                    "transaction already committed".to_string(),
                ));
            }
            TxStatus::Aborted => {
                return Err(SqlError::ExecutionError(
                    "transaction already aborted".to_string(),
                ));
            }
            TxStatus::Idle | TxStatus::Active => {
                // Autocommit: allow DML without explicit BEGIN
            }
        }
        // INT-1: Begin an implicit transaction so TM/WAL receive the change.
        let tm_tx_id = if self.current_tx_id.is_none() {
            let tx_id = self
                .transaction_manager
                .begin_transaction(self.default_isolation)
                .map_err(|e| SqlError::ExecutionError(format!("TM.begin failed: {:?}", e)))?;
            self.current_tx_id = Some(tx_id);
            self.tx_status = TxStatus::Active;
            Some(tx_id)
        } else {
            self.current_tx_id
        };
        let table_name = update.table.clone();

        // If no WHERE clause, use the simple storage.update() path
        // PR-842 Option A: compute updates from SET clauses (per-row evaluation
        // collapses to a single value for literal / constant expressions, which
        // is the common no-WHERE case). For column references, the first row
        // is used as the evaluation context — for literal values this yields
        // the correct after-image for every row.
        if update.where_clause.is_none() {
            let table_info = {
                let storage = self.storage.read().unwrap();
                storage.get_table_info(&table_name)?.clone()
            };
            let sample_row: Vec<sqlrustgo_types::Value> = {
                let storage = self.storage.read().unwrap();
                storage
                    .scan(&table_name)
                    .ok()
                    .and_then(|rows| rows.first().cloned())
                    .unwrap_or_else(|| {
                        table_info
                            .columns
                            .iter()
                            .map(|_| sqlrustgo_types::Value::Null)
                            .collect()
                    })
            };
            let updates: Vec<(usize, sqlrustgo_types::Value)> = update
                .set_clauses
                .iter()
                .filter_map(|(col_name, expr)| {
                    let col_idx = find_column_index(col_name, &table_info)?;
                    let new_val = evaluate_expression(expr, &sample_row, &table_info)
                        .unwrap_or(sqlrustgo_types::Value::Null);
                    Some((col_idx, new_val))
                })
                .collect();
            let mut storage = self.storage.write().unwrap();
            let count = storage.update(&table_name, &[], &updates)?;
            return Ok(ExecutorResult::new(vec![], count));
        }

        // Get table info and scan rows
        let table_info = {
            let storage = self.storage.read().unwrap();
            storage.get_table_info(&table_name)?.clone()
        };

        let all_rows = {
            let storage = self.storage.read().unwrap();
            storage.scan(&table_name)?
        };

        let where_clause = update.where_clause.as_ref().unwrap();

        // Filter rows that match the WHERE clause
        let rows_to_update: Vec<Vec<Value>> = all_rows
            .clone()
            .into_iter()
            .filter(|row| evaluate_where_clause(where_clause, row, &table_info))
            .collect();

        let update_plan = AstAdapter::to_update_plan(update, &table_info);
        if let Ok(plan) = update_plan {
            let ir_filtered: Vec<Vec<Value>> = all_rows
                .clone()
                .into_iter()
                .filter(|row| plan.predicate().evaluate(row, &table_info))
                .collect();

            if ir_filtered.len() != rows_to_update.len() {
                eprintln!(
                    "[IR VALIDATION] Predicate mismatch: legacy={}, ir={}",
                    rows_to_update.len(),
                    ir_filtered.len()
                );
            }
        }

        let count = rows_to_update.len();

        if count == 0 {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        // Build column index map for SET clauses
        let set_col_indices: Vec<(usize, &Expression)> = update
            .set_clauses
            .iter()
            .filter_map(|(col_name, expr)| {
                find_column_index(col_name, &table_info).map(|idx| (idx, expr))
            })
            .collect();

        // Apply SET expressions to each matching row
        let updated_rows: Vec<Vec<Value>> = rows_to_update
            .iter()
            .map(|row| {
                let mut new_row = row.clone();
                for &(col_idx, ref set_expr) in &set_col_indices {
                    let new_val =
                        evaluate_expression(set_expr, &new_row, &table_info).unwrap_or(Value::Null);
                    if col_idx < new_row.len() {
                        new_row[col_idx] = new_val;
                    }
                }
                new_row
            })
            .collect();

        // Execute BEFORE UPDATE triggers
        let trigger_executor = TriggerExecutor::new(self.storage.clone());
        let before_triggers = trigger_executor.get_triggers_for_operation(
            &table_name,
            ExecTriggerTiming::Before,
            ExecTriggerEvent::Update,
        );

        let trigger_modified_rows: Vec<Vec<Value>> = if !before_triggers.is_empty() {
            let mut modified = Vec::new();
            for (i, updated_row) in updated_rows.iter().enumerate() {
                let old_row = &rows_to_update[i];
                let result =
                    trigger_executor.execute_before_update(&table_name, old_row, updated_row)?;
                modified.push(result);
            }
            modified
        } else {
            updated_rows.clone()
        };

        let mut new_rows: Vec<Vec<Value>> = Vec::new();

        // Build new_rows by replacing matching rows with updated versions
        for row in &all_rows {
            if let Some(pos) = rows_to_update.iter().position(|r| r == row) {
                new_rows.push(trigger_modified_rows[pos].clone());
            } else {
                new_rows.push(row.clone());
            }
        }

        {
            let mut storage = self.storage.write().unwrap();

            if !table_info.check_constraints.is_empty() {
                let col_names: Vec<String> =
                    table_info.columns.iter().map(|c| c.name.clone()).collect();
                for record in &new_rows {
                    for constraint in &table_info.check_constraints {
                        let valid = sqlrustgo_storage::evaluate_check_constraint(
                            constraint, &col_names, record,
                        )?;
                        if !valid {
                            return Err(format!(
                                "CHECK constraint '{}' violated: {}",
                                constraint.name.as_deref().unwrap_or("unnamed"),
                                constraint.expression
                            )
                            .into());
                        }
                    }
                }
            }

            storage.delete(&table_name, &[])?;
            if !new_rows.is_empty() {
                storage.insert(&table_name, new_rows)?;
            }
        }

        // Execute AFTER UPDATE triggers
        let after_triggers = trigger_executor.get_triggers_for_operation(
            &table_name,
            ExecTriggerTiming::After,
            ExecTriggerEvent::Update,
        );

        if !after_triggers.is_empty() {
            for (i, updated_row) in updated_rows.iter().enumerate() {
                let old_row = &rows_to_update[i];
                trigger_executor.execute_after_update(&table_name, old_row, updated_row)?;
            }
        }

        // INT-1: For autocommit (no explicit transaction), commit so
        // WAL/MVCC receive the change. If user already started a TX,
        // leave current_tx_id intact for explicit COMMIT/ROLLBACK.
        if self.current_tx_id.is_some() && self.current_tx_id == tm_tx_id {
            let tx_id = self.current_tx_id.unwrap();
            let _ = self.transaction_manager.commit(tx_id);
            self.current_tx_id = None;
            self.tx_status = TxStatus::Idle;
        }

        Ok(ExecutorResult::new(vec![], count))
    }

    fn execute_delete(&mut self, delete: &DeleteStatement) -> SqlResult<ExecutorResult> {
        // IMPL-001 & IMPL-004: TX lifecycle enforcement
        match self.tx_status {
            TxStatus::Committed => {
                return Err(SqlError::ExecutionError(
                    "transaction already committed".to_string(),
                ));
            }
            TxStatus::Aborted => {
                return Err(SqlError::ExecutionError(
                    "transaction already aborted".to_string(),
                ));
            }
            TxStatus::Idle | TxStatus::Active => {
                // Autocommit: allow DML without explicit BEGIN
            }
        }
        // INT-1: Begin an implicit transaction so TM/WAL receive the change.
        let tm_tx_id = if self.current_tx_id.is_none() {
            let tx_id = self
                .transaction_manager
                .begin_transaction(self.default_isolation)
                .map_err(|e| SqlError::ExecutionError(format!("TM.begin failed: {:?}", e)))?;
            self.current_tx_id = Some(tx_id);
            self.tx_status = TxStatus::Active;
            Some(tx_id)
        } else {
            self.current_tx_id
        };
        let table_name = delete.table.clone();

        // If no WHERE clause, delete all rows (current behavior is correct)
        if delete.where_clause.is_none() {
            let mut storage = self.storage.write().unwrap();
            let count = storage.delete(&table_name, &[])?;
            // INT-1: Autocommit — commit the implicit TX so WAL/MVCC see this.
            if self.current_tx_id.is_some() && self.current_tx_id == tm_tx_id {
                let tx_id = self.current_tx_id.unwrap();
                let _ = self.transaction_manager.commit(tx_id);
                self.current_tx_id = None;
                self.tx_status = TxStatus::Idle;
            }
            return Ok(ExecutorResult::new(vec![], count));
        }

        // Scan all rows from the table
        let all_rows = {
            let storage = self.storage.read().unwrap();
            storage.scan(&table_name)?
        };

        // Get table info to find column indices
        let table_info = {
            let storage = self.storage.read().unwrap();
            storage.get_table_info(&table_name)?.clone()
        };

        // Filter rows based on WHERE clause
        let where_clause = delete.where_clause.as_ref().unwrap();
        let rows_to_delete: Vec<Vec<Value>> = all_rows
            .into_iter()
            .filter(|row| evaluate_where_clause(where_clause, row, &table_info))
            .collect();

        let count = rows_to_delete.len();

        if count == 0 {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        // Execute BEFORE DELETE triggers
        let trigger_executor = TriggerExecutor::new(self.storage.clone());
        let before_triggers = trigger_executor.get_triggers_for_operation(
            &table_name,
            ExecTriggerTiming::Before,
            ExecTriggerEvent::Delete,
        );

        if !before_triggers.is_empty() {
            for row in &rows_to_delete {
                trigger_executor.execute_before_delete(&table_name, row)?;
            }
        }

        // PR-842: prefer row-level deletes so WAL records one Delete entry
        // per matching row and recovery can replay them without losing the
        // pre-delete buffer state. We still call `storage.delete(table, &[])`
        // to clear out buffered rows that did not match the WHERE clause.
        let rows_to_keep: Vec<Vec<Value>> = {
            let storage = self.storage.read().unwrap();
            let all_rows = storage.scan(&table_name)?;
            all_rows
                .into_iter()
                .filter(|row| !evaluate_where_clause(where_clause, row, &table_info))
                .collect()
        };

        {
            let mut storage = self.storage.write().unwrap();
            // First drop the full table to flush any buffered inserts and
            // to provide a clean slate (this is what the legacy code did).
            storage.delete(&table_name, &[])?;
            if !rows_to_keep.is_empty() {
                storage.insert(&table_name, rows_to_keep)?;
            }
            // Then delete the matching rows from the freshly re-inserted set
            // so WAL records one Delete entry per affected row.
            //
            // FIX-2737: Extract ONLY primary key column values for delete,
            // not all columns. storage.delete() does full row comparison when
            // key_values is non-empty, so passing all columns causes delete to
            // fail if any non-PK column differs (e.g., due to serialization).
            let pk_indices: Vec<usize> = table_info
                .columns
                .iter()
                .enumerate()
                .filter(|(_, col)| col.primary_key)
                .map(|(i, _)| i)
                .collect();

            // If table has primary keys, use only PK columns for delete.
            // Otherwise, fall back to all columns (backward compatible).
            let use_indices: Vec<usize> = if pk_indices.is_empty() {
                (0..rows_to_delete[0].len()).collect()
            } else {
                pk_indices
            };

            for row in &rows_to_delete {
                let key_values: Vec<Value> = use_indices
                    .iter()
                    .map(|&i| row.get(i).cloned().unwrap_or(sqlrustgo_types::Value::Null))
                    .collect();
                storage.delete(&table_name, &key_values)?;
            }
        }

        // Execute AFTER DELETE triggers
        let after_triggers = trigger_executor.get_triggers_for_operation(
            &table_name,
            ExecTriggerTiming::After,
            ExecTriggerEvent::Delete,
        );

        if !after_triggers.is_empty() {
            for row in &rows_to_delete {
                trigger_executor.execute_after_delete(&table_name, row)?;
            }
        }

        // INT-1: For autocommit (no explicit transaction), commit so
        // WAL/MVCC receive the change. If user already started a TX,
        // leave current_tx_id intact for explicit COMMIT/ROLLBACK.
        if self.current_tx_id.is_some() && self.current_tx_id == tm_tx_id {
            let tx_id = self.current_tx_id.unwrap();
            let _ = self.transaction_manager.commit(tx_id);
            self.current_tx_id = None;
            self.tx_status = TxStatus::Idle;
        }

        Ok(ExecutorResult::new(vec![], count))
    }

    fn execute_create_table(&self, create: &CreateTableStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write().unwrap();
        let columns: Vec<ColumnDefinition> = create
            .columns
            .iter()
            .map(|c| ColumnDefinition {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                nullable: !c.primary_key,
                primary_key: c.primary_key,
            })
            .collect();
        let info = TableInfo {
            name: create.name.clone(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
        };
        storage.create_table(&info)?;
        Ok(ExecutorResult::empty())
    }

    fn execute_drop_table(&self, drop: &DropTableStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write().unwrap();
        storage.drop_table(&drop.name)?;
        Ok(ExecutorResult::empty())
    }

    fn execute_truncate(&self, truncate: &TruncateStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write().unwrap();
        // Check if table exists
        if !storage.has_table(&truncate.name) {
            return Err(SqlError::ExecutionError(format!(
                "Table not found: {}",
                truncate.name
            )));
        }
        // Delete all rows but keep the table structure
        // Using empty filter slice to delete all rows
        storage.delete(&truncate.name, &[])?;
        Ok(ExecutorResult::empty())
    }

    fn execute_create_index(&self, idx: &CreateIndexStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write().unwrap();
        let table_name = &idx.table;
        let col_name = idx
            .columns
            .first()
            .ok_or_else(|| SqlError::ExecutionError("No columns in index".to_string()))?;
        let table_info = storage.get_table_info(table_name)?;
        let col_idx = table_info
            .columns
            .iter()
            .position(|c| c.name == *col_name)
            .ok_or_else(|| SqlError::ExecutionError("Column not found".to_string()))?;
        storage.create_index(table_name, col_name, col_idx)?;
        Ok(ExecutorResult::empty())
    }

    fn execute_create_trigger(&self, stmt: &CreateTriggerStatement) -> SqlResult<ExecutorResult> {
        use sqlrustgo_storage::engine::{TriggerEvent, TriggerInfo, TriggerTiming};

        let mut storage = self.storage.write().unwrap();
        let timing = match stmt.timing.to_uppercase().as_str() {
            "BEFORE" => TriggerTiming::Before,
            "AFTER" => TriggerTiming::After,
            _ => {
                return Err(SqlError::ExecutionError(format!(
                    "Invalid trigger timing: {}",
                    stmt.timing
                )))
            }
        };
        // Handle first event (triggers support one event per trigger in storage)
        let event_str = stmt
            .events
            .first()
            .ok_or_else(|| SqlError::ExecutionError("No trigger event specified".to_string()))?;
        let event = match event_str.to_uppercase().as_str() {
            "INSERT" => TriggerEvent::Insert,
            "UPDATE" => TriggerEvent::Update,
            "DELETE" => TriggerEvent::Delete,
            _ => {
                return Err(SqlError::ExecutionError(format!(
                    "Invalid trigger event: {}",
                    event_str
                )))
            }
        };
        let trigger_info = TriggerInfo {
            name: stmt.name.clone(),
            table_name: stmt.table.clone(),
            timing,
            event,
            body: stmt.body.clone(),
        };
        storage.create_trigger(trigger_info)?;
        Ok(ExecutorResult::empty())
    }

    fn execute_call(&self, call: &CallStatement) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError("CALL statement requires stored procedure catalog".to_string())
        })?;
        let catalog = catalog_guard.read().unwrap();

        let procedure = catalog
            .get_stored_procedure(&call.procedure_name)
            .ok_or_else(|| {
                SqlError::ExecutionError(format!(
                    "Stored procedure '{}' not found",
                    call.procedure_name
                ))
            })?;

        let executor = StoredProcExecutor::new(Arc::new(catalog.clone()), self.storage.clone());

        let args: Vec<Value> = call
            .args
            .iter()
            .map(|arg| expression_to_value_from_string(arg))
            .collect();

        let result = executor
            .execute_call(&call.procedure_name, args)
            .map_err(SqlError::ExecutionError)?;

        Ok(result)
    }

    fn execute_create_procedure(
        &self,
        stmt: &CreateProcedureStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError(
                "CREATE PROCEDURE requires stored procedure catalog".to_string(),
            )
        })?;
        let mut catalog = catalog_guard.write().unwrap();

        let params: Vec<sqlrustgo_catalog::stored_proc::StoredProcParam> = stmt
            .params
            .iter()
            .map(|p| {
                let mode = match p.mode {
                    ParserParamMode::In => ParamMode::In,
                    ParserParamMode::Out => ParamMode::Out,
                    ParserParamMode::InOut => ParamMode::InOut,
                };
                sqlrustgo_catalog::stored_proc::StoredProcParam {
                    name: p.name.clone(),
                    mode,
                    data_type: p.data_type.clone(),
                }
            })
            .collect();

        let body: Vec<StoredProcStatement> = stmt
            .body
            .iter()
            .map(|s| match s {
                ParserStatement::RawSql(sql) => StoredProcStatement::RawSql(sql.clone()),
            })
            .collect();

        let procedure = StoredProcedure::new(stmt.name.clone(), params, body);

        catalog.add_stored_procedure(procedure).map_err(|e| {
            SqlError::ExecutionError(format!("Failed to create procedure: {:?}", e))
        })?;

        Ok(ExecutorResult::empty())
    }

    fn execute_transaction(&mut self, stmt: &TransactionStatement) -> SqlResult<ExecutorResult> {
        match stmt {
            TransactionStatement::Begin {
                work: _,
                isolation_level,
            } => {
                let iso = isolation_level
                    .as_ref()
                    .map(|il| match il {
                        ParserIsolationLevel::ReadCommitted => TmIsolationLevel::SnapshotIsolation,
                        ParserIsolationLevel::ReadUncommitted => {
                            TmIsolationLevel::SnapshotIsolation
                        }
                        ParserIsolationLevel::SnapshotIsolation => {
                            TmIsolationLevel::SnapshotIsolation
                        }
                        ParserIsolationLevel::Serializable => TmIsolationLevel::Serializable,
                    })
                    .unwrap_or(self.default_isolation);
                self.begin_transaction(iso)
            }
            TransactionStatement::Commit { work: _ } => self.commit_transaction(),
            TransactionStatement::Rollback { work: _ } => self.rollback_transaction(),
            TransactionStatement::SetTransaction { isolation_level } => {
                self.default_isolation = match isolation_level {
                    ParserIsolationLevel::ReadCommitted => TmIsolationLevel::SnapshotIsolation,
                    ParserIsolationLevel::ReadUncommitted => TmIsolationLevel::SnapshotIsolation,
                    ParserIsolationLevel::SnapshotIsolation => TmIsolationLevel::SnapshotIsolation,
                    ParserIsolationLevel::Serializable => TmIsolationLevel::Serializable,
                };
                Ok(ExecutorResult::empty())
            }
            TransactionStatement::StartTransaction { isolation_level } => {
                let iso = isolation_level
                    .as_ref()
                    .map(|il| match il {
                        ParserIsolationLevel::ReadCommitted => TmIsolationLevel::SnapshotIsolation,
                        ParserIsolationLevel::ReadUncommitted => {
                            TmIsolationLevel::SnapshotIsolation
                        }
                        ParserIsolationLevel::SnapshotIsolation => {
                            TmIsolationLevel::SnapshotIsolation
                        }
                        ParserIsolationLevel::Serializable => TmIsolationLevel::Serializable,
                    })
                    .unwrap_or(self.default_isolation);
                self.begin_transaction(iso)
            }
        }
    }

    fn begin_transaction(&mut self, isolation: TmIsolationLevel) -> SqlResult<ExecutorResult> {
        if self.current_tx_id.is_some() {
            return Err(SqlError::ExecutionError(
                "Transaction already in progress".to_string(),
            ));
        }
        let tx_id = self
            .transaction_manager
            .begin_transaction(isolation)
            .map_err(|e| {
                SqlError::ExecutionError(format!("Failed to begin transaction: {:?}", e))
            })?;
        self.current_tx_id = Some(tx_id);
        // PR-842: also write a `Begin` WAL entry so the recovery engine can
        // detect explicit transactions and apply the per-tx boundary rule
        // when filtering committed entries. Without this, every DML entry
        // appears to be autocommit and uncommitted work leaks into recovery.
        if let Ok(mut storage) = self.storage.write() {
            storage.set_current_tx_id(tx_id.as_u64());
            let _ = storage.begin_transaction();
        }
        self.tx_status = TxStatus::Active;
        Ok(ExecutorResult::new(
            vec![vec![Value::Integer(tx_id.as_u64() as i64)]],
            1,
        ))
    }

    fn commit_transaction(&mut self) -> SqlResult<ExecutorResult> {
        // IMPL-004: Double-commit prevention — check before ok_or_else (current_tx_id set to None after commit)
        if self.current_tx_id.is_none() {
            return Err(SqlError::ExecutionError(
                "transaction already committed".to_string(),
            ));
        }
        let tx_id = self
            .current_tx_id
            .ok_or_else(|| SqlError::ExecutionError("No transaction in progress".to_string()))?;
        // Delegate to storage engine first so WalStorage writes WAL Commit entry before clearing state
        if let Ok(mut storage) = self.storage.write() {
            let _ = storage.commit_transaction();
        }
        self.transaction_manager.commit(tx_id).map_err(|e| {
            SqlError::ExecutionError(format!("Failed to commit transaction: {:?}", e))
        })?;
        self.current_tx_id = None;
        self.tx_status = TxStatus::Committed;
        // INT-1: Reset to Idle after commit so the next statement can
        // either begin a new TX or run in autocommit mode again. Without
        // this reset, subsequent DML would reject with
        // "transaction already committed".
        self.tx_status = TxStatus::Idle;
        Ok(ExecutorResult::empty())
    }

    fn rollback_transaction(&mut self) -> SqlResult<ExecutorResult> {
        // IMPL-004: Double-rollback prevention — check before ok_or_else
        if self.current_tx_id.is_none() {
            return Err(SqlError::ExecutionError(
                "transaction already aborted".to_string(),
            ));
        }
        let tx_id = self
            .current_tx_id
            .ok_or_else(|| SqlError::ExecutionError("No transaction in progress".to_string()))?;
        // Delegate to storage engine first so WalStorage writes WAL Rollback entry before clearing state
        if let Ok(mut storage) = self.storage.write() {
            let _ = storage.rollback_transaction();
        }
        self.transaction_manager.rollback(tx_id).map_err(|e| {
            SqlError::ExecutionError(format!("Failed to rollback transaction: {:?}", e))
        })?;
        self.current_tx_id = None;
        self.tx_status = TxStatus::Aborted;
        // INT-1: Reset to Idle so the next DML can begin a new TX or run
        // in autocommit mode. (Same reasoning as commit_transaction above.)
        self.tx_status = TxStatus::Idle;
        Ok(ExecutorResult::empty())
    }

    fn execute_grant(&mut self, grant: &GrantStatement) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError("Catalog not available for GRANT".to_string())
        })?;
        let mut catalog = catalog_guard.write().unwrap();

        for privilege in &grant.privileges {
            let priv_str = match privilege {
                ParserPrivilege::Select => "SELECT",
                ParserPrivilege::Insert => "INSERT",
                ParserPrivilege::Update => "UPDATE",
                ParserPrivilege::Delete => "DELETE",
                ParserPrivilege::Read => "READ",
                ParserPrivilege::Write => "WRITE",
                ParserPrivilege::Execute => "EXECUTE",
                ParserPrivilege::Usage => "USAGE",
                ParserPrivilege::All => "ALL",
            };
            let priv_obj =
                sqlrustgo_catalog::auth::Privilege::from_str(priv_str).ok_or_else(|| {
                    SqlError::ExecutionError(format!("Unknown privilege: {}", priv_str))
                })?;

            let obj_type = match &grant.object_type {
                ParserObjectType::Table => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Database => sqlrustgo_catalog::auth::ObjectType::Database,
                ParserObjectType::Column => sqlrustgo_catalog::auth::ObjectType::Column,
                ParserObjectType::Procedure => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Function => sqlrustgo_catalog::auth::ObjectType::Table,
            };

            for recipient in &grant.recipients {
                let identity = sqlrustgo_catalog::auth::UserIdentity::new(recipient, "%");
                if grant.object_type == ParserObjectType::Column {
                    for column in &grant.columns {
                        catalog
                            .grant_column_privilege(&identity, priv_obj, &grant.object_name, column)
                            .map_err(|e| {
                                SqlError::ExecutionError(format!("GRANT failed: {}", e))
                            })?;
                    }
                } else {
                    catalog
                        .grant_privilege(
                            &identity,
                            priv_obj,
                            obj_type,
                            &grant.object_name,
                            grant.with_grant_option,
                        )
                        .map_err(|e| SqlError::ExecutionError(format!("GRANT failed: {}", e)))?;
                }
            }
        }

        Ok(ExecutorResult::new(
            vec![vec![Value::Integer(grant.recipients.len() as i64)]],
            1,
        ))
    }

    fn execute_revoke(&mut self, revoke: &RevokeStatement) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError("Catalog not available for REVOKE".to_string())
        })?;
        let mut catalog = catalog_guard.write().unwrap();

        for privilege in &revoke.privileges {
            let priv_str = match privilege {
                ParserPrivilege::Select => "SELECT",
                ParserPrivilege::Insert => "INSERT",
                ParserPrivilege::Update => "UPDATE",
                ParserPrivilege::Delete => "DELETE",
                ParserPrivilege::Read => "READ",
                ParserPrivilege::Write => "WRITE",
                ParserPrivilege::Execute => "EXECUTE",
                ParserPrivilege::Usage => "USAGE",
                ParserPrivilege::All => "ALL",
            };
            let priv_obj =
                sqlrustgo_catalog::auth::Privilege::from_str(priv_str).ok_or_else(|| {
                    SqlError::ExecutionError(format!("Unknown privilege: {}", priv_str))
                })?;

            let obj_type = match &revoke.object_type {
                ParserObjectType::Table => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Database => sqlrustgo_catalog::auth::ObjectType::Database,
                ParserObjectType::Column => sqlrustgo_catalog::auth::ObjectType::Column,
                ParserObjectType::Procedure => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Function => sqlrustgo_catalog::auth::ObjectType::Table,
            };

            for user in &revoke.from_users {
                let identity = sqlrustgo_catalog::auth::UserIdentity::new(user, "%");
                catalog
                    .revoke_privilege(&identity, priv_obj, obj_type, &revoke.object_name)
                    .map_err(|e| SqlError::ExecutionError(format!("REVOKE failed: {}", e)))?;
            }
        }

        Ok(ExecutorResult::new(
            vec![vec![Value::Integer(revoke.from_users.len() as i64)]],
            1,
        ))
    }

    fn execute_create_role(&mut self, stmt: &CreateRoleStatement) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write().unwrap();

        let parent_role_id = if let Some(ref parent_name) = stmt.parent_role {
            let parent_role = catalog_guard
                .auth_manager()
                .find_role_by_name(parent_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Parent role '{}' not found", parent_name))
                })?;
            Some(parent_role.id)
        } else {
            None
        };

        catalog_guard
            .create_role(&stmt.name, parent_role_id)
            .map_err(|e| SqlError::ExecutionError(format!("CREATE ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!("Role {} created", stmt.name))]],
            1,
        ))
    }

    fn execute_drop_role(&mut self, stmt: &DropRoleStatement) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write().unwrap();

        let role_id = {
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.name))
                })?;
            role.id
        };

        catalog_guard
            .drop_role(role_id)
            .map_err(|e| SqlError::ExecutionError(format!("DROP ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!("Role {} dropped", stmt.name))]],
            1,
        ))
    }

    fn execute_grant_role(&mut self, stmt: &GrantRoleStatement) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write().unwrap();

        let role_id = {
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.role_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.role_name))
                })?;
            role.id
        };

        let user_identity = UserIdentity::new(&stmt.user_name, stmt.host.as_deref().unwrap_or("%"));

        let user_id = {
            catalog_guard
                .auth_manager()
                .get_user_id_by_identity(&user_identity)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("User '{}' not found", stmt.user_name))
                })?
        };

        catalog_guard
            .grant_role_to_user(user_id, role_id, 0)
            .map_err(|e| SqlError::ExecutionError(format!("GRANT ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!(
                "Grant {} to {}",
                stmt.role_name, stmt.user_name
            ))]],
            1,
        ))
    }

    fn execute_revoke_role(&mut self, stmt: &RevokeRoleStatement) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write().unwrap();

        let role_id = {
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.role_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.role_name))
                })?;
            role.id
        };

        let user_identity = UserIdentity::new(&stmt.user_name, stmt.host.as_deref().unwrap_or("%"));

        let user_id = {
            catalog_guard
                .auth_manager()
                .get_user_id_by_identity(&user_identity)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("User '{}' not found", stmt.user_name))
                })?
        };

        catalog_guard
            .revoke_role_from_user(user_id, role_id)
            .map_err(|e| SqlError::ExecutionError(format!("REVOKE ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!(
                "Revoke {} from {}",
                stmt.role_name, stmt.user_name
            ))]],
            1,
        ))
    }

    fn execute_set_role(&mut self, stmt: &SetRoleStatement) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;

        let role_name = {
            let catalog_guard = catalog.read().unwrap();
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.role_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.role_name))
                })?;
            role.name.clone()
        };

        self.current_role = Some(stmt.role_name.clone());

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!("SET ROLE to {}", role_name))]],
            1,
        ))
    }

    fn execute_show_roles(&self) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let catalog_guard = catalog.read().unwrap();

        let roles = catalog_guard.auth_manager().list_roles();
        let rows: Vec<Vec<Value>> = roles
            .iter()
            .map(|r| {
                vec![
                    Value::Integer(r.id as i64),
                    Value::Text(r.name.clone()),
                    r.parent_role_id
                        .map(|id| Value::Integer(id as i64))
                        .unwrap_or(Value::Null),
                ]
            })
            .collect();

        Ok(ExecutorResult::new(rows, 3))
    }

    /// Dispatch `Statement::Show` to a concrete sub-handler.
    /// PR-SHOW-TABLES: P1 backlog fix for v3.7.0.
    fn execute_show(&self, show: &ShowStatement) -> SqlResult<ExecutorResult> {
        match show {
            ShowStatement::Tables => self.execute_show_tables(),
            ShowStatement::Databases => self.execute_show_databases(),
            ShowStatement::CreateTable { table } => self.execute_show_create_table(table),
            ShowStatement::Index { table } => self.execute_show_index(table),
            ShowStatement::Grants { user } => self.execute_show_grants(user.as_deref()),
            ShowStatement::Columns { table, pattern } => {
                self.execute_show_columns(table, pattern.as_deref())
            }
        }
    }

    /// SHOW TABLES — list all tables in the current database.
    fn execute_show_tables(&self) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read().unwrap();
        let names = storage.list_tables();
        let rows: Vec<Vec<Value>> = names.into_iter().map(|n| vec![Value::Text(n)]).collect();
        Ok(ExecutorResult::new(rows, 1))
    }

    /// SHOW DATABASES — v3.7.0 has a single in-memory catalog, so we
    /// return one row representing the current (only) database.
    fn execute_show_databases(&self) -> SqlResult<ExecutorResult> {
        // v3.7.0 has no multi-database support; the single in-memory
        // catalog IS the database. Return one placeholder row.
        Ok(ExecutorResult::new(
            vec![vec![Value::Text("default".to_string())]],
            1,
        ))
    }

    /// SHOW CREATE TABLE — return a minimal CREATE TABLE statement for `table`.
    /// v3.7.0 doesn't reconstruct full DDL, so we return a basic placeholder
    /// with the table name. Future versions should introspect the schema.
    fn execute_show_create_table(&self, table: &str) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read().unwrap();
        if !storage.list_tables().iter().any(|n| n == table) {
            return Err(SqlError::ExecutionError(format!(
                "Table '{}' does not exist",
                table
            )));
        }
        let row = vec![Value::Text(format!(
            "CREATE TABLE {} (id INTEGER) /* v3.7.0: schema reconstruction not implemented */",
            table
        ))];
        Ok(ExecutorResult::new(vec![row], 1))
    }

    /// SHOW INDEX — placeholder (v3.7.0 indexes are not cataloged).
    fn execute_show_index(&self, _table: &str) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
    }

    /// SHOW GRANTS — placeholder (v3.7.0 grant tracking is limited to roles).
    fn execute_show_grants(&self, _user: Option<&str>) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
    }

    /// SHOW COLUMNS — placeholder (v3.7.0 column metadata not exposed).
    fn execute_show_columns(
        &self,
        _table: &str,
        _pattern: Option<&str>,
    ) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
    }

    fn execute_show_grants_for(&self, user_spec: &str) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let catalog_guard = catalog.read().unwrap();

        let parts: Vec<&str> = user_spec.split('@').collect();
        let username = parts[0];
        let host = parts.get(1).unwrap_or(&"%");

        let identity = UserIdentity::new(username, host);
        let grants = catalog_guard
            .auth_manager()
            .get_all_grants_for_user(&identity);

        let rows: Vec<Vec<Value>> = grants
            .iter()
            .map(|g| {
                vec![
                    Value::Text(format!("{}@{}", g.user.username, g.user.host)),
                    Value::Text(g.privilege.to_string()),
                    Value::Text(format!("{:?}", g.object.object_type)),
                    Value::Text(g.object.object_name.clone()),
                ]
            })
            .collect();

        Ok(ExecutorResult::new(rows, 4))
    }

    fn execute_alter_table(&self, alter: &AlterTableStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write().unwrap();

        match &alter.operation {
            AlterTableOperation::AddColumn {
                name,
                data_type,
                nullable,
                default_value: _,
            } => {
                let column = ColumnDefinition {
                    name: name.clone(),
                    data_type: data_type.clone(),
                    nullable: *nullable,
                    primary_key: false,
                };
                storage.add_column(&alter.table_name, column)?;
            }
            AlterTableOperation::DropColumn { name } => {
                storage.drop_column(&alter.table_name, name)?;
            }
            AlterTableOperation::ModifyColumn {
                name,
                data_type,
                nullable,
            } => {
                let column = ColumnDefinition {
                    name: name.clone(),
                    data_type: data_type.clone(),
                    nullable: *nullable,
                    primary_key: false,
                };
                storage.modify_column(&alter.table_name, name, column)?;
            }
            AlterTableOperation::RenameTo { new_name } => {
                storage.rename_table(&alter.table_name, new_name)?;
            }
        }

        Ok(ExecutorResult::empty())
    }
}
