//! ExecutionEngine - high-level SQL execution API
//! Provides a simple interface for executing SQL statements against a storage backend.

#![allow(unused_variables, unused_imports)]

use crate::engine_utils::{
    build_aggregate_schema, build_combined_schema, build_multi_table_combined_schema,
    cartesian_product, eval_predicate, evaluate_where_clause, find_column_index, sql_compare,
    validate_foreign_keys,
};
use crate::expr_utils::{
    compare_values, evaluate_binary_op, evaluate_expr_to_string, evaluate_expression,
    evaluate_expression_with_subq, expression_to_string, expression_to_value,
    expression_to_value_from_string, resolve_subqueries_in_expr,
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
    AggregateCall,
    AggregateFunction,
    AlterTableOperation,
    AlterTableStatement,
    CallStatement,
    CreateDatabaseStatement,
    CreateIndexStatement,
    CreateProcedureStatement,
    CreateRoleStatement,
    CreateTableStatement,
    CreateTriggerStatement,
    CreateViewStatement,
    DescribeStatement,
    DropDatabaseStatement,
    DropIndexStatement,
    DropRoleStatement,
    DropTableStatement,
    DropViewStatement,
    GrantRoleStatement,
    GrantStatement,
    InsertStatement,
    MergeStatement,
    ObjectType as ParserObjectType,
    Privilege as ParserPrivilege,
    RevokeRoleStatement,
    RevokeStatement,
    SelectStatement,
    SetRoleStatement,
    ShowStatement,
    StoredProcParam as ParserStoredProcParam,
    StoredProcParamMode as ParserParamMode,
    StoredProcStatement as ParserStatement,
    TruncateStatement, // SEM-1 (#3172)
};
use sqlrustgo_parser::transaction::IsolationLevel as ParserIsolationLevel;
use sqlrustgo_parser::JoinType;
use sqlrustgo_parser::{
    DeleteStatement,
    Expression,
    // SEM-1 (#3172)
    SavepointOp,
    Statement,
    TransactionStatement,
    UpdateStatement,
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
    pub(crate) tx_readonly: bool,
    pub(crate) default_isolation: TmIsolationLevel,
    pub(crate) current_role: Option<String>,
    /// CheckpointManager field — reserved for future PR-830F WAL lifecycle
    /// integration (currently set to None in all engine builders).
    /// PR-830F lifecycle methods were removed in SPEC-002; the field is
    /// kept for future re-introduction without changing the public struct layout.
    #[allow(dead_code)]
    pub(crate) checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>,
    pub(crate) parallel_degree: usize,
    pub(crate) stmt_cache: sqlrustgo_cache::PreparedStatementCache,
    /// View definitions: view_name → CREATE VIEW SQL text.
    /// Used for SHOW CREATE VIEW and view resolution.
    pub(crate) views: HashMap<String, String>,
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
            tx_readonly: false,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
            views: HashMap::new(),
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
            tx_readonly: false,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
            views: HashMap::new(),
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
            tx_readonly: false,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
            views: HashMap::new(),
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

    pub fn parallel_degree(&self) -> usize {
        self.parallel_degree
    }

    pub fn set_parallel_degree(&mut self, degree: usize) {
        self.parallel_degree = degree.max(1);
    }

    pub fn build_parallel_executor(
        &self,
    ) -> sqlrustgo_executor::parallel_executor::ParallelVolcanoExecutor {
        sqlrustgo_executor::parallel_executor::ParallelVolcanoExecutor::new(self.parallel_degree)
    }

    /// Get table statistics for CBO
    pub fn get_table_stats(&self) -> Arc<RwLock<ExecutionStats>> {
        self.stats.clone()
    }

    /// Read-only access to the underlying storage handle.
    ///
    /// Returned as `&Arc<RwLock<S>>` so callers can lock it themselves
    /// and read table info, scan rows, etc. without taking `&mut self`
    /// on the engine. Required by the LOAD DATA LOCAL INFILE handler
    /// to look up the target table's column count.
    pub fn storage_ref(&self) -> &Arc<RwLock<S>> {
        &self.storage
    }

    /// Read-lock the storage with a busy-wait retry to avoid lock convoy
    /// (Issue #3672). The previous implementation called
    /// `self.storage.read().unwrap()` directly, which can block
    /// indefinitely under sustained mixed write load (one long-running
    /// INSERT/UPDATE/DELETE holding the write lock blocks all 32 reader
    /// workers, eventually deadlocking the server).
    ///
    /// This method retries with a short backoff. If a writer is still
    /// holding the lock after 1000 attempts, it logs a warning and
    /// proceeds anyway (the read will block momentarily, but other
    /// threads won't pile up behind it).
    pub(crate) fn storage_read(&self) -> std::sync::RwLockReadGuard<'_, S> {
        for attempt in 0..1000u32 {
            if let Ok(g) = self.storage.try_read() {
                if attempt > 100 {
                    log::warn!(
                        "storage_read contended for {} attempts before lock acquired",
                        attempt
                    );
                }
                return g;
            }
            // Small backoff to yield to the writer
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        // Fallback: just do the blocking read. Better than deadlocking.
        log::error!("storage_read timeout after 1000 attempts; falling back to blocking read");
        self.storage.read().unwrap()
    }

    /// Bulk-insert pre-parsed records directly into storage, bypassing
    /// the SQL parser. This is the LOAD DATA LOCAL INFILE hot path: a
    /// 60 000-row lineitem.tbl used to take >5 min because the previous
    /// implementation built a single `INSERT INTO ... VALUES (...), (...), ...`
    /// SQL string (~2 MB for lineitem) and ran it through `execute()`,
    /// which re-parses the SQL every batch. With this method we hand the
    /// pre-parsed `Vec<Record>` straight to `Storage::insert`, which
    /// writes to the buffer pool + WAL in one go. Same transactional
    /// guarantees as a SQL INSERT (auto-commit per call), but no parser,
    /// no AST allocation, and no 2 MB string concatenation.
    ///
    /// Returns the number of rows inserted (== records.len() on success).
    pub fn bulk_insert_records(
        &self,
        table: &str,
        records: Vec<sqlrustgo_storage::Record>,
    ) -> SqlResult<u64> {
        let n = records.len() as u64;
        let mut storage = self
            .storage
            .write()
            .map_err(|e| SqlError::IoError(format!("storage lock poisoned: {}", e)))?;
        storage
            .insert(table, records)
            .map_err(|e| SqlError::ExecutionError(format!("bulk_insert_records: {}", e)))?;
        Ok(n)
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
            Statement::DropIndex(ref drop_idx) => self.execute_drop_index(drop_idx),
            Statement::CreateView(ref view) => self.execute_create_view(view),
            Statement::DropView(ref drop_view) => self.execute_drop_view(drop_view),
            Statement::Merge(ref merge) => self.execute_merge_statement(merge),
            Statement::DropTable(ref drop) => self.execute_drop_table(drop),
            Statement::Truncate(ref truncate) => self.execute_truncate(truncate),
            Statement::WithSelect(ref with) => self.execute_with_select(with),
            Statement::WithDml(ref with_dml) => self.execute_with_dml(with_dml),
            Statement::CreateIndex(idx) => self.execute_create_index(&idx),
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
            // SEM-1 (#3172): SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE SAVEPOINT
            Statement::SavepointStatement { ref name, op } => self.execute_savepoint(name, op),
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
            Statement::Describe(ref desc) => self.execute_describe(desc),
            Statement::AlterTable(ref alter) => self.execute_alter_table(alter),
            Statement::Prepare { ref name, ref sql } => self.execute_prepare(name, sql),
            Statement::Execute {
                ref name,
                ref params,
            } => self.execute_execute(name, params),
            Statement::Deallocate { ref name } => self.execute_deallocate(name),
            Statement::CreateDatabase(ref db) => self.execute_create_database(db),
            Statement::DropDatabase(ref db) => self.execute_drop_database(db),
            Statement::UseDatabase(ref name) => self.execute_use_database(name),
        }
    }

    /// CTE 物化: 将每个 CTE 子查询结果存入临时表，然后执行主查询
    pub fn execute_with_select(
        &mut self,
        with: &sqlrustgo_parser::parser::WithSelect,
    ) -> SqlResult<ExecutorResult> {
        use sqlrustgo_parser::Statement;
        use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};

        let materialized_tables: Vec<String> = if let Some(ref with_clause) = with.with_clause {
            for cte in &with_clause.ctes {
                if with_clause.recursive {
                    return Err(SqlError::ExecutionError(
                        "Recursive CTE not yet supported".to_string(),
                    ));
                }
                let cte_rows = match cte.subquery.as_ref() {
                    Statement::Select(s) => self.execute_select(&s)?.rows,
                    _ => {
                        return Err(SqlError::ExecutionError(
                            "CTE subquery must be SELECT".to_string(),
                        ));
                    }
                };
                let column_count = if !cte.columns.is_empty() {
                    cte.columns.len()
                } else if !cte_rows.is_empty() {
                    cte_rows[0].len()
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
                    })
                    .collect();
                let table_info = TableInfo {
                    name: cte.name.clone(),
                    columns,
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    partition_info: None,
                };
                let mut storage = self.storage.write().unwrap();
                storage
                    .create_table(&table_info)
                    .map_err(|e| SqlError::ExecutionError(format!("Create CTE table: {}", e)))?;
                if !cte_rows.is_empty() {
                    storage
                        .insert(&cte.name, cte_rows)
                        .map_err(|e| SqlError::ExecutionError(format!("Insert CTE rows: {}", e)))?;
                }
            }
            with_clause.ctes.iter().map(|c| c.name.clone()).collect()
        } else {
            Vec::new()
        };

        let result = self.execute_select(&with.select);

        // 清理临时 CTE 表
        if !materialized_tables.is_empty() {
            let mut storage = self.storage.write().unwrap();
            for name in &materialized_tables {
                let _ = storage.drop_table(name);
            }
        }

        result
    }

    /// CTE + DML: 物化 CTE 后执行 DML body
    pub fn execute_with_dml(
        &mut self,
        with: &sqlrustgo_parser::parser::WithDmlStatement,
    ) -> SqlResult<ExecutorResult> {
        use sqlrustgo_parser::Statement;
        use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};

        let with_clause = &with.with_clause;
        let materialized_tables: Vec<String> = {
            for cte in &with_clause.ctes {
                if with_clause.recursive {
                    return Err(SqlError::ExecutionError(
                        "Recursive CTE not yet supported".to_string(),
                    ));
                }
                let cte_rows = match cte.subquery.as_ref() {
                    Statement::Select(s) => self.execute_select(&s)?.rows,
                    _ => {
                        return Err(SqlError::ExecutionError(
                            "CTE subquery must be SELECT".to_string(),
                        ));
                    }
                };
                let column_count = if !cte.columns.is_empty() {
                    cte.columns.len()
                } else if !cte_rows.is_empty() {
                    cte_rows[0].len()
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
                    })
                    .collect();
                let table_info = TableInfo {
                    name: cte.name.clone(),
                    columns,
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    partition_info: None,
                };
                let mut storage = self.storage.write().unwrap();
                storage
                    .create_table(&table_info)
                    .map_err(|e| SqlError::ExecutionError(format!("Create CTE table: {}", e)))?;
                if !cte_rows.is_empty() {
                    storage
                        .insert(&cte.name, cte_rows)
                        .map_err(|e| SqlError::ExecutionError(format!("Insert CTE rows: {}", e)))?;
                }
            }
            with_clause.ctes.iter().map(|c| c.name.clone()).collect()
        };

        let result = match with.body.as_ref() {
            Statement::Insert(insert) => self.execute_insert(&insert),
            Statement::Update(update) => self.execute_update(&update),
            Statement::Delete(delete) => self.execute_delete(&delete),
            _ => Err(SqlError::ExecutionError(
                "Unsupported WithDml body type".to_string(),
            )),
        };

        // 清理临时 CTE 表
        if !materialized_tables.is_empty() {
            let mut storage = self.storage.write().unwrap();
            for name in &materialized_tables {
                let _ = storage.drop_table(name);
            }
        }

        result
    }

    pub fn execute_insert(&mut self, insert: &InsertStatement) -> SqlResult<ExecutorResult> {
        // ARCH-3 (#3169): VtuGuard main-path enforcement (P0-1, Blocker-3)
        sqlrustgo_storage::vtu_guard::VtuGuard::<()>::assert_path_for_dml(
            "execute_insert",
            &insert.table,
        );
        let (tm_tx_id, started_implicit) =
            self.begin_implicit_dml_tx("execute_insert", &insert.table)?;
        let table_name = insert.table.clone();

        // Get table info first (need it for triggers and FK validation)
        let table_info = {
            let storage = self.storage.read().unwrap();
            storage.get_table_info(&table_name)?.clone()
        };

        let all_records: Vec<Vec<Value>> = if let Some(ref select) = insert.select {
            let select_result = self.execute_select(select)?;
            Self::map_select_result_to_records(select_result, &insert.columns, &table_info)?
        } else {
            Self::build_insert_records(&insert.values)
        };

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

        // Apply CHAR(N) trailing-space padding per SQL standard (#3283 Task 8)
        let processed_records: Vec<Vec<Value>> = processed_records
            .into_iter()
            .map(|mut record| {
                for (idx, col) in table_info.columns.iter().enumerate() {
                    if let Some(n) = col.char_max_length {
                        if idx < record.len() {
                            if let Value::Text(s) = &record[idx] {
                                if s.len() < n {
                                    let mut padded = String::with_capacity(n);
                                    padded.push_str(s);
                                    for _ in s.len()..n {
                                        padded.push(' ');
                                    }
                                    record[idx] = Value::Text(padded);
                                }
                            }
                        }
                    }
                }
                record
            })
            .collect();

        // Validate FK and CHECK constraints, then insert
        {
            let mut storage = self.storage.write().unwrap();
            let col_names: Vec<String> =
                table_info.columns.iter().map(|c| c.name.clone()).collect();

            if !insert.is_replace && table_info.columns.iter().any(|c| c.primary_key) {
                let existing_rows = storage.scan(&table_name)?;
                let mut odku_handled_indices: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();
                for (new_idx, new_record) in processed_records.iter().enumerate() {
                    let mut matched = false;
                    for existing in &existing_rows {
                        if self.record_matches_unique_key(existing, new_record, &table_info) {
                            matched = true;
                            if let Some(ref updates) = insert.on_duplicate_key_update {
                                Self::apply_odku(
                                    &mut *storage,
                                    &table_name,
                                    &table_info,
                                    existing,
                                    updates,
                                )?;
                                odku_handled_indices.insert(new_idx);
                            }
                            break;
                        }
                    }
                    if matched && insert.on_duplicate_key_update.is_none() {
                        let pk_repr = table_info
                            .columns
                            .iter()
                            .enumerate()
                            .find_map(|(i, c)| {
                                if c.primary_key {
                                    new_record.get(i).map(|v| v.to_sql_string())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| "?".to_string());
                        return Err(SqlError::ExecutionError(format!(
                            "Duplicate entry '{}' for key 'PRIMARY'",
                            pk_repr
                        )));
                    }
                }
                // Filter out ODUK-handled records — they're already updated in storage
                let to_insert: Vec<Vec<Value>> = processed_records
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r)| {
                        if odku_handled_indices.contains(&i) {
                            None
                        } else {
                            Some(r.clone())
                        }
                    })
                    .collect();
                if !to_insert.is_empty() {
                    for record in &to_insert {
                        if !table_info.foreign_keys.is_empty() {
                            validate_foreign_keys(&*storage, &table_info, record, &insert.columns)?;
                        }
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
                    storage.insert(&table_name, to_insert)?;
                }
            } else {
                for record in &processed_records {
                    if !table_info.foreign_keys.is_empty() {
                        validate_foreign_keys(&*storage, &table_info, record, &insert.columns)?;
                    }
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

        // INT-1: autocommit — leave the commit decision to the helper.
        self.commit_implicit_dml_tx(started_implicit);

        Ok(ExecutorResult::new(vec![], all_records.len()))
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

    pub fn execute_update(&mut self, update: &UpdateStatement) -> SqlResult<ExecutorResult> {
        if update.tables.is_empty() {
            return Err(SqlError::ExecutionError(
                "UPDATE requires at least one table".to_string(),
            ));
        }
        if update.tables.len() > 1 {
            return self.execute_update_multi_table(update);
        }
        let table_name = update.tables[0].name.clone();

        // ARCH-3 (#3169): VtuGuard main-path enforcement (P0-1, Blocker-3)
        sqlrustgo_storage::vtu_guard::VtuGuard::<()>::assert_path_for_dml(
            "execute_update",
            &table_name,
        );
        let (tm_tx_id, started_implicit) =
            self.begin_implicit_dml_tx("execute_update", &table_name)?;

        let scalar_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
            let result = self.execute_select(subq).map_err(|e| e.to_string())?;
            Ok(result
                .rows
                .first()
                .and_then(|r| r.first().cloned())
                .unwrap_or(Value::Null))
        };
        let list_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
            let result = self.execute_select(subq).map_err(|e| e.to_string())?;
            Ok(result
                .rows
                .into_iter()
                .map(|r| r.first().cloned().unwrap_or(Value::Null))
                .collect())
        };
        let resolved_set: Vec<(String, Expression)> = update
            .set_clauses
            .iter()
            .map(|(col, expr)| {
                Ok((
                    col.clone(),
                    resolve_subqueries_in_expr(expr, &scalar_eval, &list_eval)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(|e| SqlError::ExecutionError(format!("UPDATE SET subquery: {}", e)))?;
        let resolved_where: Option<Expression> = match &update.where_clause {
            Some(w) => Some(
                resolve_subqueries_in_expr(w, &scalar_eval, &list_eval).map_err(|e| {
                    SqlError::ExecutionError(format!("UPDATE WHERE subquery: {}", e))
                })?,
            ),
            None => None,
        };
        let resolved_update = UpdateStatement {
            tables: update.tables.clone(),
            set_clauses: resolved_set,
            where_clause: resolved_where,
        };

        // If no WHERE clause, use the simple storage.update() path
        // PR-842 Option A: compute updates from SET clauses (per-row evaluation
        // collapses to a single value for literal / constant expressions, which
        // is the common no-WHERE case). For column references, the first row
        // is used as the evaluation context — for literal values this yields
        // the correct after-image for every row.
        if resolved_update.where_clause.is_none() {
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
            let updates: Vec<(usize, sqlrustgo_types::Value)> = resolved_update
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

        let where_clause = resolved_update.where_clause.as_ref().unwrap();

        // Filter rows that match the WHERE clause
        let rows_to_update: Vec<Vec<Value>> = all_rows
            .clone()
            .into_iter()
            .filter(|row| evaluate_where_clause(where_clause, row, &table_info))
            .collect();

        Self::ir_validate_update_filter(&all_rows, &table_info, &rows_to_update, &resolved_update);

        let count = rows_to_update.len();

        if count == 0 {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        // Build column index map for SET clauses
        let set_col_indices: Vec<(usize, &Expression)> = resolved_update
            .set_clauses
            .iter()
            .filter_map(|(col_name, expr)| {
                find_column_index(col_name, &table_info).map(|idx| (idx, expr))
            })
            .collect();

        // Apply SET expressions to each matching row
        let updated_rows = Self::apply_set_clauses(&rows_to_update, &set_col_indices, &table_info);

        // Execute BEFORE UPDATE triggers (if any)
        let trigger_executor = TriggerExecutor::new(self.storage.clone());
        let trigger_modified_rows = Self::run_before_update_triggers(
            &trigger_executor,
            &table_name,
            &rows_to_update,
            &updated_rows,
        )?;

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
                for record in &trigger_modified_rows {
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

            let pk_idx = table_info
                .columns
                .iter()
                .position(|c| c.primary_key)
                .unwrap_or(0);
            for (prior_row, new_row) in rows_to_update.iter().zip(trigger_modified_rows.iter()) {
                let pk_val = prior_row
                    .get(pk_idx)
                    .cloned()
                    .unwrap_or(sqlrustgo_types::Value::Null);
                storage.delete(&table_name, std::slice::from_ref(&pk_val))?;
                storage.insert(&table_name, vec![new_row.clone()])?;
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

        // INT-1: autocommit — leave the commit decision to the helper.
        self.commit_implicit_dml_tx(started_implicit);

        Ok(ExecutorResult::new(vec![], count))
    }

    pub fn execute_delete(&mut self, delete: &DeleteStatement) -> SqlResult<ExecutorResult> {
        if delete.tables.is_empty() {
            return Err(SqlError::ExecutionError(
                "DELETE requires at least one table".to_string(),
            ));
        }
        if delete.tables.len() > 1 || delete.using.is_some() {
            return self.execute_delete_multi_table(delete);
        }
        let table_name = delete.tables[0].name.clone();

        // ARCH-3 (#3169): VtuGuard main-path enforcement (P0-1, Blocker-3)
        sqlrustgo_storage::vtu_guard::VtuGuard::<()>::assert_path_for_dml(
            "execute_delete",
            &table_name,
        );
        let (tm_tx_id, started_implicit) =
            self.begin_implicit_dml_tx("execute_delete", &table_name)?;

        let scalar_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
            let result = self.execute_select(subq).map_err(|e| e.to_string())?;
            Ok(result
                .rows
                .first()
                .and_then(|r| r.first().cloned())
                .unwrap_or(Value::Null))
        };
        let list_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
            let result = self.execute_select(subq).map_err(|e| e.to_string())?;
            Ok(result
                .rows
                .into_iter()
                .map(|r| r.first().cloned().unwrap_or(Value::Null))
                .collect())
        };
        let resolved_where: Option<Expression> = match &delete.where_clause {
            Some(w) => Some(
                resolve_subqueries_in_expr(w, &scalar_eval, &list_eval).map_err(|e| {
                    SqlError::ExecutionError(format!("DELETE WHERE subquery: {}", e))
                })?,
            ),
            None => None,
        };
        let resolved_delete = DeleteStatement {
            tables: delete.tables.clone(),
            using: delete.using.clone(),
            where_clause: resolved_where,
        };

        // If no WHERE clause, delete all rows (current behavior is correct)
        if resolved_delete.where_clause.is_none() {
            let mut storage = self.storage.write().unwrap();
            let count = storage.delete(&table_name, &[])?;
            drop(storage);
            // INT-1: Autocommit — commit the implicit TX so WAL/MVCC see this.
            self.commit_implicit_dml_tx(started_implicit);
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
        let where_clause = resolved_delete.where_clause.as_ref().unwrap();
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

        // INT-1: autocommit — leave the commit decision to the helper.
        self.commit_implicit_dml_tx(started_implicit);

        Ok(ExecutorResult::new(vec![], count))
    }

    /// Execute `UPDATE t1, t2, ... SET ... WHERE ...` against the
    /// cartesian product of the listed tables. Single-table UPDATE is
    /// handled inline by `execute_update`.
    fn execute_update_multi_table(
        &mut self,
        update: &UpdateStatement,
    ) -> SqlResult<ExecutorResult> {
        let scalar_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
            let result = self.execute_select(subq).map_err(|e| e.to_string())?;
            Ok(result
                .rows
                .first()
                .and_then(|r| r.first().cloned())
                .unwrap_or(Value::Null))
        };
        let list_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
            let result = self.execute_select(subq).map_err(|e| e.to_string())?;
            Ok(result
                .rows
                .into_iter()
                .map(|r| r.first().cloned().unwrap_or(Value::Null))
                .collect())
        };
        let resolved_set: Vec<(String, Expression)> = update
            .set_clauses
            .iter()
            .map(|(col, expr)| {
                Ok((
                    col.clone(),
                    resolve_subqueries_in_expr(expr, &scalar_eval, &list_eval)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(|e| SqlError::ExecutionError(format!("UPDATE SET subquery: {}", e)))?;
        let resolved_where: Option<Expression> = match &update.where_clause {
            Some(w) => Some(
                resolve_subqueries_in_expr(w, &scalar_eval, &list_eval).map_err(|e| {
                    SqlError::ExecutionError(format!("UPDATE WHERE subquery: {}", e))
                })?,
            ),
            None => None,
        };

        let table_refs = &update.tables;
        let mut per_table_rows: Vec<Vec<Vec<Value>>> = Vec::with_capacity(table_refs.len());
        let mut per_table_info: Vec<sqlrustgo_storage::TableInfo> =
            Vec::with_capacity(table_refs.len());
        let mut per_table_prefix: Vec<String> = Vec::with_capacity(table_refs.len());
        {
            let storage = self.storage.read().unwrap();
            for tref in table_refs {
                let info = storage.get_table_info(&tref.name)?.clone();
                let rows = storage.scan(&tref.name)?;
                per_table_rows.push(rows);
                per_table_info.push(info);
                let prefix = tref.alias.clone().unwrap_or_else(|| tref.name.clone());
                per_table_prefix.push(prefix);
            }
        }

        let combined_info = build_multi_table_combined_schema(&per_table_info, &per_table_prefix);
        let combined_cols: Vec<(String, usize)> = combined_info
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), i))
            .collect();
        let col_offsets: Vec<usize> = {
            let mut offs = Vec::with_capacity(table_refs.len());
            let mut acc = 0usize;
            for info in &per_table_info {
                offs.push(acc);
                acc += info.columns.len();
            }
            offs
        };

        let combined_rows = cartesian_product(&per_table_rows);

        let mut per_table_updates: Vec<Vec<(Vec<Value>, Vec<Value>)>> =
            (0..table_refs.len()).map(|_| Vec::new()).collect();
        let mut total_count = 0usize;

        for combined_row in &combined_rows {
            let matches = match &resolved_where {
                None => true,
                Some(w) => evaluate_where_clause(w, combined_row, &combined_info),
            };
            if !matches {
                continue;
            }
            let mut after_row = combined_row.clone();
            for (col, expr) in &resolved_set {
                let target_col = col.split_once('.').map(|(_, c)| c).unwrap_or(col.as_str());
                let new_val =
                    evaluate_expression(expr, combined_row, &combined_info).unwrap_or(Value::Null);
                if let Some((_, idx)) = combined_cols
                    .iter()
                    .find(|(name, _)| name.ends_with(&format!(".{}", target_col)))
                {
                    if let Some(slot) = after_row.get_mut(*idx) {
                        *slot = new_val;
                    }
                }
            }
            for (t, _) in table_refs.iter().enumerate() {
                let cols_start = col_offsets[t];
                let cols_end = cols_start + per_table_info[t].columns.len();
                let before = combined_row[cols_start..cols_end].to_vec();
                let after = after_row[cols_start..cols_end].to_vec();
                per_table_updates[t].push((before, after));
            }
            total_count += 1;
        }

        self.apply_multi_table_updates(table_refs, per_table_updates, total_count)
    }

    fn apply_multi_table_updates(
        &mut self,
        table_refs: &[sqlrustgo_parser::TableRef],
        per_table_updates: Vec<Vec<(Vec<Value>, Vec<Value>)>>,
        total_count: usize,
    ) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write().unwrap();
        for (t, tref) in table_refs.iter().enumerate() {
            let pairs = &per_table_updates[t];
            if pairs.is_empty() {
                continue;
            }
            let info = storage.get_table_info(&tref.name)?.clone();
            let pk_idx = info.columns.iter().position(|c| c.primary_key).unwrap_or(0);
            for (before, after) in pairs {
                let pk_val = before.get(pk_idx).cloned().unwrap_or(Value::Null);
                storage.delete(&tref.name, std::slice::from_ref(&pk_val))?;
                storage.insert(&tref.name, vec![after.clone()])?;
            }
        }
        Ok(ExecutorResult::new(vec![], total_count))
    }

    /// Execute `DELETE t1, t2 FROM t1, t2 WHERE ...` against the
    /// cartesian product. Single-table DELETE is inline in
    /// `execute_delete`.
    fn execute_delete_multi_table(
        &mut self,
        delete: &DeleteStatement,
    ) -> SqlResult<ExecutorResult> {
        let source_refs: Vec<sqlrustgo_parser::TableRef> = match &delete.using {
            Some(s) => s.clone(),
            None => delete.tables.clone(),
        };
        let target_refs = &delete.tables;

        let resolved_where: Option<Expression> = match &delete.where_clause {
            Some(w) => {
                let scalar_eval =
                    |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
                        let result = self.execute_select(subq).map_err(|e| e.to_string())?;
                        Ok(result
                            .rows
                            .first()
                            .and_then(|r| r.first().cloned())
                            .unwrap_or(Value::Null))
                    };
                let list_eval =
                    |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
                        let result = self.execute_select(subq).map_err(|e| e.to_string())?;
                        Ok(result
                            .rows
                            .into_iter()
                            .map(|r| r.first().cloned().unwrap_or(Value::Null))
                            .collect())
                    };
                Some(
                    resolve_subqueries_in_expr(w, &scalar_eval, &list_eval).map_err(|e| {
                        SqlError::ExecutionError(format!("DELETE WHERE subquery: {}", e))
                    })?,
                )
            }
            None => None,
        };

        let mut per_table_rows: Vec<Vec<Vec<Value>>> = Vec::with_capacity(source_refs.len());
        let mut per_table_info: Vec<sqlrustgo_storage::TableInfo> =
            Vec::with_capacity(source_refs.len());
        let mut per_table_prefix: Vec<String> = Vec::with_capacity(source_refs.len());
        {
            let storage = self.storage.read().unwrap();
            for tref in &source_refs {
                let info = storage.get_table_info(&tref.name)?.clone();
                let rows = storage.scan(&tref.name)?;
                per_table_rows.push(rows);
                per_table_info.push(info);
                let prefix = tref.alias.clone().unwrap_or_else(|| tref.name.clone());
                per_table_prefix.push(prefix);
            }
        }
        let combined_info = build_multi_table_combined_schema(&per_table_info, &per_table_prefix);
        let combined_rows = cartesian_product(&per_table_rows);

        let mut per_table_drop: Vec<Vec<Vec<Value>>> =
            (0..source_refs.len()).map(|_| Vec::new()).collect();

        for combined_row in &combined_rows {
            let matches = match &resolved_where {
                None => true,
                Some(w) => evaluate_where_clause(w, combined_row, &combined_info),
            };
            if !matches {
                continue;
            }
            for (t, tref) in source_refs.iter().enumerate() {
                if target_refs.iter().any(|x| x.name == tref.name) {
                    let cols_start: usize = (0..t).map(|k| per_table_info[k].columns.len()).sum();
                    let cols_end = cols_start + per_table_info[t].columns.len();
                    per_table_drop[t].push(combined_row[cols_start..cols_end].to_vec());
                }
            }
        }

        let mut total = 0usize;
        let mut storage = self.storage.write().unwrap();
        for (t, tref) in source_refs.iter().enumerate() {
            if !target_refs.iter().any(|x| x.name == tref.name) {
                continue;
            }
            let info = storage.get_table_info(&tref.name)?.clone();
            let pk_idx = info.columns.iter().position(|c| c.primary_key).unwrap_or(0);
            for row in &per_table_drop[t] {
                let pk_val = row.get(pk_idx).cloned().unwrap_or(Value::Null);
                if storage.delete(&tref.name, std::slice::from_ref(&pk_val))? > 0 {
                    total += 1;
                }
            }
        }
        Ok(ExecutorResult::new(vec![], total))
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
                char_max_length: c.char_max_length,
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

    fn execute_create_database(&self, db: &CreateDatabaseStatement) -> SqlResult<ExecutorResult> {
        // v3.10.0: 多数据库支持
        // 委托给 storage 创建数据库子目录
        if db.name == "default" || db.name == "postgres" || db.name == "mysql" {
            return Err(SqlError::ExecutionError(format!(
                "CREATE DATABASE '{}' is not permitted (reserved database name)",
                db.name
            )));
        }
        let mut storage = self.storage.write().unwrap();
        storage
            .create_database(&db.name)
            .map_err(|e| SqlError::ExecutionError(format!("CREATE DATABASE: {}", e)))?;
        Ok(ExecutorResult::empty())
    }

    fn execute_drop_database(&self, db: &DropDatabaseStatement) -> SqlResult<ExecutorResult> {
        // Refuse to drop the "default" database to prevent orphaned references.
        // In v3.10 multi-database mode, the current_database context will be
        // tracked in the session state instead.
        if db.name == "default" || db.name == "postgres" || db.name == "mysql" {
            return Err(SqlError::ExecutionError(format!(
                "DROP DATABASE '{}' is not permitted (reserved database name)",
                db.name
            )));
        }
        let mut storage = self.storage.write().unwrap();
        storage
            .drop_database(&db.name)
            .map_err(|e| SqlError::ExecutionError(format!("DROP DATABASE: {}", e)))?;
        Ok(ExecutorResult::empty())
    }

    fn execute_use_database(&self, _db: &str) -> SqlResult<ExecutorResult> {
        // v3.9.0 single-database: USE <database> is accepted for MySQL wire
        // compatibility but is a no-op. v3.10 multi-database mode will switch
        // the active database context.
        Ok(ExecutorResult::empty())
    }

    fn execute_truncate(&self, truncate: &TruncateStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write().unwrap();
        if !storage.has_table(&truncate.name) {
            return Err(SqlError::ExecutionError(format!(
                "Table not found: {}",
                truncate.name
            )));
        }
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

    fn execute_drop_index(&self, idx: &DropIndexStatement) -> SqlResult<ExecutorResult> {
        Err(SqlError::ExecutionError(
            "DROP INDEX not fully supported yet".to_string(),
        ))
    }

    fn execute_create_view(&mut self, view: &CreateViewStatement) -> SqlResult<ExecutorResult> {
        // 存储视图定义 (view name → SQL text)
        self.views.insert(view.name.clone(), format!("{:?}", view));
        Ok(ExecutorResult::empty())
    }
    fn execute_drop_view(&mut self, drop_view: &DropViewStatement) -> SqlResult<ExecutorResult> {
        if self.views.remove(&drop_view.name).is_some() || drop_view.if_exists {
            Ok(ExecutorResult::empty())
        } else {
            Err(SqlError::ExecutionError(format!(
                "View not found: {}",
                drop_view.name
            )))
        }
    }

    fn execute_merge_statement(&self, _merge: &MergeStatement) -> SqlResult<ExecutorResult> {
        Err(SqlError::ExecutionError(
            "MERGE not yet supported via execute() — use LocalExecutorDml path".to_string(),
        ))
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
                readonly,
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
                self.begin_transaction(iso, *readonly)
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
                self.begin_transaction(iso, false)
            }
        }
    }

    fn begin_transaction(
        &mut self,
        isolation: TmIsolationLevel,
        readonly: bool,
    ) -> SqlResult<ExecutorResult> {
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
        self.tx_readonly = readonly;
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
        self.tx_readonly = false;
        Ok(ExecutorResult::empty())
    }

    /// SEM-1 (#3172): Execute SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE SAVEPOINT.
    ///
    /// Routes the parsed statement to the per-tx SavepointManager. The
    /// physical undo of tuple changes is deferred to a future iteration;
    /// this method only manages the savepoint namespace and the undo-log
    /// cursor.
    fn execute_savepoint(&mut self, name: &str, op: SavepointOp) -> SqlResult<ExecutorResult> {
        // An active transaction is required for any savepoint operation.
        let tx_id = self.current_tx_id.ok_or_else(|| {
            SqlError::ExecutionError(
                "SAVEPOINT / ROLLBACK TO SAVEPOINT / RELEASE SAVEPOINT \
                 requires an active transaction (BEGIN or implicit autocommit TX)"
                    .to_string(),
            )
        })?;
        match op {
            SavepointOp::Save => self
                .transaction_manager
                .savepoint(tx_id, name.to_string())
                .map_err(|e| {
                    SqlError::ExecutionError(format!("SAVEPOINT {} failed: {}", name, e))
                })?,
            SavepointOp::RollbackTo => self
                .transaction_manager
                .rollback_to_savepoint(tx_id, name)
                .map_err(|e| {
                    SqlError::ExecutionError(format!(
                        "ROLLBACK TO SAVEPOINT {} failed: {}",
                        name, e
                    ))
                })?,
            SavepointOp::Release => self
                .transaction_manager
                .release_savepoint(tx_id, name)
                .map_err(|e| {
                    SqlError::ExecutionError(format!("RELEASE SAVEPOINT {} failed: {}", name, e))
                })?,
        }
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
        self.tx_readonly = false;
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

    /// SHOW CREATE TABLE — reconstruct CREATE TABLE from the live schema.
    fn execute_show_create_table(&self, table: &str) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read().unwrap();
        if !storage.list_tables().iter().any(|n| n == table) {
            return Err(SqlError::ExecutionError(format!(
                "Table '{}' does not exist",
                table
            )));
        }
        let info = storage
            .get_table_info(table)
            .map_err(|e| SqlError::ExecutionError(format!("cannot introspect {table}: {e}")))?;
        let cols: Vec<String> = info
            .columns
            .iter()
            .map(|c| {
                let nullable = if c.nullable { "" } else { " NOT NULL" };
                format!("{} {}{}", c.name, c.data_type, nullable)
            })
            .collect();
        let ddl = format!("CREATE TABLE {} ({})", table, cols.join(", "));
        Ok(ExecutorResult::new(vec![vec![Value::Text(ddl)]], 1))
    }

    /// SHOW INDEX — placeholder (v3.7.0 indexes are not cataloged).
    fn execute_show_index(&self, _table: &str) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
    }

    /// DESCRIBE table — return one row per column with Field/Type/Null/Key/Default/Extra.
    fn execute_describe(&self, desc: &DescribeStatement) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read().unwrap();
        if !storage.list_tables().iter().any(|n| n == &desc.table) {
            return Err(SqlError::ExecutionError(format!(
                "Table '{}' does not exist",
                desc.table
            )));
        }
        let info = storage.get_table_info(&desc.table).map_err(|e| {
            SqlError::ExecutionError(format!("cannot introspect {}: {e}", desc.table))
        })?;
        let rows: Vec<Vec<Value>> = info
            .columns
            .iter()
            .map(|c| {
                let null_str = if c.nullable { "YES" } else { "NO" };
                let key_str = if c.primary_key { "PRI" } else { "" };
                vec![
                    Value::Text(c.name.clone()),
                    Value::Text(c.data_type.clone()),
                    Value::Text(null_str.to_string()),
                    Value::Text(key_str.to_string()),
                    Value::Text("NULL".to_string()),
                    Value::Text(String::new()),
                ]
            })
            .collect();
        Ok(ExecutorResult::new(rows, info.columns.len()))
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
                    char_max_length: None,
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
                    char_max_length: None,
                };
                storage.modify_column(&alter.table_name, name, column)?;
            }
            AlterTableOperation::RenameTo { new_name } => {
                storage.rename_table(&alter.table_name, new_name)?;
            }
        }

        Ok(ExecutorResult::empty())
    }

    fn execute_prepare(&mut self, name: &str, sql: &str) -> SqlResult<ExecutorResult> {
        let parsed = sqlrustgo_parser::parse(sql)
            .map_err(|e| SqlError::ParseError(format!("PREPARE failed to parse SQL: {}", e)))?;
        self.stmt_cache.prepare(name, sql, parsed);
        Ok(ExecutorResult::empty())
    }

    fn execute_execute(
        &mut self,
        name: &str,
        params: &[sqlrustgo_parser::Expression],
    ) -> SqlResult<ExecutorResult> {
        let sql = self.stmt_cache.execute_with_sql(name).ok_or_else(|| {
            SqlError::ExecutionError(format!(
                "prepared statement '{}' not found (call PREPARE first)",
                name
            ))
        })?;
        if !params.is_empty() {
            return Err(SqlError::ExecutionError(
                "EXECUTE ... USING with bind parameters is not yet supported in v3.9.0; \
                 use direct parameter substitution in the SQL body for now"
                    .to_string(),
            ));
        }
        self.execute(&sql)
    }

    fn execute_deallocate(&mut self, name: &str) -> SqlResult<ExecutorResult> {
        self.stmt_cache.deallocate(name);
        Ok(ExecutorResult::empty())
    }

    pub fn stmt_cache_stats(&self) -> sqlrustgo_cache::CacheStats {
        self.stmt_cache.stats()
    }

    /// Begin an implicit TX for DML. Returns `(tx_id, started_implicit)`.
    /// `started_implicit` is `true` ONLY when this call started a fresh TX.
    fn begin_implicit_dml_tx(
        &mut self,
        op: &'static str,
        _table: &str,
    ) -> SqlResult<(Option<TxId>, bool)> {
        if self.tx_readonly {
            return Err(SqlError::ExecutionError(
                "Cannot execute DML in READONLY transaction".to_string(),
            ));
        }
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
            TxStatus::Idle | TxStatus::Active => {}
        }
        let _ = op;
        if self.current_tx_id.is_none() {
            let tx_id = self
                .transaction_manager
                .begin_transaction(self.default_isolation)
                .map_err(|e| SqlError::ExecutionError(format!("TM.begin failed: {:?}", e)))?;
            self.current_tx_id = Some(tx_id);
            self.tx_status = TxStatus::Active;
            if let Ok(mut storage) = self.storage.write() {
                storage.set_current_tx_id(tx_id.as_u64());
            }
            Ok((Some(tx_id), true))
        } else {
            Ok((self.current_tx_id, false))
        }
    }

    /// Commit the implicit DML TX started by `begin_implicit_dml_tx`.
    /// Idempotent when `started_implicit` is `false` (user controls commit/rollback).
    fn commit_implicit_dml_tx(&mut self, started_implicit: bool) {
        if started_implicit {
            let tx_id = self.current_tx_id.unwrap();
            let _ = self.transaction_manager.commit(tx_id);
            self.current_tx_id = None;
            self.tx_status = TxStatus::Idle;
        }
    }

    /// Convert `INSERT VALUES` expression rows to materialised `Value` records.
    fn build_insert_records(values: &[Vec<Expression>]) -> Vec<Vec<Value>> {
        values
            .iter()
            .map(|row_exprs| row_exprs.iter().map(expression_to_value).collect())
            .collect()
    }

    fn map_select_result_to_records(
        result: ExecutorResult,
        target_columns: &[String],
        target_table_info: &TableInfo,
    ) -> SqlResult<Vec<Vec<Value>>> {
        let target_col_indices: Vec<usize> = if target_columns.is_empty() {
            if !result.rows.is_empty() && result.rows[0].len() != target_table_info.columns.len() {
                return Err(SqlError::ExecutionError(format!(
                    "INSERT SELECT column count mismatch: SELECT has {} columns, target table has {}",
                    result.rows[0].len(),
                    target_table_info.columns.len()
                )));
            }
            (0..target_table_info.columns.len()).collect()
        } else {
            target_columns
                .iter()
                .map(|name| {
                    target_table_info
                        .columns
                        .iter()
                        .position(|c| c.name == *name)
                        .ok_or_else(|| {
                            SqlError::ExecutionError(format!(
                                "Unknown column '{}' in INSERT column list",
                                name
                            ))
                        })
                })
                .collect::<SqlResult<Vec<_>>>()?
        };

        result
            .rows
            .into_iter()
            .map(|row| {
                let mut record = Vec::with_capacity(target_col_indices.len());
                for (source_idx, &target_idx) in target_col_indices.iter().enumerate() {
                    let value = row.get(source_idx).cloned().unwrap_or(Value::Null);
                    let target_col = &target_table_info.columns[target_idx];
                    record.push(Self::coerce_value_to_column(value, target_col));
                }
                Ok(record)
            })
            .collect()
    }

    fn coerce_value_to_column(value: Value, target_col: &ColumnDefinition) -> Value {
        let upper = target_col.data_type.to_uppercase();
        match (&value, upper.as_str()) {
            (Value::Integer(i), "TEXT" | "VARCHAR" | "CHAR") => Value::Text(i.to_string()),
            (Value::Float(f), "TEXT" | "VARCHAR" | "CHAR") => Value::Text(f.to_string()),
            (Value::Boolean(b), "TEXT" | "VARCHAR" | "CHAR") => {
                Value::Text(if *b { "TRUE" } else { "FALSE" }.to_string())
            }
            (Value::Text(s), "INTEGER" | "INT" | "BIGINT" | "SMALLINT") => {
                s.parse::<i64>().map(Value::Integer).unwrap_or(Value::Null)
            }
            (Value::Text(s), "REAL" | "FLOAT" | "DOUBLE") => {
                s.parse::<f64>().map(Value::Float).unwrap_or(Value::Null)
            }
            (v, _) => v.clone(),
        }
    }

    /// Apply `ON DUPLICATE KEY UPDATE`: delete existing row + re-insert with updates.
    /// Storage has no per-PK update API, so we go via delete+insert.
    fn apply_odku(
        storage: &mut dyn StorageEngine,
        table_name: &str,
        table_info: &sqlrustgo_storage::TableInfo,
        existing_row: &[Value],
        updates: &[(String, Expression)],
    ) -> SqlResult<()> {
        let col_names: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();

        // Build update list (column index -> new value)
        let update: Vec<(usize, Value)> = updates
            .iter()
            .filter_map(|(col_name, expr)| {
                let idx = col_names.iter().position(|n| n == col_name)?;
                let val = expression_to_value(expr);
                Some((idx, val))
            })
            .collect();

        // Find PK column values for the filter
        let pk_values: Vec<Value> = table_info
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.primary_key)
            .filter_map(|(i, _)| existing_row.get(i).cloned())
            .collect();

        // Use storage.update() with PK filter to update only the matching row
        if !pk_values.is_empty() && !update.is_empty() {
            storage.update(table_name, &pk_values, &update)?;
        }

        Ok(())
    }

    /// Apply UPDATE SET-clause expressions to each matching row.
    fn apply_set_clauses(
        rows_to_update: &[Vec<Value>],
        set_col_indices: &[(usize, &Expression)],
        table_info: &sqlrustgo_storage::TableInfo,
    ) -> Vec<Vec<Value>> {
        rows_to_update
            .iter()
            .map(|row| {
                let mut new_row = row.clone();
                for &(col_idx, set_expr) in set_col_indices {
                    let new_val =
                        evaluate_expression(set_expr, &new_row, table_info).unwrap_or(Value::Null);
                    if col_idx < new_row.len() {
                        new_row[col_idx] = new_val;
                    }
                }
                new_row
            })
            .collect()
    }

    /// Run BEFORE UPDATE triggers; return rows transformed by triggers
    /// (or unmodified if no triggers). Extracted from `execute_update`.
    fn run_before_update_triggers(
        trigger_executor: &TriggerExecutor,
        table_name: &str,
        rows_to_update: &[Vec<Value>],
        updated_rows: &[Vec<Value>],
    ) -> SqlResult<Vec<Vec<Value>>> {
        let before_triggers = trigger_executor.get_triggers_for_operation(
            table_name,
            ExecTriggerTiming::Before,
            ExecTriggerEvent::Update,
        );
        if before_triggers.is_empty() {
            return Ok(updated_rows.to_vec());
        }
        let mut modified = Vec::new();
        for (i, updated_row) in updated_rows.iter().enumerate() {
            let old_row = &rows_to_update[i];
            let result =
                trigger_executor.execute_before_update(table_name, old_row, updated_row)?;
            modified.push(result);
        }
        Ok(modified)
    }

    /// Cross-validate legacy WHERE filter against IR plan; warn on mismatch.
    fn ir_validate_update_filter(
        all_rows: &[Vec<Value>],
        table_info: &sqlrustgo_storage::TableInfo,
        rows_to_update: &[Vec<Value>],
        update: &UpdateStatement,
    ) {
        let Ok(plan) = AstAdapter::to_update_plan(update, table_info) else {
            return;
        };
        let ir_filtered: Vec<Vec<Value>> = all_rows
            .iter()
            .filter(|row| plan.predicate().evaluate(row, table_info))
            .cloned()
            .collect();
        if ir_filtered.len() != rows_to_update.len() {
            eprintln!(
                "[IR VALIDATION] Predicate mismatch: legacy={}, ir={}",
                rows_to_update.len(),
                ir_filtered.len()
            );
        }
    }
    pub fn flush(&mut self) -> Result<(), SqlError> {
        self.storage.write().unwrap().flush()
    }
}
