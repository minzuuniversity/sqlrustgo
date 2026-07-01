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
        crate::engine_cte::execute_with_select(self, with)
    }

    /// CTE + DML: 物化 CTE 后执行 DML body
    pub fn execute_with_dml(
        &mut self,
        with: &sqlrustgo_parser::parser::WithDmlStatement,
    ) -> SqlResult<ExecutorResult> {
        crate::engine_cte::execute_with_dml(self, with)
    }

    pub fn execute_insert(&mut self, insert: &InsertStatement) -> SqlResult<ExecutorResult> {
        // ARCH-3 (#3169): VtuGuard main-path enforcement (P0-1, Blocker-3)
        // MUST be the first statement. The gate script
        // check_arch3_no_bypass.sh greps the first 5 lines after
        // `pub fn execute_insert` for this exact call; keep it here.
        sqlrustgo_storage::vtu_guard::VtuGuard::<()>::assert_path_for_dml(
            "execute_insert",
            &insert.table,
        );
        crate::engine_dml::execute_insert(self, insert)
    }

    pub fn execute_update(&mut self, update: &UpdateStatement) -> SqlResult<ExecutorResult> {
        // ARCH-3 (#3169): VtuGuard main-path enforcement (P0-1, Blocker-3)
        // MUST be the first statement. The gate script
        // check_arch3_no_bypass.sh greps the first 5 lines after
        // `pub fn execute_update` for this exact call; keep it here.
        sqlrustgo_storage::vtu_guard::VtuGuard::<()>::assert_path_for_dml(
            "execute_update",
            &update.tables[0].name,
        );
        crate::engine_dml::execute_update(self, update)
    }
    pub fn execute_delete(&mut self, delete: &DeleteStatement) -> SqlResult<ExecutorResult> {
        // ARCH-3 (#3169): VtuGuard main-path enforcement (P0-1, Blocker-3)
        // MUST be the first statement. The gate script
        // check_arch3_no_bypass.sh greps the first 5 lines after
        // `pub fn execute_delete` for this exact call; keep it here.
        sqlrustgo_storage::vtu_guard::VtuGuard::<()>::assert_path_for_dml(
            "execute_delete",
            &delete.tables[0].name,
        );
        crate::engine_dml::execute_delete(self, delete)
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
    /// G13-OLTP-1: `pub(crate)` so the mysql-server dispatch site can
    /// call this on a read-lock guard (the COM_QUERY / COM_STMT_EXECUTE
    /// path uses `&self` to allow concurrent SELECTs).
    pub fn execute_show(&self, show: &ShowStatement) -> SqlResult<ExecutorResult> {
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
    /// G13-OLTP-1: `pub(crate)` so the mysql-server dispatch site can
    /// call this on a read-lock guard.
    pub fn execute_describe(&self, desc: &DescribeStatement) -> SqlResult<ExecutorResult> {
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
    pub(crate) fn begin_implicit_dml_tx(
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
    pub(crate) fn commit_implicit_dml_tx(&mut self, started_implicit: bool) {
        if started_implicit {
            let tx_id = self.current_tx_id.unwrap();
            let _ = self.transaction_manager.commit(tx_id);
            self.current_tx_id = None;
            self.tx_status = TxStatus::Idle;
        }
    }

    pub fn flush(&mut self) -> Result<(), SqlError> {
        self.storage.write().unwrap().flush()
    }
}
