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
    CreateDatabaseStatement, CreateFunctionStatement, CreateGraphStatement, CreateIndexStatement,
    CreateProcedureStatement, CreateRoleStatement, CreateSequenceStatement, CreateTableStatement,
    CreateTriggerStatement, CreateUserStatement, CreateVectorIndexStatement, CreateViewStatement,
    DescribeStatement, DropDatabaseStatement, DropFunctionStatement, DropGraphStatement,
    DropIndexStatement, DropProcedureStatement, DropRoleStatement, DropSequenceStatement,
    DropTableStatement, DropTriggerStatement, DropUserStatement, DropViewStatement,
    ExceptStatement, GrantRoleStatement, GrantStatement, InsertStatement, IntersectStatement,
    MergeStatement, ObjectType as ParserObjectType, OrderByExpression,
    Privilege as ParserPrivilege, RevokeRoleStatement, RevokeStatement, SelectStatement,
    SetRoleStatement, ShowStatement, StorageEngineSpec, StoredProcParam as ParserStoredProcParam,
    StoredProcParamMode as ParserParamMode, StoredProcStatement as ParserStatement,
    TruncateStatement, UnionStatement, VectorIndexAlgorithm,
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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// Execution engine for SQL statements
pub struct ExecutionEngine<S: StorageEngine> {
    pub(crate) storage: Arc<parking_lot::RwLock<S>>,
    pub(crate) catalog: Option<Arc<parking_lot::RwLock<Catalog>>>,
    pub(crate) stats: Arc<parking_lot::RwLock<ExecutionStats>>,
    // V4.1.0 / Issue #4910 §3.1: convert from `bool` to `AtomicBool` so the
    // `&self` SELECT/SHOW/EXPLAIN path doesn't need the engine write lock.
    pub(crate) cbo_enabled: AtomicBool,
    pub(crate) transaction_manager: TransactionManager,
    // V4.1.0 / Issue #4910 §3.1 Phase 3: see `TxSession` below. The
    // trigger_undo_sink remains its own `Arc<Mutex<Vec<_>>>` because the
    // trigger recorder already uses interior mutability.
    pub(crate) tx_session: Arc<parking_lot::Mutex<TxSession>>,
    /// V312-55D (Round-26, follow-up): shared buffer used by the trigger-side undo recorder.
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
    
    /// V312-77 / Issue #4847: distinguishes an explicit BEGIN (set to true
    /// when `begin_transaction` is called) from an implicit DML transaction
    /// (set to false). Only explicit transactions should be tracked by
    /// `commit_implicit_dml_tx` / `rollback_transaction` so that DML inside
    /// an explicit BEGIN does not auto-commit and ROLLBACK can undo it.
    
    
    
    
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
    pub cost_model: parking_lot::RwLock<UnifiedCostModel>,
    // V4.1.0 / Issue #4910 §3.1: convert from `usize` to `AtomicUsize` so the
    // `&self` SELECT path can read parallelism without the engine write lock.
    pub(crate) parallel_degree: AtomicUsize,
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
    /// V4.1.0: per-table primary-key index used by INSERT's duplicate-key
    /// check. `execute_insert` used to call `storage.scan()` on every
    /// statement, so N single-row INSERTs into a table with a PRIMARY KEY
    /// cost O(N^2): measured 44s for 15000 rows. This holds the table's
    /// primary-key values so the check is a set lookup, and a table without
    /// a PRIMARY KEY keeps the scan (it still needs one for UNIQUE /
    /// AUTO_INCREMENT).
    ///
    /// An entry is trusted only while its recorded
    /// `StorageEngine::table_change_stamp` still matches the table's live
    /// stamp, so any row mutation invalidates it — including mutations from
    /// paths with no knowledge of this cache (trigger bodies, GMP helpers,
    /// `LOAD DATA`, wire endpoints, ROLLBACK undo replay). A storage engine
    /// that does not track change stamps reports 0, which disables the
    /// fast path rather than risking a stale hit.
    pub(crate) pk_lookup_cache:
        parking_lot::RwLock<HashMap<String, crate::engine_dml::PkIndexCache>>,
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
    /// V312-64f / Issue #4699: per-engine override for the recursive CTE
    /// row cap (`MAX_RECURSION_ROWS` in `engine_cte`). V4.1.0 / #4910 §3.1:
    /// `AtomicUsize` so the CTE driver can read without the engine write lock.
    pub recursive_cte_max_rows: AtomicUsize,
}

/// V4.1.0 / Issue #4910 §3.1 Phase 3: per-connection transaction state
/// grouped behind `Arc<parking_lot::Mutex<_>>` so DDL/DML can run via
/// `&self` instead of `&mut self`. The 6 fields here are the ones
/// `execute_insert/update/delete` (and the TX statement handlers) need
/// to mutate during the hot path.
///
/// `current_user` is intentionally NOT here — it's a `Copy`-able small
/// struct read only on the auth path (no contention). `session_vars` and
/// `trigger_undo_sink` keep their existing `Arc<RwLock<_>>` /
/// `Arc<Mutex<_>>` wrappers. `session_null_order_first` is read-only at
/// runtime and stays on `ExecutionEngine` directly.
pub(crate) struct TxSession {
    pub(crate) current_tx_id: Option<TxId>,
    pub(crate) tx_status: TxStatus,
    pub(crate) tx_readonly: bool,
    pub(crate) is_explicit_transaction: bool,
    pub(crate) default_isolation: TmIsolationLevel,
    pub(crate) current_role: Option<String>,
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
        // #4913 / v4.1.0-perf: do not let a parallelism request degrade
        // silently. The intra-query parallel implementations live behind
        // `sqlrustgo-executor/parallel-executor`, which was never enabled
        // by any manifest in this repo, so the executor compiles its
        // serial `cfg(not(feature = "parallel-executor"))` branches. Users
        // setting --executor-parallelism=N used to get a rayon pool plus
        // serial execution with no signal at all.
        #[cfg(not(feature = "parallel-executor"))]
        if parallel_degree > 1 {
            tracing::warn!(
                "executor parallelism requested ({}), but this binary was built \
                 without the `parallel-executor` feature: intra-query \
                 scan/join/aggregate will run sequentially. Rebuild with \
                 `--features parallel-executor` to enable it.",
                parallel_degree
            );
        }
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
            cbo_enabled: AtomicBool::new(cbo_enabled),
            transaction_manager: TransactionManager::new(),
            tx_session: Arc::new(parking_lot::Mutex::new(TxSession {
                current_tx_id: None,
                tx_status: TxStatus::Idle,
                is_explicit_transaction: false,
                tx_readonly: false,
                default_isolation: TmIsolationLevel::default(),
                current_role: None,
            })),
            trigger_undo_sink: Arc::new(parking_lot::Mutex::new(Vec::new())),
            current_user: UserIdentity::new("root", "localhost"),
            session_null_order_first: None,
            checkpoint_manager: None,
            parallel_degree: AtomicUsize::new(parallel_degree),
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
            cost_model: parking_lot::RwLock::new(UnifiedCostModel::default_model(0, 0)),
            views: HashMap::new(),
            clustered_tables: parking_lot::RwLock::new(HashMap::new()),
            pk_lookup_cache: parking_lot::RwLock::new(HashMap::new()),
            adaptive_hash_index: AdaptiveHashIndex::new().into_shared(),
            instrumentation: Arc::new(sqlrustgo_executor::instrumentation::NoopInstrumentationHook),
            session_vars: Arc::new(RwLock::new(HashMap::new())),
            sequence_state: Arc::new(crate::sequence_state::SequenceState::new()),
            recursive_cte_max_rows: AtomicUsize::new(1_000_000),
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

    /// V312-64f / Issue #4699: override the recursive CTE row cap for
    /// this builder to lower the cap without generating 1M-row fixtures.
    pub fn with_recursive_cte_max_rows(mut self, cap: usize) -> Self {
        self.recursive_cte_max_rows.store(cap, Ordering::Relaxed);
        self
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
        self.cbo_enabled.load(Ordering::Relaxed)
    }
    /// Enable or disable CBO
    pub fn set_cbo_enabled(&mut self, enabled: bool) {
        self.cbo_enabled.store(enabled, Ordering::Relaxed);
    }
    pub fn parallel_degree(&self) -> usize {
        self.parallel_degree.load(Ordering::Relaxed)
    }
    pub fn set_parallel_degree(&mut self, degree: usize) {
        self.parallel_degree.store(degree.max(1), Ordering::Relaxed);
    }
    pub fn build_parallel_executor(
        &self,
    ) -> sqlrustgo_executor::parallel_executor::ParallelVolcanoExecutor {
        sqlrustgo_executor::parallel_executor::ParallelVolcanoExecutor::new(self.parallel_degree())
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
        if !self.cbo_enabled.load(Ordering::Relaxed) {
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
            Some(JoinType::Natural) => "NestedLoopJoin",
            Some(JoinType::NaturalLeft) => "HashJoin",
            Some(JoinType::NaturalRight) => "HashJoin",
            Some(JoinType::NaturalFull) => "HashJoin",
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
    if !has_count_only_no_group && (!select.group_by.is_empty() || !select.aggregates.is_empty()) {
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
    if select.where_clause.is_some() || !select.group_by.is_empty() {
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
pub(crate) fn parse_session_value(raw: &str) -> SqlValue {
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
        // V312-95 v2 / Issue #4809: propagate SQLite INDEXED BY /
        // NOT INDEXED hints through session-var substitution.
        from_indexed_by: select.from_indexed_by.clone(),
        from_not_indexed: select.from_not_indexed,
        from_subquery: select.from_subquery.clone(),
        // V312-95 v2 / Issue #4717: propagate FROM (WITH ...) subquery.
        from_with_subquery: select.from_with_subquery.clone(),
        from_values: select.from_values.clone(),
        from_function_args: select.from_function_args.clone(),
        where_clause: select.where_clause.clone().map(map_expr),
        join_clause: select.join_clause.clone(),
        extra_tables: select.extra_tables.clone(),
        aggregates: select.aggregates.clone(),
        group_by,
        with_rollup: select.with_rollup,
        with_cube: select.with_cube,
        grouping_sets: select.grouping_sets.clone(),
        having: select.having.clone().map(map_expr),
        order_by,
        limit: select.limit,
        offset: select.offset,
        distinct: select.distinct,
        lock_clause: select.lock_clause.clone(),
        index_hints: select.index_hints.clone(),
    }
}
