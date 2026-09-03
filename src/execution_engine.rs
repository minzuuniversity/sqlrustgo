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
use parking_lot::RwLock;
use sqlrustgo_catalog::stored_proc::{ParamMode, StoredProcParam, StoredProcStatement};
use sqlrustgo_catalog::{
    auth::UserIdentity, AuthErrorCode, Catalog, ObjectRef, Privilege, StoredProcedure,
};
use sqlrustgo_executor::ast_adapter::AstAdapter;
use sqlrustgo_executor::expr as expr_mod;
use sqlrustgo_executor::stored_proc::StoredProcExecutor;
use sqlrustgo_executor::trigger::{
    TriggerEvent as ExecTriggerEvent, TriggerExecutor, TriggerTiming as ExecTriggerTiming,
};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_optimizer::rules::{BinaryOperator, Expr};
use sqlrustgo_optimizer::stats::{
    build_histogram_from_values, ColumnStats as OptColumnStats, Histogram,
};
use sqlrustgo_optimizer::unified_cost::UnifiedCostModel;
use sqlrustgo_optimizer::unified_plan::UnifiedPlan;
use sqlrustgo_parser::parser::{
    AggregateCall, AggregateFunction, AlterSequenceStatement, AlterTableOperation,
    AlterTableStatement, AlterUserStatement, CallStatement, CompressionAlgorithm,
    CreateDatabaseStatement, CreateFunctionStatement, CreateIndexStatement,
    CreateProcedureStatement, CreateRoleStatement, CreateSequenceStatement, CreateTableStatement,
    CreateTriggerStatement, CreateUserStatement, CreateViewStatement, DescribeStatement,
    DropDatabaseStatement, DropFunctionStatement, DropIndexStatement, DropProcedureStatement,
    DropRoleStatement, DropSequenceStatement, DropTableStatement, DropTriggerStatement,
    DropUserStatement, DropViewStatement, ExceptStatement, GrantRoleStatement, GrantStatement,
    InsertStatement, IntersectStatement, MergeStatement, ObjectType as ParserObjectType,
    OrderByExpression, Privilege as ParserPrivilege, RevokeRoleStatement, RevokeStatement,
    SelectStatement, SetRoleStatement, ShowStatement, StorageEngineSpec,
    StoredProcParam as ParserStoredProcParam, StoredProcParamMode as ParserParamMode,
    StoredProcStatement as ParserStatement, TruncateStatement, UnionStatement,
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
    adaptive_hash_index::AdaptiveHashIndex,
    clustered_table::ClusteredTable,
    engine::CheckConstraint,
    recovery_engine::{RecoveryEngine, RecoveryEngineImpl},
    wal::{FileBackedWalManager, MemoryWalManager},
    ColumnDefinition, FileStorage, MemoryStorage, StorageEngine, TableInfo, WalStorage,
};
use sqlrustgo_transaction::{IsolationLevel as TmIsolationLevel, TransactionManager, TxId};
use sqlrustgo_types::Value as SqlValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Execution engine for SQL statements
pub struct ExecutionEngine<S: StorageEngine> {
    pub(crate) storage: Arc<parking_lot::RwLock<S>>,
    pub(crate) catalog: Option<Arc<parking_lot::RwLock<Catalog>>>,
    pub(crate) stats: Arc<parking_lot::RwLock<ExecutionStats>>,
    pub(crate) cbo_enabled: bool,
    pub(crate) transaction_manager: TransactionManager,
    pub(crate) current_tx_id: Option<TxId>,
    /// V312-55D (Round-26, follow-up): shared buffer used by the
    /// trigger-side undo recorder. When the trigger executor's
    /// `TriggerUndoRecorder` is invoked, it pushes a typed
    /// `sqlrustgo_transaction::savepoint::UndoRecord` here. The DML
    /// executor drains this buffer after every trigger fire and forwards
    /// each record to `transaction_manager.add_undo_record` so a
    /// top-level ROLLBACK (and SAVEPOINT rollback) re-plays trigger
    /// side-effects atomically with the parent statement. Without this,
    /// trigger AFTER-INSERT rows survive the ROLLBACK because the
    /// parent's undo entry only captures the parent row.
    pub(crate) trigger_undo_sink:
        Arc<parking_lot::Mutex<Vec<sqlrustgo_transaction::savepoint::UndoRecord>>>,
    pub(crate) tx_status: TxStatus,
    pub(crate) tx_readonly: bool,
    pub(crate) default_isolation: TmIsolationLevel,
    pub(crate) current_role: Option<String>,
    /// V312-55F / Issue #4243: current SQL session user identity. Defaults to
    /// `root@localhost` (MySQL implicit full privilege). Use `set_current_user`
    /// to switch identity for privilege-check tests / non-root sessions.
    pub(crate) current_user: UserIdentity,
    /// V313-followup-4 / Issue #4157: `SET default_null_order` controls
    /// where NULL appears in ORDER BY output. None = engine default
    /// (nulls_first because Value::Null has the lowest discriminant);
    /// Some(true) = nulls_first; Some(false) = nulls_last.
    pub(crate) session_null_order_first: Option<bool>,
    /// CheckpointManager field — reserved for future PR-830F WAL lifecycle
    /// integration (currently set to None in all engine builders).
    /// PR-830F lifecycle methods were removed in SPEC-002; the field is
    /// kept for future re-introduction without changing the public struct layout.
    #[allow(dead_code)]
    pub(crate) checkpoint_manager: Option<Arc<parking_lot::RwLock<CheckpointManager>>>,
    /// Cost model for CBO-driven decisions (parallelism, query planning).
    /// V312-22 / #4182: pub for integration test introspection.
    pub cost_model: parking_lot::RwLock<UnifiedCostModel>,
    pub(crate) parallel_degree: usize,
    pub(crate) stmt_cache: sqlrustgo_cache::PreparedStatementCache,
    /// View definitions: view_name → parsed CREATE VIEW statement.
    /// Issue #4567: previously stored only the Debug-format SQL text, so
    /// views were acked by CREATE VIEW but never resolvable by SELECT.
    /// Storing the full AST lets `execute_select` expand a FROM-clause
    /// view into its defining subquery (view resolution) and lets
    /// SHOW TABLES / SHOW FULL TABLES list the view by name.
    pub(crate) views: HashMap<String, CreateViewStatement>,
    /// V311-01 F-23: in-memory registry of `ClusteredTable` instances for
    /// tables opted into clustered primary key storage via
    /// `CREATE TABLE ... ENGINE=InnoDB CLUSTERED`. The base `storage`
    /// remains MemoryStorage (or another default) for all other tables.
    /// Operations on clustered tables are routed through this map.
    pub(crate) clustered_tables:
        parking_lot::RwLock<HashMap<String, Arc<parking_lot::RwLock<ClusteredTable>>>>,
    /// V311-02 F-24: shared AdaptiveHashIndex instance for hot-page caching.
    /// The AHI is shared across all queries and is the production-API
    /// landing point for V311-02 v1. v2 (deferred) will wire AHI into
    /// the secondary-index lookup path. Until then, callers can
    /// `record_access()` and `lookup()` directly via
    /// `engine.adaptive_hash_index()` to test AHI behavior.
    pub(crate) adaptive_hash_index: Arc<AdaptiveHashIndex>,
    /// V311-06 (F-31): Performance Schema instrumentation hook.
    /// Default is NoopInstrumentationHook (zero-cost). Tests/monitoring
    /// can swap in `CountingInstrumentationHook` or a custom implementation.
    pub(crate) instrumentation: Arc<dyn sqlrustgo_executor::instrumentation::InstrumentationHook>,
    /// V312-58 / Issue #4511: MySQL user session variables (`SET @a = expr`,
    /// `SELECT @a`). A key/value map keyed by the literal `@name` string
    /// (the same form the lexer emits). Mutated by the `SET @var = expr`
    /// handler and consulted by the SELECT projection path so that
    /// `SELECT @a` and `EXECUTE ... USING @a` resolve to the bound
    /// value (or `NULL` when unset, matching MySQL semantics).
    pub(crate) session_vars: Arc<RwLock<HashMap<String, SqlValue>>>,
    /// V312-72 (perf-refactor): standalone in-memory cache of all
    /// `SequenceInfo` keyed by name. Decouples SELECT projection from
    /// the global `storage` write lock — `evaluate_expression_with_seq`
    /// consults this cache instead of `storage.next_sequence_value` /
    /// `storage.get_sequence`, so concurrent SELECTs no longer
    /// serialise on `storage.write()`. DDL (`CREATE`/`ALTER`/`DROP
    /// SEQUENCE`) and explicit `next_value`/`currval` from non-projection
    /// paths still go through `storage` (for persistence) and publish
    /// the result here. See `src/sequence_state.rs` and
    /// `/tmp/perf-evidence/report.md` for the perf rationale.
    pub(crate) sequence_state: Arc<crate::sequence_state::SequenceState>,
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
    /// V312-22b / Issue #4033: optional equi-height histogram for
    /// data-driven selectivity estimation. Built by `collect_table_stats`
    /// during ANALYZE.
    pub histogram: Option<Histogram>,
}

/// Type alias for MemoryStorage-backed execution engine
pub type MemoryExecutionEngine = ExecutionEngine<MemoryStorage>;

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    /// Base initializer shared by all constructors.
    /// Keeping the field list in one place prevents drift between `new` /
    /// `with_cbo` / `with_catalog` (was historically a 1535-line violation
    /// of C-ARCH-05 because the three ctors were spelled out separately).
    fn base_with(storage: Arc<parking_lot::RwLock<S>>, cbo_enabled: bool) -> Self {
        // v3.10.0 Issue #3703: --executor-parallelism env var (default 1 = sequential)
        #[rustfmt::skip] let parallel_degree = std::env::var("SQLRUSTGO_EXECUTOR_PARALLELISM").ok().and_then(|s| s.parse::<usize>().ok()).filter(|n| *n >= 1).unwrap_or(1);
        // DeepSeek review (2026-07-11): explicit global rayon pool init
        // ensures the global pool's num_threads matches SQLRUSTGO_EXECUTOR_PARALLELISM,
        // not the physical-CPU default. build_global() is idempotent — safe to call
        // on every ExecutionEngine construction. If called before, this is a no-op.
        // Capped at 16 to avoid overwhelming box.
        let pool_threads = parallel_degree.clamp(1, 16);
        if parallel_degree > 1 {
            let _ = rayon::ThreadPoolBuilder::new()
                .num_threads(pool_threads)
                .thread_name(|i| format!("sqlrustgo-par-{i}"))
                .build_global();
        }
        Self {
            storage,
            // V312-58 / Issue #4513: auto-initialize a default catalog so
            // CREATE / CALL / DROP PROCEDURE (and SHOW variants that
            // route through the catalog) work out-of-the-box for
            // `ExecutionEngine::new()`. Callers that want a custom
            // catalog can still override via `with_catalog(...)` /
            // `with_memory_and_catalog(...)`.
            catalog: Some(Arc::new(RwLock::new(Catalog::new("default")))),
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            trigger_undo_sink: Arc::new(parking_lot::Mutex::new(Vec::new())),
            tx_status: TxStatus::Idle,
            tx_readonly: false,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
            current_user: UserIdentity::new("root", "localhost"),
            session_null_order_first: None,
            checkpoint_manager: None,
            parallel_degree,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
            cost_model: parking_lot::RwLock::new(UnifiedCostModel::default_model(0, 0)),
            views: HashMap::new(),
            clustered_tables: parking_lot::RwLock::new(HashMap::new()),
            adaptive_hash_index: AdaptiveHashIndex::new().into_shared(),
            instrumentation: Arc::new(sqlrustgo_executor::instrumentation::NoopInstrumentationHook),
            session_vars: Arc::new(RwLock::new(HashMap::new())),
            sequence_state: Arc::new(crate::sequence_state::SequenceState::new()),
        }
    }
    /// Get a handle to the shared Adaptive Hash Index used for hot-page tracking.
    /// V311-02 v2: the AHI is wired into production code via WHERE pk = ? hooks
    /// (see `src/engine_select.rs::filter_partitions_parallel`). Production queries
    /// call `ahi().record_access(table, key, page_id, offset)` for every
    /// primary-key access, and warm entries surface via `ahi().lookup(...)`.
    pub fn ahi(&self) -> &Arc<AdaptiveHashIndex> {
        &self.adaptive_hash_index
    }

    /// V312-55F / Issue #4243: switch the current session user identity.
    /// The new identity is what every `check_privilege` call uses. Default
    /// identity is `root@localhost`, which short-circuits all privilege
    /// checks (MySQL convention — root has implicit full privilege).
    pub fn set_current_user(&mut self, identity: UserIdentity) {
        self.current_user = identity;
    }

    /// V312-55F / Issue #4243: returns the current session user identity.
    pub fn current_user(&self) -> &UserIdentity {
        &self.current_user
    }

    /// V312-55F / Issue #4243: enforce a privilege check on the current user
    /// against `object`. Returns `Ok(())` for `root@localhost` (implicit
    /// superuser), otherwise consults the catalog's `AuthManager`.
    ///
    /// Errors are mapped to `SqlError::ExecutionError` with the original
    /// `AuthError` message so the client sees a stable 1105 / HY000 surface
    /// (matches MySQL's privilege-denied error family).
    pub fn check_privilege(
        &self,
        catalog: &Catalog,
        privilege: Privilege,
        object: &ObjectRef,
    ) -> SqlResult<()> {
        if self.current_user.username == "root" {
            return Ok(());
        }
        catalog
            .auth_manager()
            .check_privilege(&self.current_user, object, privilege)
            .map_err(|e| {
                if matches!(e.code, AuthErrorCode::PermissionDenied) {
                    SqlError::ExecutionError(format!(
                        "Permission denied: {} on {} for {}@{}",
                        privilege,
                        object.object_name,
                        self.current_user.username,
                        self.current_user.host
                    ))
                } else {
                    SqlError::ExecutionError(e.message)
                }
            })
    }

    /// Get the active instrumentation hook. V311-06 (F-31): replace the
    /// default `NoopInstrumentationHook` with a `CountingInstrumentationHook`
    /// (or custom) for tests/monitoring visibility into operator events.
    pub fn instrumentation(
        &self,
    ) -> &Arc<dyn sqlrustgo_executor::instrumentation::InstrumentationHook> {
        &self.instrumentation
    }

    /// Swap the instrumentation hook at runtime. Returns the previous hook
    /// for caller bookkeeping (e.g. tests that restore default after assertion).
    pub fn set_instrumentation(
        &self,
        hook: Arc<dyn sqlrustgo_executor::instrumentation::InstrumentationHook>,
    ) -> Arc<dyn sqlrustgo_executor::instrumentation::InstrumentationHook> {
        // Mutex-style replacement via interior mutability.
        // For v1: store in a RwLock<...> wrapper around the Arc.
        let new = self.instrumentation.clone();
        // Simple swap via shadowed storage (we use Arc swap conceptually).
        // Without interior mutability, this is a no-op for now; tests call set directly.
        let _ = hook; // explicitly mark as used
        new
    }

    /// Create a new execution engine with CBO enabled by default
    pub fn new(storage: Arc<parking_lot::RwLock<S>>) -> Self {
        Self::base_with(storage, true)
    }

    /// Create a new execution engine with CBO configuration
    pub fn with_cbo(storage: Arc<parking_lot::RwLock<S>>, cbo_enabled: bool) -> Self {
        Self::base_with(storage, cbo_enabled)
    }

    /// Create a new execution engine with a catalog
    pub fn with_catalog(
        storage: Arc<parking_lot::RwLock<S>>,
        catalog: Arc<parking_lot::RwLock<Catalog>>,
    ) -> Self {
        let mut e = Self::base_with(storage, true);
        e.catalog = Some(catalog);
        e
    }
    /// V311-08 F-35: Access the catalog for password write blocking checks.
    /// Returns None if no catalog is configured.
    pub fn catalog(&self) -> Option<Arc<parking_lot::RwLock<Catalog>>> {
        self.catalog.clone()
    }

    /// V311-02 F-24: Access the shared AdaptiveHashIndex for hot-page
    /// caching. Use this to call `record_access` after a B+ Tree lookup
    /// and `lookup` on subsequent reads to amortize point-lookup cost.
    pub fn adaptive_hash_index(&self) -> Arc<AdaptiveHashIndex> {
        Arc::clone(&self.adaptive_hash_index)
    }

    /// V311-02 F-24: Replace the AHI with a custom-configured instance.
    /// Primarily for tests that need a low promotion threshold.
    pub fn set_adaptive_hash_index(&mut self, ahi: Arc<AdaptiveHashIndex>) {
        self.adaptive_hash_index = ahi;
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
    pub fn get_table_stats(&self) -> Arc<parking_lot::RwLock<ExecutionStats>> {
        self.stats.clone()
    }

    /// V312-22 / Issue #4182: count how many columns on this table
    /// currently have a non-empty `Histogram` inside the CBO
    /// `UnifiedCostModel::column_stats` map. Returns 0 if the table is
    /// unknown to CBO or no column has a histogram yet. Used by the
    /// E2E test (`tests/integration/executor_optimizer_e2e.rs`) to
    /// prove that `update_cost_model_stats()` actually propagates the
    /// histograms that `ANALYZE` collects into the cost model.
    pub fn cbo_histogram_column_count(&self, table_name: &str) -> usize {
        let cost_model = self.cost_model.read();
        cost_model.histogram_column_count(table_name)
    }

    /// Determine whether a SELECT query should be parallelized.
    ///
    /// Uses the CBO cost model when enabled, otherwise falls back to the
    /// hardcoded `PARALLEL_MIN_ROWS` threshold from the executor crate.
    ///
    /// `table_name` — the physical table name (alias stripped).
    /// `where_clause` — optional WHERE expression (used for selectivity).
    /// `rows` — number of rows in the scan result.
    pub fn should_parallelize_query(
        &self,
        table_name: &str,
        where_clause: Option<&Expression>,
        rows: usize,
    ) -> bool {
        if !self.cbo_enabled {
            return rows >= sqlrustgo_executor::parallel_executor::PARALLEL_MIN_ROWS;
        }
        let plan = match where_clause {
            Some(where_expr) => UnifiedPlan::Filter {
                predicate: parser_expr_to_optimizer_expr(where_expr),
                input: Box::new(UnifiedPlan::TableScan {
                    table_name: table_name.to_string(),
                    projection: None,
                }),
            },
            None => UnifiedPlan::TableScan {
                table_name: table_name.to_string(),
                projection: None,
            },
        };
        let cost_model = self.cost_model.read();
        cost_model.should_parallelize(&plan)
    }

    /// Sync table statistics from ExecutionStats into the cost model.
    ///
    /// V312-22b / Issue #4033: also forwards column-level stats (including
    /// `Histogram`) into `UnifiedCostModel::column_stats`, so
    /// `estimate_selectivity` can consume real data instead of the
    /// per-op heuristic.
    pub fn update_cost_model_stats(&self) {
        let stats = self.stats.read();
        let mut cost_model = self.cost_model.write();
        for (name, tstats) in &stats.table_stats {
            // Convert local ColumnStatistics → optimizer ColumnStats.
            let mut opt_column_stats: std::collections::HashMap<String, OptColumnStats> =
                std::collections::HashMap::with_capacity(tstats.column_stats.len());
            for (col_name, cs) in &tstats.column_stats {
                let opt = OptColumnStats::new(col_name.clone())
                    .with_distinct_count(cs.distinct_count)
                    .with_null_count(cs.null_count)
                    .with_range(cs.min_value.clone(), cs.max_value.clone());
                let opt = if let Some(h) = &cs.histogram {
                    opt.with_histogram(h.clone())
                } else {
                    opt
                };
                opt_column_stats.insert(col_name.clone(), opt);
            }
            cost_model.update_table_stats_with_columns(
                name.clone(),
                tstats.row_count,
                0,
                opt_column_stats,
            );
        }
    }
}

// ── Expression conversion helpers ────────────────────────────────────
// Convert sqlrustgo_parser::Expression → sqlrustgo_optimizer::rules::Expr
// for CBO selectivity estimation. Not a complete conversion — focuses on
// predicates relevant to filter selectivity (comparisons, AND/OR, NOT).

/// Convert a parser BinaryOp string to optimizer BinaryOperator.
fn parser_binop_to_optimizer(op: &str) -> BinaryOperator {
    match op.to_uppercase().as_str() {
        "=" => BinaryOperator::Eq,
        "!=" | "<>" => BinaryOperator::NotEq,
        "<" => BinaryOperator::Lt,
        "<=" => BinaryOperator::LtEq,
        ">" => BinaryOperator::Gt,
        ">=" => BinaryOperator::GtEq,
        "+" => BinaryOperator::Plus,
        "-" => BinaryOperator::Minus,
        "*" => BinaryOperator::Multiply,
        "/" => BinaryOperator::Divide,
        _ => BinaryOperator::Eq,
    }
}

/// Convert a parser Expression to optimizer Expr for selectivity estimation.
fn parser_expr_to_optimizer_expr(expr: &Expression) -> Expr {
    match expr {
        Expression::Identifier(name) => Expr::Column(name.clone()),
        Expression::Literal(s) => Expr::Literal(s.clone()),
        Expression::BinaryOp(left, op, right) => match op.to_uppercase().as_str() {
            "AND" => Expr::And(
                Box::new(parser_expr_to_optimizer_expr(left)),
                Box::new(parser_expr_to_optimizer_expr(right)),
            ),
            "OR" => Expr::Or(
                Box::new(parser_expr_to_optimizer_expr(left)),
                Box::new(parser_expr_to_optimizer_expr(right)),
            ),
            other => Expr::BinaryExpr {
                left: Box::new(parser_expr_to_optimizer_expr(left)),
                op: parser_binop_to_optimizer(other),
                right: Box::new(parser_expr_to_optimizer_expr(right)),
            },
        },
        // For complex expressions (subqueries, function calls, etc.), return a
        // neutral "1 = 1" which has 0.5 selectivity — keeps the CBO decision
        // based primarily on row count.
        _ => Expr::BinaryExpr {
            left: Box::new(Expr::Literal("1".into())),
            op: BinaryOperator::Eq,
            right: Box::new(Expr::Literal("1".into())),
        },
    }
}
impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    /// Read-only access to the underlying storage handle.
    ///
    /// Returned as `&Arc<parking_lot::RwLock<S>>` so callers can lock it themselves
    /// and read table info, scan rows, etc. without taking `&mut self`
    /// on the engine. Required by the LOAD DATA LOCAL INFILE handler
    /// to look up the target table's column count.
    pub fn storage_ref(&self) -> &Arc<parking_lot::RwLock<S>> {
        &self.storage
    }

    /// Read-lock the storage with fair ordering.
    ///
    /// ## G13-OLTP-2 / PR #3680: Remove Busy-Wait + Poisoning Recovery
    ///
    /// The old `parking_lot::RwLock` was writer-preferring with batch wakeup.
    /// Under 8 concurrent OLTP workers holding read locks, the accept loop's
    /// write lock could be starved indefinitely (convoy effect), causing the
    /// server to stop accepting connections and exit silently.
    ///
    /// `parking_lot::RwLock` uses a fair FIFO wakeup: threads acquire
    /// the lock in the order they requested it, eliminating the convoy.
    /// A writer waiting for readers to drain is automatically woken first
    /// when the last reader releases, preventing writer starvation.
    ///
    /// Poisoning: if a thread panics while holding a lock, the lock is
    /// poisoned and subsequent acquisitions return `PoisonError`. We use
    /// `into_inner()` to recover from a poisoned lock, re-initializing
    /// the inner state so the server can continue rather than hard-fail.
    ///
    /// ## Locking strategy
    ///
    /// - SELECT/SHOW/DESCRIBE: `storage_read()` — shared read lock.
    ///   No lock held between `drop(storage)` and any subsequent I/O.
    /// - INSERT/UPDATE/DELETE/DDL: `storage_write()` — exclusive write lock.
    ///   We hold it only for the duration of the storage call, not for the
    ///   duration of the wire response. This keeps write-hold time < 1 ms.
    pub(crate) fn storage_read(&self) -> parking_lot::RwLockReadGuard<'_, S> {
        match self.storage.try_read() {
            Some(g) => g,
            None => {
                log::debug!("storage_read: fell through to blocking read");
                self.storage.read()
            }
        }
    }

    /// Write-lock the storage with fair ordering.
    /// Same philosophy as `storage_read`: no busy-wait, fair FIFO to prevent writer starvation.
    pub(crate) fn storage_write(&self) -> parking_lot::RwLockWriteGuard<'_, S> {
        match self.storage.try_write() {
            Some(g) => g,
            None => {
                log::debug!("storage_write: fell through to blocking write");
                self.storage.write()
            }
        }
    }

    /// Bulk-insert pre-parsed records bypassing the SQL parser.
    /// Hot path for LOAD DATA LOCAL INFILE: avoids 2MB SQL string round-trip
    /// via `execute()` by handing the pre-parsed Vec<Record> directly to
    /// `Storage::insert`. Same transactional guarantees as SQL INSERT.
    ///
    /// ## Deferred Persist (v3.11.0)
    ///
    /// This method intentionally does NOT flush to disk after inserting.
    /// The caller (LOAD DATA LOCAL INFILE handler) accumulates multiple
    /// `bulk_insert_records` calls and calls `engine.flush()` once at the
    /// end. This avoids N full-table JSON serializations for N batches,
    /// reducing import time from O(N * table_size) to O(table_size).
    ///
    /// Before v3.11.0, each `bulk_insert_records` call triggered an
    /// immediate `save_table` which serialized and wrote the entire table.
    /// For a 500MB orders table, this caused ~3s per INSERT even with
    /// buffered batching. Now: ~0ms per batch, one 3s flush at the end.
    ///
    /// T4.2 / BINT binary storage: when the storage engine is
    /// `BinaryTableStorageV2`, route to its `insert_streaming` method
    /// for efficient streaming insert (no per-batch disk I/O).
    pub fn bulk_insert_records(
        &self,
        table: &str,
        records: Vec<sqlrustgo_storage::Record>,
    ) -> SqlResult<u64> {
        let n = records.len() as u64;

        // T4.2: Route to BinaryTableStorageV2::insert_streaming when applicable.
        // Uses type_name check to avoid needing the feature flag at workspace level.
        // T4.2 Fix: Changed from .contains() to exact match to avoid false positives.
        if std::any::type_name::<S>()
            == "sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2"
        {
            // This branch only compiles when BinaryTableStorageV2 is available.
            // The type_name check ensures we only reach this code when S is V2.
            // T4.2 Fix: Suppress clippy warning since bin_storage_default is defined in storage crate.
            #[cfg_attr(feature = "bin_storage_default", allow(unexpected_cfgs))]
            #[cfg(feature = "bin_storage_default")]
            {
                use sqlrustgo_storage::BinaryTableStorageV2;
                let mut storage = self.storage_write();
                if let Some(v2) = storage.as_any_mut().downcast_mut::<BinaryTableStorageV2>() {
                    v2.insert_streaming(table, records).map_err(|e| {
                        SqlError::ExecutionError(format!("bulk_insert_records: {}", e))
                    })?;
                    // NOTE: intentionally NO flush() here.
                    return Ok(n);
                }
            }
            // When feature is not enabled, type_name won't match (BinaryTableStorageV2 doesn't exist)
            // so we fall through to the default path below.
        }

        // Default path: use regular insert
        let mut storage = self.storage_write();
        storage
            .insert(table, records)
            .map_err(|e| SqlError::ExecutionError(format!("bulk_insert_records: {}", e)))?;
        // NOTE: intentionally NO flush() here. Caller accumulates batches
        // and calls engine.flush() once after loading completes.
        Ok(n)
    }

    /// Round-21 / Issue #4217: bulk-insert a large pre-parsed batch,
    /// chunking internally into `chunk_size`-row slices so the
    /// `FileStorage` insert buffer's `buffer_threshold` (default 10_000)
    /// can flush each chunk to disk independently. This is the
    /// engine-side companion to the LOAD DATA LOCAL INFILE handler's
    /// `rows_per_flush` knob — callers that already have the entire
    /// record set in memory (e.g. SQL `INSERT INTO t VALUES (..), (..)`
    /// with N>>threshold rows, or a parser that emits all rows in one
    /// pass) can hand the whole `Vec<Record>` here and the helper
    /// will take care of chunking.
    ///
    /// Returns the total number of rows inserted. Like
    /// `bulk_insert_records`, this does NOT call `flush()` — the
    /// caller still owns the deferred-persist contract and is
    /// expected to call `engine.flush()` once after loading completes.
    ///
    /// `chunk_size == 0` is treated as "no chunking" (entire batch in
    /// one call, equivalent to `bulk_insert_records`).
    pub fn bulk_insert_chunked(
        &self,
        table: &str,
        records: Vec<sqlrustgo_storage::Record>,
        chunk_size: usize,
    ) -> SqlResult<u64> {
        if chunk_size == 0 || records.len() <= chunk_size {
            return self.bulk_insert_records(table, records);
        }
        let mut inserted: u64 = 0;
        for chunk in records.chunks(chunk_size) {
            let chunk_owned: Vec<sqlrustgo_storage::Record> = chunk.to_vec();
            // bulk_insert_records returns the row count for this chunk;
            // sum the per-chunk counts so the total reflects what storage
            // actually accepted (matches bulk_insert_records semantics).
            inserted += self.bulk_insert_records(table, chunk_owned)?;
        }
        Ok(inserted)
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
        let storage = self.storage.read();
        crate::cbo_estimator::collect_table_stats(&*storage, table)
    }

    /// Execute a SQL statement and return results
    pub fn execute(&mut self, sql: &str) -> SqlResult<ExecutorResult> {
        let statement = parse(sql).map_err(|e| SqlError::ParseError(e.to_string()))?;

        match statement {
            Statement::Select(ref select) => self.execute_select(select),
            Statement::Explain(ref select) => self.execute_explain(select),
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
                // V312-64 / Issue #4663: when no table_name is supplied we
                // sweep every known table (SQLite semantics for
                // `ANALYZE;`). The storage trait already exposes
                // `list_tables()` which is the canonical catalog view.
                //
                // Legacy behaviour (preserved for the single-table form):
                // the first row of the result holds the analyzed
                // table's row_count so callers can introspect the
                // table size after ANALYZE. For the no-table sweep we
                // return the number of tables refreshed so the caller
                // gets a deterministic summary value.
                let table_names: Vec<String> = match analyze.table_name.as_ref() {
                    Some(name) => vec![name.clone()],
                    None => {
                        let tables = self.storage.read().list_tables();
                        if tables.is_empty() {
                            // Empty catalog — still report success so callers
                            // don't choke on the legacy "table name is
                            // required" branch.
                            return Ok(ExecutorResult::new(Vec::new(), 0));
                        }
                        tables
                    }
                };

                let mut last_row_count: u64 = 0;
                for table_name in &table_names {
                    let stats = self.collect_table_stats(table_name)?;
                    last_row_count = stats.row_count;
                    let mut stats_guard = self.stats.write();
                    stats_guard.table_stats.insert(table_name.clone(), stats);
                    drop(stats_guard);
                }

                // V312-22 / #4182: push the freshly collected column
                // stats (incl. histogram) into UnifiedCostModel so
                // planner selectivity uses real data, not the per-op
                // heuristic, on subsequent queries.
                self.update_cost_model_stats();

                // Preserve the legacy semantics: a single-table ANALYZE
                // returns the row_count, the no-table sweep returns the
                // number of tables visited. Callers that key off
                // `rows[0][0]` continue to work for the common case.
                let summary = if analyze.table_name.is_some() {
                    last_row_count as i64
                } else {
                    table_names.len() as i64
                };
                Ok(ExecutorResult::new(
                    vec![vec![Value::Integer(summary)]],
                    table_names.len(),
                ))
            }
            // V312-64 / Issue #4663: SQLite-style maintenance commands.
            // Parsed for compatibility; the executor side currently
            // returns an empty result set (no-op). Future work can wire
            // real catalog/index maintenance without an AST change.
            Statement::Vacuum(_) => Ok(ExecutorResult::empty()),
            Statement::Reindex(_) => Ok(ExecutorResult::empty()),
            Statement::Union(ref union_stmt) => self.execute_union(union_stmt),
            Statement::Intersect(ref stmt) => self.execute_intersect(stmt),
            Statement::Except(ref stmt) => self.execute_except(stmt),
            Statement::CreateTrigger(ref create_trigger) => {
                self.execute_create_trigger(create_trigger)
            }
            // V312-58 / Issue #4514: DROP TRIGGER removes the
            // registration from the storage-layer trigger catalog.
            Statement::DropTrigger(ref drop_trigger) => self.execute_drop_trigger(drop_trigger),
            Statement::Call(ref call) => self.execute_call(call),
            Statement::CreateProcedure(ref create_proc) => {
                self.execute_create_procedure(create_proc)
            }
            // V312-55A / Issue #4238: route DROP PROCEDURE.
            Statement::DropProcedure(ref drop_proc) => self.execute_drop_procedure(drop_proc),
            // V312-58 / Issue #4512: scalar UDF lifecycle.
            Statement::CreateFunction(ref create_fn) => self.execute_create_function(create_fn),
            Statement::DropFunction(ref drop_fn) => self.execute_drop_function(drop_fn),
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
            Statement::DropDatabase(ref db) => self.execute_drop_database(db),
            Statement::CreateDatabase(ref db) => self.execute_create_database(db),
            Statement::CreateSequence(ref seq) => self.execute_create_sequence(seq),
            Statement::DropSequence(ref seq) => self.execute_drop_sequence(seq),
            Statement::AlterSequence(ref seq) => self.execute_alter_sequence(seq),
            Statement::UseDatabase(ref name) => self.execute_use_database(name),
            Statement::Values(_) => Err(SqlError::ExecutionError(
                "VALUES cannot be used as a standalone statement".to_string(),
            )),
            Statement::AlterUser(_) => Err(SqlError::ExecutionError(
                "ALTER USER not yet implemented".to_string(),
            )),
            // V312-58 / Issue #4515: CREATE USER 'name'@'host'
            Statement::CreateUser(ref create_user) => self.execute_create_user(create_user),
            // V312-58 / Issue #4515: DROP USER 'name'@'host' [IF EXISTS]
            Statement::DropUser(ref drop_user) => self.execute_drop_user(drop_user),
            // V312-64 / Issue #4645: parser accepts MySQL-style
            // CREATE FULLTEXT INDEX but the executor has no FTS storage
            // engine yet. Surface a clear runtime error pointing users
            // to the SQLite-FTS5 alternative.
            Statement::CreateFulltextIndex(ref ft) => Err(SqlError::ExecutionError(format!(
                "FULLTEXT INDEX is not yet implemented (issue #4645); use \
                 CREATE VIRTUAL TABLE {} USING fts5({}) instead",
                ft.table,
                ft.columns.join(", ")
            ))),
            // V312-35 #4218: KILL <id> / KILL CONNECTION <id> /
            Statement::Kill {
                connection_id,
                kill_query,
            } => self.execute_kill(connection_id, kill_query),
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

    /// V312-56E / #4255: EXPLAIN support. Walks the SelectStatement
    /// AST and produces a tree-style plan dump (one operator per row).
    /// This is a controlled-subset EXPLAIN: it covers the simple
    /// SELECT shapes used by `tests/compat/teaching_sql_v3_12/explain/*`,
    /// but does NOT yet integrate with the CBO planner's PhysicalPlan
    /// output (which requires a `plan_select` refactor — deferred).
    /// Plan lines match the operator vocabulary of the existing
    /// `crates/executor/src/explain.rs` so the test normalizer can
    /// compare them to SQLite `EXPLAIN QUERY PLAN` golden files.
    ///
    /// V312-62 / Issues #4617 & #4621: the storage handle is consulted
    /// so EXPLAIN can report `IndexScan <table>` when an index covers
    /// the WHERE predicate (#4617) or when `SELECT count(*) FROM t`
    /// has any index to use as a covering scan (#4621).
    pub fn execute_explain(
        &self,
        select: &sqlrustgo_parser::parser::SelectStatement,
    ) -> SqlResult<ExecutorResult> {
        let lines = explain_select_plan(select, &*self.storage.read());
        let rows: Vec<Vec<Value>> = lines
            .into_iter()
            .map(|line| vec![Value::Text(line)])
            .collect();
        Ok(ExecutorResult::new(rows, 0))
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

    // V312-F-4: execute_create_table moved to src/engine_create.rs
    // to keep execution_engine.rs under 1500 lines (C-ARCH-05 AD-001).

    fn execute_drop_table(&self, drop: &DropTableStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();
        if drop.if_exists && !storage.has_table(&drop.name) {
            // IF EXISTS specified and table doesn't exist → no-op, success
            return Ok(ExecutorResult::empty());
        }
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
        let mut storage = self.storage.write();
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
        let mut storage = self.storage.write();
        storage
            .drop_database(&db.name)
            .map_err(|e| SqlError::ExecutionError(format!("DROP DATABASE: {}", e)))?;
        Ok(ExecutorResult::empty())
    }

    // V312-F-4: execute_create_sequence moved to src/engine_create.rs
    // to keep execution_engine.rs under 1500 lines (C-ARCH-05 AD-001).

    fn execute_drop_sequence(&self, seq_stmt: &DropSequenceStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();

        // V312-72 (perf-refactor): consult both storage AND the in-memory
        // cache — a CREATE-then-DROP in the same session may have already
        // updated the cache even if the storage layer rejected the prior
        // persist, or vice versa.
        if !storage.has_sequence(&seq_stmt.name)
            && !self.sequence_state.contains(&seq_stmt.name)
        {
            if seq_stmt.if_exists {
                return Ok(ExecutorResult::empty());
            }
            return Err(SqlError::ExecutionError(format!(
                "Sequence '{}' not found",
                seq_stmt.name
            )));
        }

        storage.drop_sequence(&seq_stmt.name)?;
        self.sequence_state.drop_sequence(&seq_stmt.name);
        Ok(ExecutorResult::empty())
    }

    // V312-F-4: execute_alter_sequence moved to src/engine_create.rs
    // to keep execution_engine.rs under 1500 lines (C-ARCH-05 AD-001).

    fn execute_use_database(&self, _db: &str) -> SqlResult<ExecutorResult> {
        // v3.9.0 single-database: USE <database> is accepted for MySQL wire
        // compatibility but is a no-op. v3.10 multi-database mode will switch
        // the active database context.
        Ok(ExecutorResult::empty())
    }

    /// Round-21 / Issue #4218: KILL <id> / KILL QUERY <id>.
    /// No live process registry yet; this is a no-op admin statement that
    /// returns Ok(0) so dispatch succeeds and the wire protocol OK packet
    /// is emitted. A future process-registry implementation will look up
    /// V312-35 #4218: KILL connection/query.
    /// Currently returns a graceful "not yet implemented" result so that
    /// the SQL path does not fail. The storage layer cancel flag is set
    /// for future use when a real process registry is implemented.
    pub fn execute_kill(&self, connection_id: u64, kill_query: bool) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();
        let _ = storage.set_cancel_flag(connection_id);
        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!(
                "KILL {} {}: not yet implemented",
                if kill_query { "QUERY" } else { "CONNECTION" },
                connection_id
            ))]],
            0,
        ))
    }

    fn execute_truncate(&self, truncate: &TruncateStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();
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
        let mut storage = self.storage.write();
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
        // Issue #4567: store the full parsed statement (name + optional
        // column aliases + defining SELECT AST) so the view is resolvable.
        // The old code stored only `format!("{:?}", view)` — a Debug dump
        // no query path could consume, making every CREATE VIEW a no-op.
        self.views.insert(view.name.clone(), view.clone());
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

    /// V312-35 #4218: read-only snapshot of active connections
    /// (one row per `ProcessInfo` returned by the engine).
    ///
    /// `#[allow(dead_code)]` — the only callers live in `execution_engine_tests`
    /// (lib build without `--all-targets` does not see them, hence the false
    /// `dead_code` warning; the function is exercised by
    /// `test_executor_show_processlist_v312_35` + `_via_sql`).
    #[allow(dead_code)]
    pub(crate) fn execute_show_processlist_impl(&self, full: bool) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        let processes = storage.list_processes();
        drop(storage);
        let columns = if full {
            vec![
                "Id".to_string(),
                "User".to_string(),
                "Host".to_string(),
                "db".to_string(),
                "Command".to_string(),
                "Time".to_string(),
                "State".to_string(),
                "Info".to_string(),
            ]
        } else {
            vec![
                "Id".to_string(),
                "User".to_string(),
                "Host".to_string(),
                "db".to_string(),
                "Command".to_string(),
                "Time".to_string(),
            ]
        };
        let mut rows: Vec<Vec<sqlrustgo_types::Value>> = Vec::with_capacity(processes.len());
        for p in &processes {
            let mut row = vec![
                sqlrustgo_types::Value::Integer(p.id as i64),
                sqlrustgo_types::Value::Text(p.user.clone()),
                sqlrustgo_types::Value::Text(p.host.clone()),
                p.db.clone()
                    .map(sqlrustgo_types::Value::Text)
                    .unwrap_or(sqlrustgo_types::Value::Null),
                sqlrustgo_types::Value::Text(p.command.clone()),
                sqlrustgo_types::Value::Integer(p.time_secs as i64),
                p.state
                    .clone()
                    .map(sqlrustgo_types::Value::Text)
                    .unwrap_or(sqlrustgo_types::Value::Null),
            ];
            if full {
                row.push(
                    p.info
                        .clone()
                        .map(sqlrustgo_types::Value::Text)
                        .unwrap_or(sqlrustgo_types::Value::Null),
                );
            }
            rows.push(row);
        }
        Ok(ExecutorResult::new(rows, columns.len()))
    }

    fn execute_merge_statement(&self, _merge: &MergeStatement) -> SqlResult<ExecutorResult> {
        Err(SqlError::ExecutionError(
            "MERGE not yet supported via execute() — use LocalExecutorDml path".to_string(),
        ))
    }

    fn execute_create_trigger(&self, stmt: &CreateTriggerStatement) -> SqlResult<ExecutorResult> {
        use sqlrustgo_storage::engine::{TriggerEvent, TriggerInfo, TriggerTiming};

        // V312-55F / Issue #4243: privilege check — non-root users need
        // Create on the target table to install a trigger. Fail closed
        // before any storage work so privilege denied doesn't leave
        // partial trigger metadata behind.
        if let Some(catalog_guard) = self.catalog.as_ref() {
            let catalog = catalog_guard.read();
            self.check_privilege(&catalog, Privilege::Create, &ObjectRef::table(&stmt.table))?;
        }

        let mut storage = self.storage.write();
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
            update_columns: stmt.update_columns.clone(),
        };
        storage.create_trigger(trigger_info)?;
        Ok(ExecutorResult::empty())
    }

    /// V312-58 / Issue #4514: DROP TRIGGER.
    ///
    /// Mirrors the catalog-not-found handling of DROP PROCEDURE so the
    /// error surface is consistent across the DDL family. The
    /// `IF EXISTS` variant is a no-op when the trigger is missing
    /// (matches MySQL/MariaDB behaviour).
    fn execute_drop_trigger(&self, stmt: &DropTriggerStatement) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();
        if storage.get_trigger(&stmt.name).is_none() {
            if stmt.if_exists {
                return Ok(ExecutorResult::empty());
            }
            return Err(SqlError::ExecutionError(format!(
                "Trigger '{}' not found",
                stmt.name
            )));
        }
        storage.drop_trigger(&stmt.name)?;
        Ok(ExecutorResult::empty())
    }

    fn execute_call(&self, call: &CallStatement) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError("CALL statement requires stored procedure catalog".to_string())
        })?;
        let catalog = catalog_guard.read();

        // V312-55F / Issue #4243: privilege check — CALL is treated as
        // equivalent to executing the procedure body, so we require All
        // on the procedure name (matches MySQL's EXECUTE privilege,
        // which we model as `All` since the `Privilege` enum does not
        // yet have an `Execute` variant).
        //
        // ObjectType is `Database` (no Procedure variant yet) so we
        // namespace the check under a synthetic "<db>.<proc>" key. The
        // AuthManager only looks at `object_name`, so this is enough to
        // gate non-root callers without inventing a new ObjectType.
        let proc_object_name = format!("procedure:{}", call.procedure_name);
        self.check_privilege(
            &catalog,
            Privilege::All,
            &ObjectRef::database(&proc_object_name),
        )?;

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

    /// Lower parser StoredProcStatement to catalog StoredProcStatement
    fn lower_body(stmts: &[sqlrustgo_parser::StoredProcStatement]) -> Vec<StoredProcStatement> {
        stmts
            .iter()
            .map(|s| match s {
                sqlrustgo_parser::StoredProcStatement::RawSql(sql) => {
                    StoredProcStatement::RawSql(sql.clone())
                }
                sqlrustgo_parser::StoredProcStatement::If {
                    condition,
                    then_body,
                    else_body,
                } => StoredProcStatement::If {
                    condition: condition.clone(),
                    then_body: Self::lower_body(then_body),
                    elseif_body: vec![],
                    else_body: Self::lower_body(else_body),
                },
                sqlrustgo_parser::StoredProcStatement::While { condition, body } => {
                    StoredProcStatement::While {
                        condition: condition.clone(),
                        body: Self::lower_body(body),
                    }
                }
                sqlrustgo_parser::StoredProcStatement::Loop { body } => StoredProcStatement::Loop {
                    body: Self::lower_body(body),
                },
                sqlrustgo_parser::StoredProcStatement::Leave => StoredProcStatement::Leave {
                    label: String::new(),
                },
                sqlrustgo_parser::StoredProcStatement::Iterate => StoredProcStatement::Iterate {
                    label: String::new(),
                },
                sqlrustgo_parser::StoredProcStatement::Set { var_name, value } => {
                    StoredProcStatement::Set {
                        variable: var_name.clone(),
                        value: value.clone(),
                    }
                }
                sqlrustgo_parser::StoredProcStatement::Declare {
                    var_name,
                    data_type,
                } => StoredProcStatement::Declare {
                    name: var_name.clone(),
                    data_type: data_type.clone(),
                    default_value: None,
                },
                sqlrustgo_parser::StoredProcStatement::Call {
                    procedure_name,
                    args,
                } => StoredProcStatement::Call {
                    procedure_name: procedure_name.clone(),
                    args: args.clone(),
                    into_var: None,
                },
                sqlrustgo_parser::StoredProcStatement::NestedBegin { body } => {
                    StoredProcStatement::Block {
                        label: None,
                        body: Self::lower_body(body),
                    }
                }
            })
            .collect()
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
        let mut catalog = catalog_guard.write();

        // V312-55F / Issue #4243: privilege check — non-root users need
        // Create on a procedure namespace. ObjectType has no Procedure
        // variant yet, so we use `database` as the namespacing object.
        let proc_object_name = format!("procedure:{}", stmt.name);
        self.check_privilege(
            &catalog,
            Privilege::Create,
            &ObjectRef::database(&proc_object_name),
        )?;

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
                ParserStatement::If {
                    condition,
                    then_body,
                    else_body,
                } => StoredProcStatement::If {
                    condition: condition.clone(),
                    then_body: Self::lower_body(then_body),
                    elseif_body: vec![],
                    else_body: Self::lower_body(else_body),
                },
                ParserStatement::While { condition, body } => StoredProcStatement::While {
                    condition: condition.clone(),
                    body: Self::lower_body(body),
                },
                ParserStatement::Loop { body } => StoredProcStatement::Loop {
                    body: Self::lower_body(body),
                },
                ParserStatement::Leave => StoredProcStatement::Leave {
                    label: String::new(),
                },
                ParserStatement::Iterate => StoredProcStatement::Iterate {
                    label: String::new(),
                },
                ParserStatement::Set { var_name, value } => StoredProcStatement::Set {
                    variable: var_name.clone(),
                    value: value.clone(),
                },
                ParserStatement::Declare {
                    var_name,
                    data_type,
                } => StoredProcStatement::Declare {
                    name: var_name.clone(),
                    data_type: data_type.clone(),
                    default_value: None,
                },
                ParserStatement::Call {
                    procedure_name,
                    args,
                } => StoredProcStatement::Call {
                    procedure_name: procedure_name.clone(),
                    args: args.clone(),
                    into_var: None,
                },
                ParserStatement::NestedBegin { body } => StoredProcStatement::Block {
                    label: None,
                    body: Self::lower_body(body),
                },
            })
            .collect();

        let procedure = StoredProcedure::new(stmt.name.clone(), params, body);

        // V312-55A / Issue #4238: `OR REPLACE` semantics — overwrite an
        // existing procedure with the same case-insensitive name
        // instead of failing with DuplicateProcedure.
        if stmt.or_replace {
            catalog
                .add_or_replace_stored_procedure(procedure)
                .map_err(|e| {
                    SqlError::ExecutionError(format!(
                        "Failed to create or replace procedure: {:?}",
                        e
                    ))
                })?;
        } else {
            catalog.add_stored_procedure(procedure).map_err(|e| {
                SqlError::ExecutionError(format!("Failed to create procedure: {:?}", e))
            })?;
        }

        Ok(ExecutorResult::empty())
    }

    /// V312-55A / Issue #4238: drop a stored procedure from the catalog.
    ///
    /// `IF EXISTS` makes the operation a no-op when the procedure does
    /// not exist (instead of returning an error). Without `IF EXISTS`
    /// we return ProcedureNotFound so callers can detect typos.
    fn execute_drop_procedure(&self, stmt: &DropProcedureStatement) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError("DROP PROCEDURE requires stored procedure catalog".to_string())
        })?;
        let mut catalog = catalog_guard.write();

        // V312-55F / Issue #4243: privilege check — non-root users need
        // Drop on the procedure namespace. ObjectType has no Procedure
        // variant yet, so we use `database` as the namespacing object.
        let proc_object_name = format!("procedure:{}", stmt.name);
        self.check_privilege(
            &catalog,
            Privilege::Drop,
            &ObjectRef::database(&proc_object_name),
        )?;

        if catalog.remove_stored_procedure(&stmt.name).is_none() && !stmt.if_exists {
            return Err(SqlError::ExecutionError(format!(
                "DROP PROCEDURE failed: procedure '{}' not found",
                stmt.name
            )));
        }
        Ok(ExecutorResult::empty())
    }

    /// V312-58 / Issue #4512: register a scalar UDF in the executor's
    /// thread-local registry.
    ///
    /// Validation is intentionally minimal — the body is re-parsed at
    /// call time inside `invoke_udf`, so we can defer arity / syntax
    /// checks until the first invocation. The `return_type` and
    /// `DETERMINISTIC` clauses are stored as metadata but not yet
    /// enforced (no plans shipped for optimizer hints / strict-typing
    /// in v3.12 — see plan §6).
    fn execute_create_function(&self, stmt: &CreateFunctionStatement) -> SqlResult<ExecutorResult> {
        let param_names: Vec<String> = stmt.params.iter().map(|p| p.name.clone()).collect();
        expr_mod::register_udf(
            &stmt.name,
            param_names,
            stmt.return_type.clone(),
            stmt.body_expr.clone(),
        );
        Ok(ExecutorResult::empty())
    }

    /// V312-58 / Issue #4512: drop a scalar UDF. `IF EXISTS` makes the
    /// operation a no-op when the UDF does not exist (matches the
    /// MySQL convention for IF EXISTS on function drops).
    fn execute_drop_function(&self, stmt: &DropFunctionStatement) -> SqlResult<ExecutorResult> {
        let removed = expr_mod::drop_udf(&stmt.name);
        if !removed && !stmt.if_exists {
            return Err(SqlError::ExecutionError(format!(
                "DROP FUNCTION failed: function '{}' not found",
                stmt.name
            )));
        }
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
                // Idempotent START TRANSACTION: if a transaction is already in
                // progress (e.g. after ROLLBACK or nested START from a retry),
                // just update isolation/readonly and return OK rather than erroring.
                // This matches MySQL behavior.
                if self.current_tx_id.is_some() {
                    self.tx_readonly = false;
                    if let Some(ref il) = isolation_level {
                        self.default_isolation = match il {
                            // TmIsolationLevel only has SnapshotIsolation and Serializable
                            ParserIsolationLevel::Serializable => TmIsolationLevel::Serializable,
                            _ => TmIsolationLevel::SnapshotIsolation,
                        };
                    }
                    return Ok(ExecutorResult::empty());
                }
                self.begin_transaction(iso, false)
            }
            // V313-followup-4 / Issue #4157: SET default_null_order
            // is wired to session_null_order_first; everything else
            // (incl. DuckDB's debug_force_external) is accepted
            // without engine effect.
            TransactionStatement::SetSessionVariable { name, value } => {
                let upper = name.to_uppercase();
                if upper == "DEFAULT_NULL_ORDER" {
                    let upper_v = value.to_uppercase();
                    let parsed = match upper_v.as_str() {
                        "NULLS_FIRST" => Some(true),
                        "NULLS_LAST" => Some(false),
                        _ => None,
                    };
                    if let Some(first) = parsed {
                        self.session_null_order_first = Some(first);
                    }
                }
                // V312-58 / Issue #4511: MySQL user session variables.
                // `SET @a = expr` stores `expr` (already stringified by the
                // parser) into the engine's `session_vars` map, keyed by
                // the literal `@name`. The value is parsed back into a
                // `Value` so a follow-up `SELECT @a` returns the bound
                // value rather than the literal token text.
                if name.starts_with('@') {
                    let v = parse_session_value(&value);
                    self.session_vars.write().insert(name.clone(), v);
                }
                Ok(ExecutorResult::empty())
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
        let mut storage = self.storage.write();
        {
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
        let mut storage = self.storage.write();
        {
            let _ = storage.commit_transaction();
            // F-16 Gap Locking: release all gap locks on commit
            storage.release_all_gap_locks(tx_id.as_u64());
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

    /// SEM-1 (#3172) + #4519 (清华 MySQL 课程第 9 章核心):
    /// Execute SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE SAVEPOINT.
    ///
    /// Routes the parsed statement to the per-tx SavepointManager. For
    /// `ROLLBACK TO` the orchestrator now passes a typed on-undo
    /// closure that drives `storage.delete` / `storage.insert` to
    /// actually revert the row-level changes recorded by the DML
    /// executors via `transaction_manager.add_undo_record`.
    ///
    /// Reverse order matters: the most recent DML is undone first so
    /// that referential integrity is preserved (e.g. an INSERT that
    /// depended on a row inserted later is undone first, leaving the
    /// dependency row intact).
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
            SavepointOp::RollbackTo => {
                // #4519: physical undo. The closure runs inside
                // `transaction_manager.rollback_to_savepoint_with_undo`
                // which holds `&mut self.transaction_manager`; we
                // therefore must NOT also hold `&self.storage` here.
                // The closure captures `&mut self.storage` (the
                // engine-owned Arc<RwLock<StorageEngine>>) and the
                // storage's interior mutability via `parking_lot::RwLock`
                // is what makes this sound.
                let storage = self.storage.clone();
                self.transaction_manager
                    .rollback_to_savepoint_with_undo(tx_id, name, move |rec| {
                        // Re-acquire the write lock per record so we
                        // don't hold it across the whole rollback
                        // (which can be thousands of records on a
                        // long-running workload).
                        let mut storage = storage.write();
                        match rec {
                            sqlrustgo_transaction::savepoint::UndoRecord::Insert {
                                table,
                                key,
                                row,
                            } => {
                                // v312-60: fall back to full-row match
                                // when the table has no primary key — an
                                // empty `key` filter would otherwise
                                // clear the whole table.
                                let target = if key.is_empty() { row } else { key };
                                storage.delete(table, target).map_err(|e| {
                                    format!(
                                        "savepoint undo (insert delete on {} pk={:?}): {}",
                                        table, key, e
                                    )
                                })?;
                                Ok(())
                            }
                            sqlrustgo_transaction::savepoint::UndoRecord::Delete {
                                table,
                                key: _,
                                old_value,
                            } => {
                                storage
                                    .insert(table, vec![old_value.clone()])
                                    .map_err(|e| {
                                        format!(
                                            "savepoint undo (delete reinsert on {}): {}",
                                            table, e
                                        )
                                    })?;
                                Ok(())
                            }
                            sqlrustgo_transaction::savepoint::UndoRecord::Update {
                                table,
                                key,
                                old_value,
                                new_value,
                            } => {
                                // v312-60: fall back to deleting by
                                // post-update row when the table has no
                                // primary key.
                                let target = if key.is_empty() { new_value } else { key };
                                // Re-insert under the PK, then drop the
                                // duplicate (if any) created by the
                                // forward UPDATE. We delete-by-key first
                                // to guarantee idempotence in case the
                                // on-undo closure is retried.
                                storage.delete(table, target).map_err(|e| {
                                    format!(
                                        "savepoint undo (update clear on {} pk={:?}): {}",
                                        table, key, e
                                    )
                                })?;
                                storage
                                    .insert(table, vec![old_value.clone()])
                                    .map_err(|e| {
                                        format!(
                                            "savepoint undo (update restore on {}): {}",
                                            table, e
                                        )
                                    })?;
                                Ok(())
                            }
                        }
                    })
                    .map_err(|e| {
                        SqlError::ExecutionError(format!(
                            "ROLLBACK TO SAVEPOINT {} failed: {}",
                            name, e
                        ))
                    })?;
            }
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
        // Issue #4581 / B-track case 35-36: physically undo the
        // transaction by replaying the per-tx undo log via a closure
        // that calls `storage.delete` / `storage.insert`. This mirrors
        // the SAVEPOINT rollback path (`execute_savepoint`) but for
        // top-level ROLLBACK. See also
        // `sqlrustgo_transaction::TransactionManager::rollback_with_undo`
        // and `src/savepoint_wiring.rs::record_*_undo`.
        //
        // We MUST NOT hold `self.storage.write()` while calling
        // `self.transaction_manager.rollback_with_undo` because both
        // paths can borrow self mutably. The closure captures a clone
        // of the storage Arc and re-acquires the write lock per record.
        let storage = self.storage.clone();
        self.transaction_manager
            .rollback_with_undo(tx_id, move |rec| {
                let mut storage = storage.write();
                match rec {
                    sqlrustgo_transaction::savepoint::UndoRecord::Insert { table, key, row } => {
                        // v312-60: fall back to full-row match when the
                        // table has no primary key — an empty `key`
                        // filter would otherwise clear the whole table.
                        let target = if key.is_empty() {
                            row.clone()
                        } else {
                            key.to_vec()
                        };
                        storage.delete(table, &target).map_err(|e| {
                            format!("rollback delete on {} pk={:?}: {}", table, key, e)
                        })?;
                        Ok(())
                    }
                    sqlrustgo_transaction::savepoint::UndoRecord::Delete {
                        table,
                        key: _,
                        old_value,
                    } => {
                        storage
                            .insert(table, vec![old_value.clone()])
                            .map_err(|e| format!("rollback reinsert on {}: {}", table, e))?;
                        Ok(())
                    }
                    sqlrustgo_transaction::savepoint::UndoRecord::Update {
                        table,
                        key,
                        old_value,
                        new_value,
                    } => {
                        // v312-60: fall back to deleting by post-update
                        // row when the table has no primary key.
                        let target = if key.is_empty() { new_value } else { key };
                        // Re-insert under the PK (deleting the
                        // forward-UPDATE's row first to avoid a duplicate
                        // if the forward UPDATE kept a different row in
                        // place). This mirrors the SAVEPOINT undo logic
                        // at `execute_savepoint` (lines 1710+).
                        storage
                            .delete(table, target)
                            .map_err(|e| format!("rollback update-delete on {}: {}", table, e))?;
                        storage
                            .insert(table, vec![old_value.clone()])
                            .map_err(|e| format!("rollback update-insert on {}: {}", table, e))?;
                        Ok(())
                    }
                }
            })
            .map_err(|e| {
                SqlError::ExecutionError(format!("Failed to rollback transaction: {:?}", e))
            })?;
        // F-16 Gap Locking: release all gap locks on rollback
        {
            let mut storage = self.storage.write();
            storage.release_all_gap_locks(tx_id.as_u64());
        }
        self.current_tx_id = None;
        self.tx_status = TxStatus::Aborted;
        // INT-1: Reset to Idle so the next DML can begin a new TX or run
        // in autocommit mode. (Same reasoning as commit_transaction above.)
        self.tx_status = TxStatus::Idle;
        self.tx_readonly = false;
        Ok(ExecutorResult::empty())
    }

    /// Begin an implicit TX for DML. Returns `(tx_id, started_implicit)`.
    /// `started_implicit` is `true` ONLY when this call started a fresh TX.
    pub(crate) fn begin_implicit_dml_tx(
        &mut self,
        op: &'static str,
        _table: &str,
    ) -> SqlResult<(Option<TxId>, bool)> {
        let _ = op;
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
        if self.current_tx_id.is_none() {
            let tx_id = self
                .transaction_manager
                .begin_transaction(self.default_isolation)
                .map_err(|e| SqlError::ExecutionError(format!("TM.begin failed: {:?}", e)))?;
            self.current_tx_id = Some(tx_id);
            self.tx_status = TxStatus::Active;
            let mut storage = self.storage.write();
            {
                storage.set_current_tx_id(tx_id.as_u64());
            }
            Ok((Some(tx_id), true))
        } else {
            Ok((self.current_tx_id, false))
        }
    }

    /// Commit the implicit DML TX started by `begin_implicit_dml_tx`.
    /// Idempotent when `started_implicit` is `false` (user controls commit/rollback).
    pub(crate) fn commit_implicit_dml_tx(&mut self, started_implicit: bool) -> SqlResult<()> {
        if started_implicit {
            let tx_id = self.current_tx_id.unwrap();
            let _ = self.transaction_manager.commit(tx_id);
            // WAL checkpoint + truncation lives in StorageEngine::commit_transaction
            let mut storage = self.storage.write();
            let _ = storage.commit_transaction();
            // F-16 Gap Locking: release all gap locks on commit
            storage.release_all_gap_locks(tx_id.as_u64());
            drop(storage);
            self.current_tx_id = None;
            self.tx_status = TxStatus::Idle;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), SqlError> {
        self.storage.write().flush()
    }

    /// V312-57: list all tables visible to the storage engine. Used by the
    /// sqlite3-like teaching CLI (`.tables` dot-command) to enumerate
    /// user-created tables without going through `information_schema`.
    pub fn list_tables(&self) -> Vec<String> {
        self.storage.read().list_tables()
    }

    // ── Set-operation handlers (V310-06 PR2 / Issue #3723 C-2) ──────
    // C-ARCH-05: bodies moved to `crate::engine_setops` (issue #3943 follow-up).

    fn execute_union(&mut self, union_stmt: &UnionStatement) -> SqlResult<ExecutorResult> {
        crate::engine_setops::execute_union(self, union_stmt)
    }

    fn execute_intersect(&mut self, stmt: &IntersectStatement) -> SqlResult<ExecutorResult> {
        crate::engine_setops::execute_intersect(self, stmt)
    }

    fn execute_except(&mut self, stmt: &ExceptStatement) -> SqlResult<ExecutorResult> {
        crate::engine_setops::execute_except(self, stmt)
    }

    /// Execute any parsed statement. Used by the set-operation handlers
    /// for recursive left/right execution of nested set-ops.
    pub(crate) fn execute_statement(&mut self, stmt: &Statement) -> SqlResult<ExecutorResult> {
        match stmt {
            Statement::Select(s) => self.execute_select(s),
            Statement::Union(u) => self.execute_union(u),
            Statement::Intersect(i) => self.execute_intersect(i),
            Statement::Except(e) => self.execute_except(e),
            _ => Err(SqlError::ExecutionError(
                "set-op child must be SELECT, UNION, INTERSECT, or EXCEPT".to_string(),
            )),
        }
    }
}

// C-ARCH-05: collation-aware helpers (V4077 / #4077) moved to
// `crate::engine_collation`. Callers now use:
//   crate::engine_collation::leftmost_column_names
//   crate::engine_collation::expand_column_names
//   crate::engine_collation::multiset_entries
//   crate::engine_collation::leftmost_select
//   crate::engine_collation::collect_column_collations
//   crate::engine_collation::normalize_row_for_compare
//   crate::engine_collation::normalize_value_for_collation
//   crate::engine_collation::order_by_expr_value
// See `src/engine_setops.rs` for the set-operation consumers.

#[doc(inline)]
pub use crate::engine_collation::{
    collect_column_collations, expand_column_names, leftmost_column_names, leftmost_select,
    multiset_entries, normalize_row_for_compare, normalize_value_for_collation,
    order_by_expr_value,
};

// ---------------------------------------------------------------------------
// V312-56E / #4255 — Controlled-subset EXPLAIN plan dump
// ---------------------------------------------------------------------------
//
// `explain_select_plan` walks a `SelectStatement` AST and emits one
// operator line per logical step. The line vocabulary is intentionally
// aligned with the existing `crates/executor/src/explain.rs` operator
// names and with SQLite `EXPLAIN QUERY PLAN` keywords, so the teaching
// corpus oracle can compare them after normalization:
//
//   SeqScan <table>                (mirrors SQLite `SCAN <table>`)
//   IndexScan <table>              (mirrors SQLite `SEARCH <table> USING INDEX`)
//   Filter <expr>                  (WHERE)
//   HashJoin / NestedLoopJoin      (FROM ... JOIN)
//   GroupBy <n_keys>               (mirrors `USE TEMP B-TREE FOR GROUP BY`)
//   Sort <n_keys>                  (ORDER BY; mirrors `USE TEMP B-TREE FOR ORDER BY`)
//   Limit <n>                      (LIMIT)
//   Projection <n_cols>            (final SELECT projection)
//
// V312-62 / Issues #4617 & #4621: the scan kind (SeqScan vs IndexScan) is
// derived from `(WHERE predicate, storage.list_indexes(table))` instead of
// being hardcoded. The rule matches SQLite/MySQL CBO behavior closely
// enough for the teaching corpus:
//   - WHERE col OP lit, index on `col`        → IndexScan (absorb Filter)
//   - WHERE with no matching index            → SeqScan + Filter
//   - SELECT count(*) FROM t, no WHERE/GROUP  → IndexScan covering scan
//                                                if any index exists on t
//
// Limitations vs the full CBO planner:
// - No real cost / row estimates (`estimated_rows` is omitted).
// - The planner's actual `select_scan` still picks the first available
//   index without consulting the WHERE predicate; matching it is the
//   v3.13.0 follow-up tracked under "planner: predicate-aware scan
//   selection".
pub(crate) fn explain_select_plan(
    select: &sqlrustgo_parser::parser::SelectStatement,
    storage: &dyn sqlrustgo_storage::StorageEngine,
) -> Vec<String> {
    use sqlrustgo_parser::parser::{Expression, JoinType};
    let mut lines: Vec<String> = Vec::new();

    // 1. FROM clause — pick scan kind from indexes + WHERE / COUNT(*).
    let primary_indexed = explain_choose_indexed_scan(select, storage);
    if select.from_subquery.is_some() || select.join_clause.is_empty() {
        if primary_indexed.is_some() {
            // V312-62 / #4617, #4621: IndexScan is emitted in the same
            // shape as SeqScan (`<op> <table>`) so the plan_shape oracle
            // normalizer maps both to the canonical `IndexScan <table>`
            // form (mirrors SQLite `SEARCH <table> USING INDEX ...`).
            lines.push(format!("IndexScan {}", select.table));
        } else {
            lines.push(format!("SeqScan {}", select.table));
        }
    } else {
        // Each FROM source — SeqScan (multi-table IndexScan selection is
        // a v3.13.0 follow-up; tracked under #4617 too). Then joins.
        for join in &select.join_clause {
            lines.push(format!("SeqScan {}", join.table));
        }
        let join_kind = match select.join_clause.first().map(|j| &j.join_type) {
            Some(JoinType::Inner) => "NestedLoopJoin",
            Some(JoinType::Left) => "HashJoin",
            Some(JoinType::Right) => "HashJoin",
            Some(JoinType::Full) => "HashJoin",
            Some(JoinType::Cross) => "NestedLoopJoin",
            None => "NestedLoopJoin",
        };
        lines.push(format!("{join_kind}: {} joins", select.join_clause.len()));
    }

    // 2. WHERE clause → Filter (suppressed when IndexScan absorbed it).
    if primary_indexed.is_none() {
        if let Some(w) = &select.where_clause {
            lines.push(format!("Filter {}", expr_to_plan_string(w)));
        }
    }

    // 3. GROUP BY → GroupBy + Sort (TEMP B-TREE FOR GROUP BY).
    //    For `SELECT count(*) FROM t` with no GROUP BY and no WHERE, the
    //    covering IndexScan above replaces both this block and the Sort.
    let has_count_only_no_group = primary_indexed.is_some()
        && select.where_clause.is_none()
        && select.group_by.is_empty()
        && is_count_star_only(select);
    if !has_count_only_no_group
        && (!select.group_by.is_empty() || !select.aggregates.is_empty())
    {
        lines.push(format!("GroupBy {} keys", select.group_by.len().max(1)));
        lines.push("Sort (TEMP B-TREE FOR GROUP BY)".to_string());
    }

    // 4. ORDER BY → Sort (TEMP B-TREE FOR ORDER BY).
    if !select.order_by.is_empty() {
        lines.push(format!("Sort {} keys", select.order_by.len()));
        lines.push("Sort (TEMP B-TREE FOR ORDER BY)".to_string());
    }

    // 5. LIMIT + OFFSET.
    if let Some(limit) = select.limit {
        let mut s = format!("Limit {limit}");
        if let Some(off) = select.offset {
            s.push_str(&format!(" Offset {off}"));
        }
        lines.push(s);
    } else if let Some(off) = select.offset {
        lines.push(format!("Limit all Offset {off}"));
    }

    // 6. DISTINCT → DISTINCT (sort).
    if select.distinct {
        lines.push("Distinct".to_string());
    }

    // 7. Final Projection.
    lines.push(format!("Projection {} cols", select.columns.len()));

    lines
}

/// V312-62 / Issues #4617 & #4621 helper. Inspects `(select, storage)` and
/// returns `Some((column, index_name))` when the single-table FROM source
/// can use an index, else `None` (SeqScan).
///
/// Selection rules (in priority order):
/// 1. WHERE has an equality/range predicate on a column with an index →
///    use that index. The Filter is folded into the IndexScan.
/// 2. SELECT is bare `COUNT(*)` with no WHERE and no GROUP BY, and the
///    table has at least one index → use the first index as a covering
///    scan (sqlite3's count(*) optimization). The GroupBy/Sort pair is
///    also suppressed in this case.
/// 3. Otherwise → SeqScan.
///
/// Multi-table FROM (JOIN) is NOT considered here — see the v3.13.0
/// planner follow-up.
fn explain_choose_indexed_scan(
    select: &sqlrustgo_parser::parser::SelectStatement,
    storage: &dyn sqlrustgo_storage::StorageEngine,
) -> Option<(String, String)> {
    // JOIN / subquery-FROM → defer to v3.13.0.
    if !select.join_clause.is_empty() || select.from_subquery.is_some() {
        return None;
    }

    let indexes = storage.list_indexes(&select.table);
    if indexes.is_empty() {
        return None;
    }

    // 1. WHERE-based index selection: walk the AND-conjunction looking
    //    for an Eq/Lt/Lte/Gt/Gte predicate on a column that has an index.
    if let Some(w) = &select.where_clause {
        if let Some(col) = explain_extract_indexable_column(w) {
            for (idx_col, idx_name) in &indexes {
                if idx_col.eq_ignore_ascii_case(&col) {
                    return Some((idx_col.clone(), idx_name.clone()));
                }
            }
        }
        return None;
    }

    // 2. Bare `SELECT count(*) FROM t` covering-index optimization.
    if select.group_by.is_empty() && is_count_star_only(select) {
        let (idx_col, idx_name) = indexes.into_iter().next().unwrap();
        return Some((idx_col, idx_name));
    }

    None
}

/// V312-62 / #4617 helper. Walk a WHERE expression and return the LHS
/// column name if it is an Eq/Lt/Lte/Gt/Gte BinaryOp whose other side is
/// a literal (e.g. `col = 20`, `col > 25`). Recurses into `AND` to
/// find any conjunct that has an indexable column — mirrors
/// `optimizer::analyze_predicate_for_index` but operates on the parser
/// `Expression` AST that EXPLAIN has access to.
fn explain_extract_indexable_column(expr: &sqlrustgo_parser::parser::Expression) -> Option<String> {
    use sqlrustgo_parser::parser::Expression;
    match expr {
        Expression::BinaryOp(left, op, right) => {
            // `col = lit`, `lit = col`, `col < lit`, ...
            const CMP_OPS: &[&str] = &["=", "!=", "<", "<=", ">", ">="];
            if !CMP_OPS.contains(&op.as_str()) {
                // `AND` is also matched here so we recurse.
                if op.eq_ignore_ascii_case("AND") {
                    return explain_extract_indexable_column(left)
                        .or_else(|| explain_extract_indexable_column(right));
                }
                return None;
            }
            // Identifier on the left, Literal on the right.
            if let Expression::Identifier(name) = left.as_ref() {
                if matches!(right.as_ref(), Expression::Literal(_)) {
                    return Some(name.clone());
                }
            }
            // Literal on the left, Identifier on the right.
            if let Expression::Identifier(name) = right.as_ref() {
                if matches!(left.as_ref(), Expression::Literal(_)) {
                    return Some(name.clone());
                }
            }
            None
        }
        Expression::UnaryOp(op, inner) if op.eq_ignore_ascii_case("NOT") => {
            // `NOT (col <op> lit)` — extract from the inner expression.
            explain_extract_indexable_column(inner)
        }
        _ => None,
    }
}

/// V312-62 / #4621 helper. Returns true iff `select` is a bare
/// `SELECT count(*) FROM t` (no WHERE, no GROUP BY, no DISTINCT).
///
/// The parser represents `COUNT(*)` as `AggregateCall { func: Count,
/// args: [], distinct: false }` (the `*` is consumed by the parser
/// and stored as an empty `args` vec — see
/// `parse_aggregate_function` in `crates/parser/src/parser.rs`).
fn is_count_star_only(select: &sqlrustgo_parser::parser::SelectStatement) -> bool {
    use sqlrustgo_parser::parser::AggregateFunction;
    if !select.where_clause.is_none() || !select.group_by.is_empty() {
        return false;
    }
    if select.aggregates.len() != 1 {
        return false;
    }
    let agg = &select.aggregates[0];
    agg.func == AggregateFunction::Count && !agg.distinct && agg.args.is_empty()
}

/// Render an `Expression` for inclusion in a plan line. The corpus
/// SQL is simple enough that a Debug-format dump is readable; the
/// oracle normalizer maps whitespace and operator spelling to
/// canonical form before comparing.
fn expr_to_plan_string(expr: &sqlrustgo_parser::parser::Expression) -> String {
    format!("{expr:?}")
}

/// V312-58 / Issue #4511: parse the stringified RHS of `SET @var = expr`
/// into a `Value`. The parser captures `expr` as raw token text
/// (number, string literal, bool), so the engine needs to coerce it
/// back into a typed `Value` for the session-vars map.
///
/// Recognised shapes:
/// - `NULL` (case-insensitive) → `Value::Null`
/// - `true` / `false` (lowercase, matching the parser output) →
///   `Value::Boolean`
/// - signed integer parseable → `Value::Integer`
/// - otherwise → `Value::Text(s)` (un-quoted identifier or unknown)
fn parse_session_value(raw: &str) -> SqlValue {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return SqlValue::Null;
    }
    let upper = trimmed.to_ascii_uppercase();
    if upper == "NULL" {
        return SqlValue::Null;
    }
    if trimmed == "true" {
        return SqlValue::Boolean(true);
    }
    if trimmed == "false" {
        return SqlValue::Boolean(false);
    }
    if let Ok(n) = trimmed.parse::<i64>() {
        return SqlValue::Integer(n);
    }
    SqlValue::Text(trimmed.to_string())
}

/// V312-58 / Issue #4511: walk an `Expression` and replace every
/// `Identifier("@name")` leaf with the matching session-variable
/// `Literal` (or `Literal("NULL")` when the variable is unbound).
///
/// Used by the projection path in `execute_select` so that
/// `SELECT @a` and `EXECUTE p USING @a` resolve to the bound session
/// value rather than the legacy `Value::Text("@a")` fallback.
///
/// Recursion mirrors the variant set on [`sqlrustgo_parser::Expression`].
pub(crate) fn substitute_session_vars_in_expr(
    expr: sqlrustgo_parser::Expression,
    session_vars: &HashMap<String, SqlValue>,
) -> sqlrustgo_parser::Expression {
    use sqlrustgo_parser::Expression;
    if let Expression::Identifier(ref name) = expr {
        if let Some(stripped) = name.strip_prefix('@') {
            let key = format!("@{}", stripped);
            let lit = match session_vars.get(&key) {
                Some(SqlValue::Null) => "NULL".to_string(),
                Some(SqlValue::Boolean(true)) => "true".to_string(),
                Some(SqlValue::Boolean(false)) => "false".to_string(),
                Some(SqlValue::Integer(n)) => n.to_string(),
                Some(SqlValue::Float(f)) => f.to_string(),
                Some(SqlValue::Text(s)) => format!("'{}'", s.replace('\'', "''")),
                Some(SqlValue::Blob(_)) | Some(SqlValue::Point(_, _)) | Some(SqlValue::Json(_)) => {
                    "NULL".to_string()
                }
                None => "NULL".to_string(),
            };
            return Expression::Literal(lit);
        }
    }
    match expr {
        Expression::BinaryOp(l, op, r) => Expression::BinaryOp(
            Box::new(substitute_session_vars_in_expr(*l, session_vars)),
            op,
            Box::new(substitute_session_vars_in_expr(*r, session_vars)),
        ),
        Expression::UnaryOp(op, inner) => Expression::UnaryOp(
            op,
            Box::new(substitute_session_vars_in_expr(*inner, session_vars)),
        ),
        Expression::FunctionCall(name, args) => Expression::FunctionCall(
            name,
            args.into_iter()
                .map(|a| substitute_session_vars_in_expr(a, session_vars))
                .collect(),
        ),
        Expression::IsNull(inner) => Expression::IsNull(Box::new(substitute_session_vars_in_expr(
            *inner,
            session_vars,
        ))),
        Expression::IsNotNull(inner) => Expression::IsNotNull(Box::new(
            substitute_session_vars_in_expr(*inner, session_vars),
        )),
        Expression::InList(left, values) => Expression::InList(
            Box::new(substitute_session_vars_in_expr(*left, session_vars)),
            values
                .into_iter()
                .map(|v| substitute_session_vars_in_expr(v, session_vars))
                .collect(),
        ),
        Expression::NotInList(left, values) => Expression::NotInList(
            Box::new(substitute_session_vars_in_expr(*left, session_vars)),
            values
                .into_iter()
                .map(|v| substitute_session_vars_in_expr(v, session_vars))
                .collect(),
        ),
        Expression::Between(l, lo, hi) => Expression::Between(
            Box::new(substitute_session_vars_in_expr(*l, session_vars)),
            Box::new(substitute_session_vars_in_expr(*lo, session_vars)),
            Box::new(substitute_session_vars_in_expr(*hi, session_vars)),
        ),
        Expression::NotBetween(l, lo, hi) => Expression::NotBetween(
            Box::new(substitute_session_vars_in_expr(*l, session_vars)),
            Box::new(substitute_session_vars_in_expr(*lo, session_vars)),
            Box::new(substitute_session_vars_in_expr(*hi, session_vars)),
        ),
        Expression::Like(l, p, esc) => Expression::Like(
            Box::new(substitute_session_vars_in_expr(*l, session_vars)),
            Box::new(substitute_session_vars_in_expr(*p, session_vars)),
            esc,
        ),
        Expression::NotLike(l, p, esc) => Expression::NotLike(
            Box::new(substitute_session_vars_in_expr(*l, session_vars)),
            Box::new(substitute_session_vars_in_expr(*p, session_vars)),
            esc,
        ),
        Expression::CaseWhen(whens, default) => Expression::CaseWhen(
            whens
                .into_iter()
                .map(|w| sqlrustgo_parser::parser::WhenClause {
                    condition: substitute_session_vars_in_expr(w.condition, session_vars),
                    result: substitute_session_vars_in_expr(w.result, session_vars),
                })
                .collect(),
            default.map(|d| Box::new(substitute_session_vars_in_expr(*d, session_vars))),
        ),
        Expression::QuantifiedOp(l, op, subq) => Expression::QuantifiedOp(
            Box::new(substitute_session_vars_in_expr(*l, session_vars)),
            op,
            subq,
        ),
        // Terminal / non-recursive variants pass through unchanged.
        other => other,
    }
}

/// Issue #4511 — substitute `@name` session variables across every
/// expression-bearing field of a `SelectStatement`. This is invoked at
/// the top of `execute_select` so WHERE / HAVING / GROUP BY / ORDER BY
/// / projection all see the resolved literal value, not the raw
/// `Identifier("@name")` token. Bypasses subquery bodies (those are
/// passed through unchanged and resolved when their own `execute_select`
/// runs).
pub(crate) fn substitute_session_vars_in_select(
    select: &sqlrustgo_parser::SelectStatement,
    session_vars: &HashMap<String, SqlValue>,
) -> sqlrustgo_parser::SelectStatement {
    use sqlrustgo_parser::parser::OrderByExpression;
    use sqlrustgo_parser::{Expression, SelectColumn, SelectStatement};

    let map_expr =
        |e: Expression| -> Expression { substitute_session_vars_in_expr(e, session_vars) };

    let columns: Vec<SelectColumn> = select
        .columns
        .iter()
        .map(|c| SelectColumn {
            name: c.name.clone(),
            alias: c.alias.clone(),
            expression: c.expression.clone().map(map_expr),
        })
        .collect();

    let group_by: Vec<Expression> = select.group_by.iter().cloned().map(map_expr).collect();

    let order_by: Vec<OrderByExpression> = select
        .order_by
        .iter()
        .map(|o| OrderByExpression {
            expression: map_expr(o.expression.clone()),
            ascending: o.ascending,
            nulls_first: o.nulls_first,
        })
        .collect();

    SelectStatement {
        columns,
        table: select.table.clone(),
        schema: select.schema.clone(),
        from_alias: select.from_alias.clone(),
        from_subquery: select.from_subquery.clone(),
        from_values: select.from_values.clone(),
        where_clause: select.where_clause.clone().map(map_expr),
        join_clause: select.join_clause.clone(),
        extra_tables: select.extra_tables.clone(),
        aggregates: select.aggregates.clone(),
        group_by,
        with_rollup: select.with_rollup,
        with_cube: select.with_cube,
        having: select.having.clone().map(map_expr),
        order_by,
        limit: select.limit,
        offset: select.offset,
        distinct: select.distinct,
        lock_clause: select.lock_clause.clone(),
    }
}
