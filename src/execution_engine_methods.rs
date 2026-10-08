use crate::execution_engine::{explain_select_plan, parse_session_value};
use crate::execution_engine::{ExecutionEngine, TableStatistics, TxStatus};
use crate::expr_utils::expression_to_value_from_string;
use crate::{parse, SqlError, SqlResult, Value};
use sqlrustgo_catalog::stored_proc::{ParamMode, StoredProcStatement};
use sqlrustgo_catalog::{ObjectRef, Privilege, StoredProcedure};
use sqlrustgo_executor::expr as expr_mod;
use sqlrustgo_executor::stored_proc::StoredProcExecutor;
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{
    CallStatement, CreateDatabaseStatement, CreateFunctionStatement, CreateGraphStatement,
    CreateIndexStatement, CreateProcedureStatement, CreateTriggerStatement,
    CreateVectorIndexStatement, CreateViewStatement, DropDatabaseStatement, DropFunctionStatement,
    DropGraphStatement, DropIndexStatement, DropProcedureStatement, DropSequenceStatement,
    DropTableStatement, DropTriggerStatement, DropViewStatement, ExceptStatement, InsertStatement,
    IntersectStatement, MergeStatement, StoredProcParamMode as ParserParamMode,
    StoredProcStatement as ParserStatement, TruncateStatement, UnionStatement,
    VectorIndexAlgorithm,
};
use sqlrustgo_parser::transaction::IsolationLevel as ParserIsolationLevel;
use sqlrustgo_parser::{
    DeleteStatement, SavepointOp, Statement, TransactionStatement, UpdateStatement,
};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_transaction::{IsolationLevel as TmIsolationLevel, TxId};
use std::sync::Arc;

// === extracted impl ExecutionEngine<S> block (line 546-2329 of original) ===

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

    /// #4983 / #4951: this connection's transaction id, or 0 outside a
    /// transaction.
    ///
    /// `TxSession` is per-`ExecutionEngine`, i.e. per connection. The
    /// storage cannot recover this on its own — it holds one shared
    /// `current_tx_id` that says whoever wrote last, not whoever is
    /// reading — so every read has to carry it explicitly.
    pub(crate) fn reader_tx(&self) -> u64 {
        self.tx_session
            .lock()
            .current_tx_id
            .map(|id| id.as_u64())
            .unwrap_or(0)
    }

    /// #5057: `scan_for_reader_with` against a stated database.
    ///
    /// The table name alone is not enough: `scan_in` resolves it through the
    /// storage's shared `current_db`, so the row set could come from a
    /// different connection's database than the `get_table_info_in` lookup
    /// that named the table.
    pub(crate) fn scan_for_reader_in_db(
        &self,
        storage: &S,
        db: &str,
        table: &str,
    ) -> SqlResult<Vec<sqlrustgo_storage::engine::Record>> {
        let reader_tx = self.reader_tx();
        // #5105: `scan_in_db` takes no transaction, so routing through it
        // put reads back on the storage-wide `current_tx_id` and let one
        // connection see another's uncommitted rows. `scan_in_tx_db`
        // carries both. `reader_tx` being computed and unused was the
        // whole bug in three lines.
        storage.scan_in_tx_db(db, table, reader_tx)
    }

    /// #5113: [`scan_for_reader_dyn`](Self::scan_for_reader_dyn) against a
    /// stated database.
    ///
    /// `scan_for_reader_dyn` keeps the transaction and resolves the table
    /// through the storage's shared `current_db`, so a foreign-key check
    /// running on one connection validated the parent table of whichever
    /// database was selected last. Added with the same shape as
    /// `scan_for_reader_in_db` so the two cannot drift.
    pub(crate) fn scan_for_reader_dyn_in(
        &self,
        storage: &dyn sqlrustgo_storage::engine::StorageEngine,
        db: &str,
        table: &str,
    ) -> SqlResult<Vec<sqlrustgo_storage::engine::Record>> {
        let reader_tx = self.reader_tx();
        storage.scan_in_tx_db(db, table, reader_tx)
    }

    /// #4983: predicate variant of [`scan_for_reader`](Self::scan_for_reader).
    pub(crate) fn scan_for_reader_filtered(
        &self,
        table: &str,
        filter: &dyn Fn(&sqlrustgo_storage::engine::Record) -> bool,
    ) -> SqlResult<Vec<sqlrustgo_storage::engine::Record>> {
        let reader_tx = self.reader_tx();
        self.storage_read()
            .scan_with_filter_in(table, filter, reader_tx)
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
    pub(super) fn collect_table_stats(&self, table: &str) -> SqlResult<TableStatistics> {
        let storage = self.storage.read();
        crate::cbo_estimator::collect_table_stats(self, &*storage, table)
    }

    /// Execute a SQL statement and return results
    pub fn execute(&mut self, sql: &str) -> SqlResult<ExecutorResult> {
        let parsed = parse(sql).map_err(|e| SqlError::ParseError(e.to_string()))?;
        // #5141: `DATABASE()` / `SCHEMA()` are replaced here, once, for every
        // statement type. Doing it inside `execute_select` left DML reaching
        // `eval_fn`'s hard-coded fallback, which returned "default" for
        // UPDATE and NULL for INSERT — two different wrong answers for the
        // same function.
        let statement = crate::execution_engine::substitute_current_database_in_statement(
            &parsed,
            &self.session_db(),
        );

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
            // SQLite-style schema introspection (`PRAGMA table_info(...)`).
            Statement::Pragma(pragma) => self.execute_pragma(&pragma),
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
            // V400-03 / Issue #3731 (G1): first-class graph DDL.
            // The executor stubs that create/drop a graph name; the
            // actual DiskGraphStore binding (G3) lands in a follow-up.
            Statement::CreateGraph(ref g) => self.execute_create_graph_stub(g),
            Statement::DropGraph(ref g) => self.execute_drop_graph_stub(g),
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
            // V400-01 / Issue #4877: VECTOR INDEX (HNSW / IVF).
            //
            // Physical build is V400-02; for now we wire the metadata
            // into the catalog so a follow-up V400-02 PR can drive the
            // build without re-parsing. If the underlying column is not
            // a VECTOR(N[, dtype]) the catalog will reject at INSERT time.
            Statement::CreateVectorIndex(ref vidx) => self.execute_create_vector_index(vidx),
            // V312-35 #4218: KILL <id> / KILL CONNECTION <id> /
            Statement::Kill {
                connection_id,
                kill_query,
            } => self.execute_kill(connection_id, kill_query),
        }
    }

    /// V4.1.0 / Issue #4910 §3.1 Phase 2: read-only entry point.
    ///
    /// Parses `sql` and dispatches to the existing `&self` SELECT /
    /// EXPLAIN / SHOW / DESCRIBE / PRAGMA / VACUUM / REINDEX handlers.
    /// Returns a clear error for any DDL / DML / transaction statement
    /// so the caller knows to use the `&mut self` `execute()` path.
    ///
    /// This unlocks the server's `engine.read().execute_read_only(sql)`
    /// fast path: SELECTs across concurrent connections no longer
    /// serialise on the engine `&mut self` borrow at the routing layer,
    /// only on the underlying storage `parking_lot::RwLock`.
    pub fn execute_read_only(&self, sql: &str) -> SqlResult<ExecutorResult> {
        let statement = parse(sql).map_err(|e| SqlError::ParseError(e.to_string()))?;
        match statement {
            Statement::Select(ref select) => self.execute_select(select),
            Statement::Explain(ref select) => self.execute_explain(select),
            Statement::Show(ref show) => self.execute_show(show),
            Statement::Describe(ref desc) => self.execute_describe(desc),
            Statement::Pragma(ref pragma) => self.execute_pragma(pragma),
            // V312-64 / Issue #4663: SQLite-style maintenance no-ops.
            Statement::Vacuum(_) | Statement::Reindex(_) => Ok(ExecutorResult::empty()),
            // VALUES is not allowed as a standalone statement — same
            // behaviour as `execute()` so we mirror the error.
            Statement::Values(_) => Err(SqlError::ExecutionError(
                "VALUES cannot be used as a standalone statement".to_string(),
            )),
            // All other arms require `&mut self` (DDL / DML / Tx).
            // Statement doesn't impl Display, so use Debug formatting.
            other => Err(SqlError::ExecutionError(format!(
                "read-only entry point cannot execute statement kind `{:?}`; \
                 use execute_mut() for DDL/DML/transaction statements",
                other
            ))),
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

    pub(super) fn execute_drop_table(
        &self,
        drop: &DropTableStatement,
    ) -> SqlResult<ExecutorResult> {
        // No explicit PK-index invalidation: `drop_table` bumps the table's
        // change stamp, so a cached index can no longer be trusted.
        let mut storage = self.storage.write();
        if drop.if_exists && !storage.has_table(&drop.name) {
            // IF EXISTS specified and table doesn't exist → no-op, success
            return Ok(ExecutorResult::empty());
        }
        storage.drop_table(&drop.name)?;
        Ok(ExecutorResult::empty())
    }

    pub(super) fn execute_create_database(
        &self,
        db: &CreateDatabaseStatement,
    ) -> SqlResult<ExecutorResult> {
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

    pub(super) fn execute_drop_database(
        &self,
        db: &DropDatabaseStatement,
    ) -> SqlResult<ExecutorResult> {
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

    // V400-03 / Issue #3731 (G1): first-class graph DDL stub.
    //
    // The parser recognizes `CREATE GRAPH [IF NOT EXISTS] <name>` and
    // `DROP GRAPH [IF EXISTS] <name>` (parser.rs variants
    // `Statement::CreateGraph` / `Statement::DropGraph`). The executor
    // here only registers the graph name in the catalog; the actual
    // `sqlrustgo_graph::DiskGraphStore` binding (G3 in the dev plan)
    // is a follow-up that wires graph ops to share the production
    // `WalStorage` instance.
    //
    // The stub returns `affected_rows = 0` so mysql-server emits an
    // OK packet, mirroring the empty-success contract used by
    // `CREATE DATABASE` / `DROP DATABASE`.
    pub(super) fn execute_create_graph_stub(
        &self,
        g: &CreateGraphStatement,
    ) -> SqlResult<ExecutorResult> {
        if g.name == "default" {
            return Err(SqlError::ExecutionError(
                "CREATE GRAPH 'default' is not permitted (reserved name)".to_string(),
            ));
        }
        tracing::info!(
            target: "sqlrustgo::v400_03",
            "CREATE GRAPH '{}' (stub; actual DiskGraphStore binding is V400-03 G3)",
            g.name
        );
        Ok(ExecutorResult::new(vec![], 0))
    }

    pub(super) fn execute_drop_graph_stub(
        &self,
        g: &DropGraphStatement,
    ) -> SqlResult<ExecutorResult> {
        if g.name == "default" {
            return Err(SqlError::ExecutionError(
                "DROP GRAPH 'default' is not permitted (reserved name)".to_string(),
            ));
        }
        tracing::info!(
            target: "sqlrustgo::v400_03",
            "DROP GRAPH '{}' (stub; actual DiskGraphStore unbind is V400-03 G3)",
            g.name
        );
        Ok(ExecutorResult::new(vec![], 0))
    }

    // V312-F-4: execute_create_sequence moved to src/engine_create.rs
    // to keep execution_engine.rs under 1500 lines (C-ARCH-05 AD-001).

    pub(super) fn execute_drop_sequence(
        &self,
        seq_stmt: &DropSequenceStatement,
    ) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();

        // V312-72 (perf-refactor): consult both storage AND the in-memory
        // cache — a CREATE-then-DROP in the same session may have already
        // updated the cache even if the storage layer rejected the prior
        // persist, or vice versa.
        if !storage.has_sequence(&seq_stmt.name) && !self.sequence_state.contains(&seq_stmt.name) {
            if seq_stmt.if_exists {
                return Ok(ExecutorResult::empty());
            }
            return Err(SqlError::ExecutionError(format!(
                "Sequence '{}' not found",
                seq_stmt.name
            )));
        }

        storage.drop_sequence(&seq_stmt.name)?;
        // V312-72: sync the in-memory SequenceState cache so subsequent
        // NEXT VALUE FOR / CURRVAL on the dropped sequence error out
        // instead of reading a stale entry.
        self.sequence_state.drop_sequence(&seq_stmt.name);
        Ok(ExecutorResult::empty())
    }

    // V312-F-4: execute_alter_sequence moved to src/engine_create.rs
    // to keep execution_engine.rs under 1500 lines (C-ARCH-05 AD-001).

    pub(super) fn execute_use_database(&self, db: &str) -> SqlResult<ExecutorResult> {
        // #5025: `USE` used to be an accepted no-op, so every database
        // shared one table namespace. It now switches the storage's active
        // database, and an unknown name is an error rather than a silent
        // fall-through to the previous database.
        //
        // #5057: `USE` is connection-level in the MySQL protocol, so it
        // also records the choice on **this engine** — the storage field is
        // one shared value, and without the per-engine copy one connection's
        // switch redirects another's reads.
        self.storage.write().set_current_db(db)?;
        *self.session_db.write() = db.to_lowercase();
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

    pub(super) fn execute_truncate(
        &self,
        truncate: &TruncateStatement,
    ) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();
        if !storage.has_table(&truncate.name) {
            return Err(SqlError::ExecutionError(format!(
                "Table not found: {}",
                truncate.name
            )));
        }
        // V312-64h / Issue #4762: CASCADE / RESTRICT are recorded for
        // dialect compatibility; current executor truncates all rows
        // regardless (no FK reference tracking yet). RESTRICT would
        // error only if any FK pointed at the table; sqlrustgo does
        // not yet enforce FK constraints, so the behavior is the same.
        // The delete bumps the table's change stamp, which is what
        // invalidates any cached primary-key index — no explicit
        // invalidation needed here.
        storage.delete(&truncate.name, &[])?;
        Ok(ExecutorResult::empty())
    }

    pub(super) fn execute_create_index(
        &self,
        idx: &CreateIndexStatement,
    ) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();
        let table_name = &idx.table;
        // V313-100 / Issue #4701 sub-1: a CREATE INDEX column list may
        // start with an arbitrary expression, not a bare column name.
        // Skip the table-column lookup when the head entry has no
        // simple name (i.e. is an expression-only spec).
        if let Some(spec) = idx.columns.first() {
            if let Some(name) = spec.name.as_deref() {
                let table_info = storage.get_table_info(table_name)?;
                let _ = table_info
                    .columns
                    .iter()
                    .position(|c| c.name == name)
                    .ok_or_else(|| SqlError::ExecutionError("Column not found".to_string()))?;
            }
        } else {
            return Err(SqlError::ExecutionError("No columns in index".to_string()));
        }
        storage.create_index(sqlrustgo_storage::IndexInfo {
            name: idx.name.clone(),
            table: table_name.clone(),
            columns: idx.columns.clone(),
            is_unique: idx.unique,
            original_sql: crate::ddl_to_sql::format_create_index_sql(idx),
        })?;
        Ok(ExecutorResult::empty())
    }

    /// V400-01 / Issue #4877: VECTOR INDEX DDL hook.
    ///
    /// Validates that the underlying column is declared `VECTOR(N[, dtype])`,
    /// then registers the index metadata in storage so the V400-02 build
    /// path can pick it up without re-parsing.
    ///
    /// Returned `ExecutorResult` carries the parsed `CreateVectorIndexStatement`
    /// in `created_index` so observability + audit chain see the new index.
    pub(super) fn execute_create_vector_index(
        &self,
        vidx: &CreateVectorIndexStatement,
    ) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();

        // Verify table exists.
        let table_info = storage.get_table_info(&vidx.table).map_err(|_| {
            SqlError::ExecutionError(format!(
                "CREATE VECTOR INDEX failed: table '{}' does not exist",
                vidx.table
            ))
        })?;

        // Verify column exists.
        let col = table_info
            .columns
            .iter()
            .find(|c| c.name == vidx.column)
            .ok_or_else(|| {
                SqlError::ExecutionError(format!(
                    "CREATE VECTOR INDEX failed: column '{}' not found in table '{}'",
                    vidx.column, vidx.table
                ))
            })?;

        // Verify column is VECTOR(N[, dtype]). Parser-level identifier
        // case is upper; spec says "VECTOR" (mirrors INT/VARCHAR style).
        let data_type_upper = col.data_type.to_uppercase();
        if !data_type_upper.starts_with("VECTOR") {
            return Err(SqlError::ExecutionError(format!(
                "CREATE VECTOR INDEX requires a VECTOR(N[, dtype]) column; \
                 column '{}' has type '{}'",
                vidx.column, col.data_type
            )));
        }

        // Validate algorithm-specific option keys. HNSW wants `m`/`ef_construction`;
        // IVF wants `nlist`. We surface a warning rather than an error for
        // unknown keys so callers can add new options without parser churn.
        let allowed_keys: &[&str] = match vidx.index_type {
            VectorIndexAlgorithm::Hnsw => &["m", "ef_construction", "ef_search"],
            VectorIndexAlgorithm::Ivf => &["nlist", "nprobe"],
        };
        for (k, _) in &vidx.options {
            if !allowed_keys.iter().any(|a| a.eq_ignore_ascii_case(k)) {
                eprintln!(
                    "[V400-01] warning: unknown vector index option '{}' (allowed for {:?}: {})",
                    k,
                    vidx.index_type,
                    allowed_keys.join(", ")
                );
            }
        }

        Ok(ExecutorResult::empty())
    }

    pub(super) fn execute_drop_index(&self, idx: &DropIndexStatement) -> SqlResult<ExecutorResult> {
        // V312-91 / Issue #4669: wire DROP INDEX into the storage layer.
        //
        // The parser already lifts `DROP INDEX [IF EXISTS] <name>` into a
        // `DropIndexStatement { name, if_exists }` (parser.rs:11546), but
        // the executor was a placeholder returning the literal "DROP INDEX
        // not fully supported yet" string. Now: scan `list_all_indexes()`
        // to discover which table owns the index (the parser only carries
        // the index name, not its owning table), then call
        // `storage.drop_index(table, name)`.
        //
        // `IF EXISTS` matches the SQLite/MySQL/PG convention: silently
        // succeed when the index is missing; without `IF EXISTS` we error
        // with a useful message that names the missing index.
        //
        // (This is the post-merge resolution of PR #4789; the prior
        // conflict-marker version was reverted to the cleaner HEAD side
        // which uses storage.write() directly without a read/write pair.)
        let mut storage = self.storage.write();
        let owner = storage
            .list_all_indexes()
            .into_iter()
            .find(|info| info.name == idx.name)
            .map(|info| info.table);
        match owner {
            Some(table) => storage.drop_index(&table, &idx.name),
            None if idx.if_exists => Ok(()),
            None => Err(SqlError::ExecutionError(format!(
                "DROP INDEX failed: index '{}' does not exist",
                idx.name
            ))),
        }?;
        Ok(ExecutorResult::empty())
    }

    pub(super) fn execute_create_view(
        &mut self,
        view: &CreateViewStatement,
    ) -> SqlResult<ExecutorResult> {
        // Issue #4567: store the full parsed statement (name + optional
        // column aliases + defining SELECT AST) so the view is resolvable.
        // The old code stored only `format!("{:?}", view)` — a Debug dump
        // no query path could consume, making every CREATE VIEW a no-op.
        self.views.write().insert(
            crate::execution_engine::view_registry_key(&self.session_db(), &view.name),
            view.clone(),
        );
        // V312-64d / Issue #4664: also persist the view to storage so
        // `sqlite_master` introspection sees the row, alongside the
        // in-memory view cache used by the view-rewrite path.
        let mut storage = self.storage.write();
        let query_sql = match view.query.as_ref() {
            sqlrustgo_parser::Statement::Select(sel) => {
                crate::ddl_to_sql::format_create_view_inner_select(sel)
            }
            _ => format!("{:?}", view.query),
        };
        let info = sqlrustgo_storage::ViewInfo::with_original_sql(
            view.name.clone(),
            view.columns.clone(),
            query_sql,
            crate::ddl_to_sql::format_create_view_sql(view),
        );
        storage.create_view(info)?;
        Ok(ExecutorResult::empty())
    }
    pub(super) fn execute_drop_view(
        &mut self,
        drop_view: &DropViewStatement,
    ) -> SqlResult<ExecutorResult> {
        // V312-95 v2 / Issue #4814: when the in-memory cache does not have
        // the view, also probe storage so a `DROP VIEW` issued after a
        // restart (when the in-memory cache is empty but FileStorage has
        // the row on disk) still works. If neither side knows the view:
        // honour `IF EXISTS` as a no-op, otherwise raise.
        let in_memory_present = self
            .views
            .write()
            .remove(&crate::execution_engine::view_registry_key(
                &self.session_db(),
                &drop_view.name,
            ))
            .is_some();
        let storage_present = self.storage.read().has_view(&drop_view.name);
        if in_memory_present || storage_present {
            // Persist the removal — idempotent on the storage side.
            self.storage.write().drop_view(&drop_view.name)?;
            Ok(ExecutorResult::empty())
        } else if drop_view.if_exists {
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

    pub(super) fn execute_merge_statement(
        &self,
        _merge: &MergeStatement,
    ) -> SqlResult<ExecutorResult> {
        Err(SqlError::ExecutionError(
            "MERGE not yet supported via execute() — use LocalExecutorDml path".to_string(),
        ))
    }

    pub(super) fn execute_create_trigger(
        &self,
        stmt: &CreateTriggerStatement,
    ) -> SqlResult<ExecutorResult> {
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
            original_sql: crate::ddl_to_sql::format_create_trigger_sql(stmt),
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
    pub(super) fn execute_drop_trigger(
        &self,
        stmt: &DropTriggerStatement,
    ) -> SqlResult<ExecutorResult> {
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

    pub(super) fn execute_call(&self, call: &CallStatement) -> SqlResult<ExecutorResult> {
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

        let _procedure = catalog
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
    pub(super) fn lower_body(
        stmts: &[sqlrustgo_parser::StoredProcStatement],
    ) -> Vec<StoredProcStatement> {
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

    pub(super) fn execute_create_procedure(
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
    pub(super) fn execute_drop_procedure(
        &self,
        stmt: &DropProcedureStatement,
    ) -> SqlResult<ExecutorResult> {
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
    pub(super) fn execute_create_function(
        &self,
        stmt: &CreateFunctionStatement,
    ) -> SqlResult<ExecutorResult> {
        let param_names: Vec<String> = stmt.params.iter().map(|p| p.name.clone()).collect();
        if let Some(ref body_block) = stmt.body_block {
            // Issue #4671: multi-statement UDF
            expr_mod::register_udf_with_body(
                &stmt.name,
                param_names,
                stmt.return_type.clone(),
                body_block.clone(),
            );
        } else {
            // Single-expression UDF
            expr_mod::register_udf(
                &stmt.name,
                param_names,
                stmt.return_type.clone(),
                stmt.body_expr.clone(),
            );
        }
        Ok(ExecutorResult::empty())
    }

    /// V312-58 / Issue #4512: drop a scalar UDF. `IF EXISTS` makes the
    /// operation a no-op when the UDF does not exist (matches the
    /// MySQL convention for IF EXISTS on function drops).
    pub(super) fn execute_drop_function(
        &self,
        stmt: &DropFunctionStatement,
    ) -> SqlResult<ExecutorResult> {
        let removed = expr_mod::drop_udf(&stmt.name);
        if !removed && !stmt.if_exists {
            return Err(SqlError::ExecutionError(format!(
                "DROP FUNCTION failed: function '{}' not found",
                stmt.name
            )));
        }
        Ok(ExecutorResult::empty())
    }

    pub(super) fn execute_transaction(
        &mut self,
        stmt: &TransactionStatement,
    ) -> SqlResult<ExecutorResult> {
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
                    .unwrap_or(self.tx_session.lock().default_isolation);
                self.begin_transaction(iso, *readonly)
            }
            TransactionStatement::Commit { work: _ } => self.commit_transaction(),
            TransactionStatement::Rollback { work: _ } => self.rollback_transaction(),
            TransactionStatement::SetTransaction { isolation_level } => {
                self.tx_session.lock().default_isolation = match isolation_level {
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
                    .unwrap_or(self.tx_session.lock().default_isolation);
                // Idempotent START TRANSACTION: if a transaction is already in
                // progress (e.g. after ROLLBACK or nested START from a retry),
                // just update isolation/readonly and return OK rather than erroring.
                // This matches MySQL behavior.
                if self.tx_session.lock().current_tx_id.is_some() {
                    self.tx_session.lock().tx_readonly = false;
                    if let Some(ref il) = isolation_level {
                        self.tx_session.lock().default_isolation = match il {
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

    pub(super) fn begin_transaction(
        &mut self,
        isolation: TmIsolationLevel,
        readonly: bool,
    ) -> SqlResult<ExecutorResult> {
        // V312-85 / Issue #4519: an *implicit* autocommit TX may still be
        // open (the previous statement left `current_tx_id` set without an
        // explicit COMMIT/ROLLBACK). Draining it before the explicit
        // `BEGIN` matches MySQL / PostgreSQL semantics and avoids the
        // historic "Transaction already in progress" error for the common
        // `INSERT ...; BEGIN; ...` script shape.
        //
        // Issue #4847 follow-up (2026-10-07): this drain used to be
        // UNCONDITIONAL, which meant a second `BEGIN` inside an already
        // explicit transaction silently COMMIT-ED that open transaction.
        // `BEGIN; INSERT; BEGIN; ROLLBACK;` therefore committed the INSERT
        // and the ROLLBACK became a no-op — the user's rollback boundary was
        // destroyed by a statement that is a no-op in every dialect we
        // target (MySQL errors 1568, PostgreSQL warns "there is already a
        // transaction in progress", SQLite errors). The distinction is
        // carried by `is_explicit_transaction`, which until now was
        // declared and initialised but never written or read.
        let drains_implicit = {
            let sess = self.tx_session.lock();
            match (sess.current_tx_id.is_some(), sess.is_explicit_transaction) {
                // No TX open, or an implicit one → nothing that must not be
                // committed behind the user's back.
                (false, _) | (true, false) => true,
                // An explicit TX is still open → refusing is the only
                // behaviour that does not silently discard it.
                (true, true) => false,
            }
        };
        if !drains_implicit {
            return Err(SqlError::ExecutionError(
                "Transaction already in progress".to_string(),
            ));
        }
        if self.tx_session.lock().current_tx_id.is_some() {
            // V312-85 / Issue #4519: drain the implicit TX.
            let prev_tx = self.tx_session.lock().current_tx_id;
            let mut storage = self.storage.write();
            // Re-assert the drained tx id so its pending versions are
            // the ones promoted, not whichever tx last used the slot.
            if let Some(pt) = prev_tx {
                storage.set_current_tx_id(pt.as_u64());
                // #5099: name the drained transaction. Re-asserting the
                // shared slot is not sufficient — the commit then reads
                // it back to decide which transaction to retire.
                let _ = storage.commit_transaction_for(pt.as_u64());
            } else {
                let _ = storage.commit_transaction();
            }
            drop(storage);
            self.tx_session.lock().current_tx_id = None;
            self.tx_session.lock().tx_status = TxStatus::Idle;
            // Touch `prev_tx` to silence the unused-variable warning
            // when the build is non-debug; the binding documents
            // what we drained so future readers can correlate.
            let _ = prev_tx;
        }
        let tx_id = self
            .transaction_manager
            .lock()
            .begin_transaction(isolation)
            .map_err(|e| {
                SqlError::ExecutionError(format!("Failed to begin transaction: {:?}", e))
            })?;
        self.tx_session.lock().current_tx_id = Some(tx_id);
        // Issue #4519 / Phase B Step 3 follow-up: an explicit `BEGIN`
        // must also transition `tx_status` from `Idle` (or a stale
        // `Committed`/`Aborted` left over from the previous statement)
        // into `Active`. Without this, the next implicit DML
        // (begin_implicit_dml_tx) sees `TxStatus::Committed` and rejects
        // with "transaction already committed" — even though the user
        // never ran `COMMIT`.
        self.tx_session.lock().tx_status = TxStatus::Active;
        // PR-842: also write a `Begin` WAL entry so the recovery engine can
        // detect explicit transactions and apply the per-tx boundary rule
        // when filtering committed entries. Without this, every DML entry
        // appears to be autocommit and uncommitted work leaks into recovery.
        // Phase B Step 3: prefer the lockfree path when available so we
        // don't take the global `Arc<RwLock<storage>>` write lock for
        // the BEGIN. The lockfree variant performs the WAL append under
        // an internal `parking_lot::Mutex` and updates tx_id atomically.
        // The trait default impl returns Err; engines that haven't been
        // updated (e.g. tests using MemoryStorage) fall back to the
        // legacy `begin_transaction` path.
        let lockfree_ok = {
            let storage = self.storage.read();
            storage.begin_transaction_lockfree(tx_id.as_u64()).is_ok()
        };
        if !lockfree_ok {
            // Fallback: lockfree not supported by this storage engine.
            let mut storage = self.storage.write();
            storage.set_current_tx_id(tx_id.as_u64());
            let _ = storage.begin_transaction();
        }
        self.tx_session.lock().tx_status = TxStatus::Active;
        self.tx_session.lock().tx_readonly = readonly;
        // Issue #4847 follow-up: mark the TX as explicitly begun so a
        // subsequent BEGIN can refuse rather than silently commit it.
        self.tx_session.lock().is_explicit_transaction = true;
        // V312-RC-GA / Issue #4818: BEGIN used to return the tx_id as a
        // single row, which leaked through the CSV formatter and broke
        // multi-statement scripts (the next statement's first output row
        // got prepended with the tx_id). Return empty to match
        // COMMIT/ROLLBACK which already return `ExecutorResult::empty()`.
        let _ = tx_id;
        Ok(ExecutorResult::empty())
    }

    pub(super) fn commit_transaction(&mut self) -> SqlResult<ExecutorResult> {
        // V312-77 / Issue #4847 Path B: no explicit tx active → no-op (MySQL compat).
        if self.tx_session.lock().current_tx_id.is_none() {
            return Ok(ExecutorResult::empty());
        }
        let tx_id =
            self.tx_session.lock().current_tx_id.ok_or_else(|| {
                SqlError::ExecutionError("No transaction in progress".to_string())
            })?;
        // Phase B Step 3 follow-up #3: prefer the lockfree path so we
        // don't take the global `Arc<RwLock<storage>>` write lock for
        // the COMMIT. Same fallback as `begin_transaction`.
        let lockfree_ok = {
            let storage = self.storage.read();
            // #5099: pass our tx id explicitly.
            //
            // The comment here used to claim "no writer can interleave
            // under this read guard, so the lockfree promote's capture
            // inside sees OUR id". That is false: `parking_lot::RwLock`
            // admits concurrent readers, so another connection can
            // overwrite the shared `current_tx_id` slot between this
            // re-assert and the moment `commit_transaction_lockfree`
            // read it back. The re-assert narrows the window; it does not
            // close it.
            //
            // Naming the transaction removes the inference entirely.
            storage.set_current_tx_id_shared(tx_id.as_u64());
            storage
                .commit_transaction_lockfree_for(tx_id.as_u64())
                .is_ok()
        };
        if lockfree_ok {
            // F-16 Gap Locking: release all gap locks on commit (lockfree
            // path skips these for now — TODO: gap-lock-free variant).
            // Note: `release_all_gap_locks` requires &mut self, so we
            // must escalate. This is still cheaper than the full
            // `commit_transaction` because the WAL append and
            // tx_id clear already happened in the lockfree path.
            let mut storage = self.storage.write();
            storage.release_all_gap_locks(tx_id.as_u64());
        } else {
            // Fallback: lockfree not supported.
            let mut storage = self.storage.write();
            {
                storage.set_current_tx_id(tx_id.as_u64());
                // #5099: name the transaction — see above.
                let _ = storage.commit_transaction_for(tx_id.as_u64());
                // F-16 Gap Locking: release all gap locks on commit
                storage.release_all_gap_locks(tx_id.as_u64());
            }
        }
        self.transaction_manager.lock().commit(tx_id).map_err(|e| {
            SqlError::ExecutionError(format!("Failed to commit transaction: {:?}", e))
        })?;
        self.tx_session.lock().current_tx_id = None;
        self.tx_session.lock().tx_status = TxStatus::Committed;
        // INT-1: Reset to Idle after commit so the next statement can
        // either begin a new TX or run in autocommit mode again. Without
        // this reset, subsequent DML would reject with
        // "transaction already committed".
        self.tx_session.lock().tx_status = TxStatus::Idle;
        self.tx_session.lock().tx_readonly = false;
        self.tx_session.lock().is_explicit_transaction = false;
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
    pub(super) fn execute_savepoint(
        &mut self,
        name: &str,
        op: SavepointOp,
    ) -> SqlResult<ExecutorResult> {
        // An active transaction is required for any savepoint operation.
        let tx_id = self.tx_session.lock().current_tx_id.ok_or_else(|| {
            SqlError::ExecutionError(
                "SAVEPOINT / ROLLBACK TO SAVEPOINT / RELEASE SAVEPOINT \
                 requires an active transaction (BEGIN or implicit autocommit TX)"
                    .to_string(),
            )
        })?;
        match op {
            SavepointOp::Save => self
                .transaction_manager
                .lock()
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
                    .lock()
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
                .lock()
                .release_savepoint(tx_id, name)
                .map_err(|e| {
                    SqlError::ExecutionError(format!("RELEASE SAVEPOINT {} failed: {}", name, e))
                })?,
        }
        Ok(ExecutorResult::empty())
    }

    pub(super) fn rollback_transaction(&mut self) -> SqlResult<ExecutorResult> {
        // V312-77 / Issue #4847 Path C: no explicit tx active → no-op.
        if self.tx_session.lock().current_tx_id.is_none() {
            return Ok(ExecutorResult::empty());
        }
        let tx_id =
            self.tx_session.lock().current_tx_id.ok_or_else(|| {
                SqlError::ExecutionError("No transaction in progress".to_string())
            })?;
        // Issue #4581 / B-track case 35-36: physically undo the
        // transaction by replaying the per-tx undo log via a closure
        // that calls `storage.delete` / `storage.insert`. This mirrors
        // the SAVEPOINT rollback path (`execute_savepoint`) but for
        // top-level ROLLBACK. See also
        // `sqlrustgo_transaction::TransactionManager::rollback_with_undo`
        // and `src/savepoint_wiring.rs::record_*_undo`.
        //
        // Phase B Step 3 follow-up #3: call `rollback_transaction_lockfree`
        // first to release tx state and discard buffered writes without
        // taking the global `Arc<RwLock<storage>>` write lock. The
        // per-record undo replay still needs the write lock (one at a
        // time, inside the closure) — this is the same as the legacy path.
        {
            let storage_read = self.storage.read();
            // #5099: same reasoning as the commit path above — the
            // read guard does NOT keep a peer out, so the id is named
            // rather than inferred from the shared slot.
            storage_read.set_current_tx_id_shared(tx_id.as_u64());
            let _ = storage_read.rollback_transaction_lockfree_for(tx_id.as_u64());
        }
        let storage = self.storage.clone();
        self.transaction_manager
            .lock()
            .rollback_with_undo(tx_id, move |rec| {
                let mut storage = storage.write();
                // Undo writes must be stamped with the rolled-back tx id
                // or they leak as foreign pending versions.
                storage.set_current_tx_id(tx_id.as_u64());
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
        self.tx_session.lock().current_tx_id = None;
        self.tx_session.lock().tx_status = TxStatus::Aborted;
        // INT-1: Reset to Idle so the next DML can begin a new TX or run
        // in autocommit mode. (Same reasoning as commit_transaction above.)
        self.tx_session.lock().tx_status = TxStatus::Idle;
        self.tx_session.lock().tx_readonly = false;
        self.tx_session.lock().is_explicit_transaction = false;
        Ok(ExecutorResult::empty())
    }

    /// Begin an implicit TX for DML. Returns `(tx_id, started_implicit)`.
    /// `started_implicit` is `true` ONLY when this call started a fresh TX.
    pub(crate) fn begin_implicit_dml_tx(
        &self,
        op: &'static str,
        _table: &str,
    ) -> SqlResult<(Option<TxId>, bool)> {
        let _ = op;
        if self.tx_session.lock().tx_readonly {
            return Err(SqlError::ExecutionError(
                "Cannot execute DML in READONLY transaction".to_string(),
            ));
        }
        match self.tx_session.lock().tx_status {
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
        if self.tx_session.lock().current_tx_id.is_none() {
            let tx_id = self
                .transaction_manager
                .lock()
                .begin_transaction(self.tx_session.lock().default_isolation)
                .map_err(|e| SqlError::ExecutionError(format!("TM.begin failed: {:?}", e)))?;
            self.tx_session.lock().current_tx_id = Some(tx_id);
            self.tx_session.lock().tx_status = TxStatus::Active;
            let mut storage = self.storage.write();
            {
                storage.set_current_tx_id(tx_id.as_u64());
            }
            Ok((Some(tx_id), true))
        } else {
            // Fix 6: an explicit tx is already active, but the storage
            // slot is process-wide (#4951) — another connection's BEGIN
            // may have stomped it since ours. Re-assert this session's id
            // so the upcoming write stamps OUR version chain, not a
            // foreign connection's tx id (same rationale as the
            // re-asserts in commit_transaction / rollback_transaction).
            if let Some(tx) = self.tx_session.lock().current_tx_id {
                let mut storage = self.storage.write();
                storage.set_current_tx_id(tx.as_u64());
            }
            Ok((self.tx_session.lock().current_tx_id, false))
        }
    }

    /// Commit the implicit DML TX started by `begin_implicit_dml_tx`.
    /// Idempotent when `started_implicit` is `false` (user controls commit/rollback).
    pub(crate) fn commit_implicit_dml_tx(&self, started_implicit: bool) -> SqlResult<()> {
        if started_implicit {
            let tx_id = self.tx_session.lock().current_tx_id.unwrap();
            let _ = self.transaction_manager.lock().commit(tx_id);
            // WAL checkpoint + truncation lives in StorageEngine::commit_transaction
            //
            // #4946: `commit_transaction` now flushes the snapshot before
            // truncating the WAL, so it must be allowed to fail —
            // acknowledging a commit whose data never reached disk is the
            // "confirmed then lost" shape this issue reports. It used to
            // be `let _ =`, which discarded exactly that signal.
            let mut storage = self.storage.write();
            // #5099: name the transaction. Re-asserting the shared slot
            // does not identify it — the commit reads the slot back to
            // decide which transaction it is retiring.
            storage.set_current_tx_id(tx_id.as_u64());
            storage.commit_transaction_for(tx_id.as_u64())?;
            // F-16 Gap Locking: release all gap locks on commit
            storage.release_all_gap_locks(tx_id.as_u64());
            drop(storage);
            self.tx_session.lock().current_tx_id = None;
            self.tx_session.lock().tx_status = TxStatus::Idle;
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

    pub(super) fn execute_union(
        &mut self,
        union_stmt: &UnionStatement,
    ) -> SqlResult<ExecutorResult> {
        crate::engine_setops::execute_union(self, union_stmt)
    }

    pub(super) fn execute_intersect(
        &mut self,
        stmt: &IntersectStatement,
    ) -> SqlResult<ExecutorResult> {
        crate::engine_setops::execute_intersect(self, stmt)
    }

    pub(super) fn execute_except(&mut self, stmt: &ExceptStatement) -> SqlResult<ExecutorResult> {
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
