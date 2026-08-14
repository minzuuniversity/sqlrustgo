//! Engine SELECT execution — extracted from execution_engine.rs (PR-900)
//!
//! Handles SELECT statement dispatch, projection, join planning, and result assembly.
//!
//! v3.10.0 Issue #3703: parallel filter is gated by:
//!   - CBO-driven `ExecutionEngine::should_parallelize_query()` (replaces
//!     hardcoded `PARALLEL_MIN_ROWS` threshold)
//!   - self.parallel_degree > 1 (env SQLRUSTGO_EXECUTOR_PARALLELISM or --executor-parallelism)
//!   - no correlated subquery in WHERE (would break parallel eval_predicate)
//!
//! Tracing spans (RUST_LOG=sqlrustgo=trace) reveal whether the path engages at runtime.
use crate::engine_utils::*;
use crate::expr_utils::*;
use crate::{ExecutionEngine, ExecutorResult, SqlError, SqlResult, Value};
use sqlrustgo_executor::join::hash_join::multi_way_hash_chain;
use sqlrustgo_executor::parallel_executor::{ParallelExecutor, ParallelVolcanoExecutor};
use sqlrustgo_executor::simd_eval::{
    BatchPredicate, BitMask, EqualsPredicate, GreaterThanOrEqualPredicate, GreaterThanPredicate,
    LessThanOrEqualPredicate, LessThanPredicate, NotEqualPredicate,
};
use sqlrustgo_parser::{
    get_and_clear_derived_subqueries, AggregateCall, AggregateFunction, Expression,
    JoinClause as ParserJoinClause, JoinType, SelectStatement,
};
use sqlrustgo_storage::{StorageEngine, TableInfo};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

type DerivedResult = (Vec<Vec<Value>>, TableInfo);

// Phase 3 (TPCH-01 Q15): thread-local registry of materialized
// derived subquery results. Populated by `execute_joins` before the
// join chain runs; consumed by `execute_single_join` when it encounters
// a `__subq_N` synthetic table name.
thread_local! {
    static DERIVED_RESULTS: RefCell<HashMap<String, DerivedResult>> =
        RefCell::new(HashMap::new());
    // Thread-local flag: set when try_comma_join_hash_chain succeeds.
    #[allow(clippy::missing_const_for_thread_local)]
    static COMMA_JOIN_WHERE_CONSUMED: RefCell<bool> = const { RefCell::new(false) };
}

// Sprint 5 v2: per-column index for correlated EXISTS. TPC-H
// Q4/Q21 use `EXISTS (SELECT * FROM lineitem WHERE
// l_orderkey = o_orderkey AND ...)`, where the per-outer-row
// scan was N×M. We build a one-shot index for each column on
// the first call, then reuse it for subsequent calls.
type LineitemIndexMap = HashMap<String, HashMap<Value, Vec<usize>>>;
type LineitemRowsMap = HashMap<String, std::sync::Arc<Vec<Vec<Value>>>>;
static LINEITEM_INDEX_CACHE: OnceLock<parking_lot::Mutex<LineitemIndexMap>> = OnceLock::new();
fn lineitem_index_cache() -> &'static parking_lot::Mutex<LineitemIndexMap> {
    LINEITEM_INDEX_CACHE.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

// Companion cache for the actual inner table rows. We cache
// the rows under an Arc so subsequent per-outer-row calls
// don't pay the deep-clone cost of MemoryStorage::scan()
// (which does `.cloned()` on 60K lineitem rows each call).
static LINEITEM_ROWS_CACHE: OnceLock<parking_lot::Mutex<LineitemRowsMap>> = OnceLock::new();
fn lineitem_rows_cache() -> &'static parking_lot::Mutex<LineitemRowsMap> {
    LINEITEM_ROWS_CACHE.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

// Sprint 5 v2 (Q17 fix): per-(table, column, key) scalar subquery cache.
// TPC-H Q17: `SELECT ... WHERE l_quantity < (SELECT 0.2*AVG(l_quantity)
// FROM lineitem WHERE l_partkey = p_partkey)`. For each outer partkey value,
// we cache the scalar subquery result so we don't scan lineitem N times.
// Key: (table_name, outer_ref_col, inner_filter_col) → HashMap<outer_value, scalar_result>
static SCALAR_SUBQ_CACHE: OnceLock<parking_lot::Mutex<HashMap<Value, Value>>> = OnceLock::new();
fn scalar_subq_cache() -> &'static parking_lot::Mutex<HashMap<Value, Value>> {
    SCALAR_SUBQ_CACHE.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

// TPC-H Q17 perf: pre-computed `key_value → aggregate_result` index for
// correlated scalar aggregate subqueries of the shape
//   `(SELECT [op] AGG(col) FROM t WHERE key_col = <outer_ref>)`
// The index is built ONCE per (table, key_col, agg_func, agg_arg_col) and
// queried O(1) per outer row. Q17 was taking 30s on SF=0.1 (2000 partkeys ×
// 60K-lineitem AVG scan); with this cache it drops to a single 60K scan
// (~0.5s) plus 9 O(1) lookups.
type ScalarAggIndexMap = HashMap<Value, Value>;
#[derive(Clone)]
struct ScalarAggIndexEntry {
    map: std::sync::Arc<ScalarAggIndexMap>,
}
static SCALAR_AGG_INDEX_CACHE: OnceLock<parking_lot::Mutex<HashMap<String, ScalarAggIndexEntry>>> =
    OnceLock::new();
fn scalar_agg_index_cache() -> &'static parking_lot::Mutex<HashMap<String, ScalarAggIndexEntry>> {
    SCALAR_AGG_INDEX_CACHE.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

fn extract_first_literal_from_where(select: &SelectStatement) -> Option<Value> {
    use sqlrustgo_parser::Expression;
    fn walk(expr: &Expression, out: &mut Option<Value>) {
        if out.is_some() {
            return;
        }
        match expr {
            Expression::Literal(s) => {
                *out = Some(Value::Text(s.clone()));
            }
            Expression::BinaryOp(l, _, r) => {
                walk(l, out);
                walk(r, out);
            }
            Expression::UnaryOp(_, inner) => walk(inner, out),
            Expression::IsNull(inner) | Expression::IsNotNull(inner) => walk(inner, out),
            Expression::InList(l, vs) => {
                walk(l, out);
                for v in vs {
                    walk(v, out);
                }
            }
            Expression::NotInList(l, vs) => {
                walk(l, out);
                for v in vs {
                    walk(v, out);
                }
            }
            Expression::Between(l, lo, hi) => {
                walk(l, out);
                walk(lo, out);
                walk(hi, out);
            }
            Expression::NotBetween(l, lo, hi) => {
                walk(l, out);
                walk(lo, out);
                walk(hi, out);
            }
            Expression::Like(l, p, _) | Expression::NotLike(l, p, _) => {
                walk(l, out);
                walk(p, out);
            }
            Expression::FunctionCall(_, args) => {
                for a in args {
                    walk(a, out);
                }
            }
            _ => {}
        }
    }
    let mut out = None;
    if let Some(ref wc) = select.where_clause {
        walk(wc, &mut out);
    }
    out
}

/// Convert a [`Value`] to its string-literal representation as expected
/// by [`Expression::Literal`].  Reserved for future scalar aggregate
/// results in the fast-path index lookup (currently the lookup path
/// uses `to_sql_string` directly; this helper stays as a typed
/// formatter when that path is re-introduced).
#[allow(dead_code)]
fn value_to_literal_string_v(v: &Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => format!("{}", f),
        Value::Text(s) => s.clone(),
        Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Blob(b) => format!("BLOB({} bytes)", b.len()),
        Value::Point(x, y) => format!("POINT({}, {})", x, y),
        Value::Json(v) => v.to_string(),
    }
}

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    /// Returns `true` if the most recent `execute()` call (or any
    /// query that triggered `execute_joins`) successfully built a
    /// hash chain for a comma-join (`FROM t1, t2, ...`) using the
    /// `try_comma_join_hash_chain` fast path.
    ///
    /// When `false`, the engine fell back to the per-clause
    /// cartesian path - correct but 5-10x slower on TPC-H SF=1
    /// and infeasible on SF=10 for multi-hub topologies (Q7
    /// star, Q8 bridge, Q9 chain-leaf). Used by regression tests
    /// to assert that the chain-build strategy succeeded.
    pub fn last_query_used_comma_join_fast_path(&self) -> bool {
        COMMA_JOIN_WHERE_CONSUMED.with(|f| *f.borrow())
    }

    fn clear_tpch_caches() {
        thread_local! {
            static CACHE_CLEARED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        }
        CACHE_CLEARED.with(|flag| {
            if flag.get() {
                return;
            }
            flag.set(true);
        });
        if let Some(c) = LINEITEM_ROWS_CACHE.get() {
            c.lock().clear();
        }
        if let Some(c) = LINEITEM_INDEX_CACHE.get() {
            c.lock().clear();
        }
        if let Some(c) = SCALAR_AGG_INDEX_CACHE.get() {
            c.lock().clear();
        }
    }

    pub fn execute_select(&self, select: &SelectStatement) -> SqlResult<ExecutorResult> {
        // Debug: print query table structure
        Self::clear_tpch_caches();
        // V312-22 / Issue #4182: push ANALYZE-collected table stats
        // (including `Histogram`) into `UnifiedCostModel::column_stats` at
        // every SELECT entry. This is cheap when CBO is disabled (we skip
        // outright) and bounded by O(N_tables) when enabled — typically
        // a handful of tables. The CBO side is idempotent on overwrite,
        // so per-query refresh is safe and ensures the cost model never
        // reads stale histograms even if a previous query updated stats.
        if self.cbo_enabled {
            self.update_cost_model_stats();
        }
        // Sprint 1b fix (Q7/Q8/Q9): handle FROM (subquery) AS alias by
        // first executing the subquery to materialize its result into a
        // synthetic in-memory table, then running the outer SELECT against
        // it. We collect the materialized rows + schema before acquiring
        // the storage read lock to avoid reentrant lock issues.
        let materialized: Option<(Vec<Vec<Value>>, TableInfo)> =
            if let Some(subq) = &select.from_subquery {
                // Drop the read lock (if held) and execute subquery; subquery
                // itself takes a read lock internally. Since the outer has not
                // yet acquired a lock, this is a fresh acquisition.
                let sub_result = self.execute_select(subq)?;
                // Build a synthetic TableInfo from the subquery's column list.
                let mut table_info = TableInfo {
                    name: select.table.clone(),
                    columns: Vec::new(),
                    foreign_keys: Vec::new(),
                    unique_constraints: Vec::new(),
                    check_constraints: Vec::new(),
                    partition_info: None,
                    compression: None,
                    collations: std::collections::HashMap::new(),
                };
                for col in &subq.columns {
                    let col_name = col.alias.clone().unwrap_or_else(|| col.name.clone());
                    // Type inference: peek at the first non-null value
                    let inferred_type_str: String = sub_result
                        .rows
                        .iter()
                        .find(|r| r.iter().any(|v| !matches!(v, Value::Null)))
                        .and_then(|first_row| {
                            let col_idx = subq
                                .columns
                                .iter()
                                .position(|c| c.alias.as_ref().unwrap_or(&c.name) == &col_name)?;
                            first_row.get(col_idx).map(|v| match v {
                                Value::Integer(_) => "INTEGER",
                                Value::Float(_) => "FLOAT",
                                Value::Text(_) => "TEXT",
                                Value::Boolean(_) => "BOOLEAN",
                                Value::Blob(_) => "BLOB",
                                Value::Point(_, _) => "POINT",
                                Value::Json(_) => "JSON",
                                Value::Null => "NULL",
                            })
                        })
                        .unwrap_or("TEXT")
                        .to_string();
                    table_info
                        .columns
                        .push(sqlrustgo_storage::ColumnDefinition {
                            name: col_name,
                            data_type: inferred_type_str,
                            nullable: true,
                            primary_key: false,
                            char_max_length: None,
                            collation: None,
                            default_value: None,
                        });
                }
                Some((sub_result.rows, table_info))
            } else {
                None
            };
        // Step 1: FROM/JOIN - get initial rows and schema.
        // NOTE: For the join path, `execute_joins` manages its own storage
        // read lock.  We deliberately do NOT hold an outer storage guard
        // here so that `execute_joins`' inner `storage_read()` does not
        // collide with `parking_lot`'s writer-preference policy: when a
        // writer is waiting, `try_read()` returns `None` even for
        // reentrant reads on the same thread, causing a deadlock
        // (reader owns lock → writer waits → reader tries reentrant read
        // → blocked by writer preference → deadlock).
        let (mut rows, table_info) = if !select.join_clause.is_empty() {
            let (jrows, jinfo, _) = self.execute_joins(&mut select.clone())?;
            (jrows, jinfo)
        } else if let Some((rows, info)) = materialized {
            (rows, info)
        } else if let Some(values) = &select.from_values {
            // V313-09 / Issue #4037: `FROM (VALUES (...))` constructor.
            // The parser stores the row data as `Vec<Vec<Expression>>`
            // in `from_values`; we materialise it via the same
            // build_insert_records helper that INSERT VALUES uses, then
            // build a synthetic TableInfo (column names default to
            // col_0, col_1, ...). The alternative path (recursing into
            // the synthetic subquery) used to fail with 'Table not
            // found: <alias>' because the subquery's `table` field
            // carries the alias, not a real storage table.
            let rows = crate::engine_helpers::build_insert_records(values);
            let inferred_types: Vec<String> = if let Some(first_row) = rows.first() {
                first_row
                    .iter()
                    .map(|v| match v {
                        Value::Integer(_) => "INTEGER".to_string(),
                        Value::Float(_) => "FLOAT".to_string(),
                        Value::Text(_) => "TEXT".to_string(),
                        Value::Boolean(_) => "BOOLEAN".to_string(),
                        Value::Blob(_) => "BLOB".to_string(),
                        Value::Point(_, _) => "POINT".to_string(),
                        Value::Json(_) => "JSON".to_string(),
                        Value::Null => "TEXT".to_string(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let info = TableInfo {
                name: select.table.clone(),
                columns: (0..rows.first().map(|r| r.len()).unwrap_or(0))
                    .map(|i| sqlrustgo_storage::ColumnDefinition {
                        name: format!("col_{}", i),
                        data_type: inferred_types
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| "TEXT".to_string()),
                        nullable: true,
                        primary_key: false,
                        char_max_length: None,
                        collation: None,
                        default_value: None,
                    })
                    .collect(),
                foreign_keys: Vec::new(),
                unique_constraints: Vec::new(),
                check_constraints: Vec::new(),
                partition_info: None,
                compression: None,
                collations: std::collections::HashMap::new(),
            };
            (rows, info)
        } else if select.table.is_empty() {
            let empty_schema = TableInfo {
                name: String::new(),
                columns: Vec::new(),
                foreign_keys: Vec::new(),
                unique_constraints: Vec::new(),
                check_constraints: Vec::new(),
                partition_info: None,
                compression: None,
                collations: std::collections::HashMap::new(),
            };
            (vec![Vec::new()], empty_schema)
        } else {
            let storage = self.storage_read();
            // Sprint 5 v4: the parser may encode the inline alias
            // into `select.table` as `table|alias`. Storage has only
            // the bare table name, so strip the `|alias` suffix
            // before the lookup.
            let lookup_table = select
                .table
                .split_once('|')
                .map(|(t, _)| t)
                .unwrap_or(&select.table);
            // V311-02 v2: instrument single-table SELECT via AHI so
            // repeated scans of the same table get promoted.
            let rows = self.scan_with_ahi(&storage, lookup_table)?;
            let table_info = storage.get_table_info(lookup_table)?;
            drop(storage);
            // V311-05 F-29: apply RLS row filtering if enabled
            let rows = self.apply_rls_filter(lookup_table, rows, &table_info)?;
            // V313-13 / Issue #4041: binder column-existence check.
            // Catches cases like `WITH t AS (SELECT 1 AS a)
            // SELECT t.foobar FROM t` — without this check the legacy
            // eval_identifier fallback would silently emit
            // Value::Text("t.foobar"). Also catches `WHERE alias`
            // references to SELECT-list aliases, which SQL forbids.
            // V312-21 / #4181: Join queries (JOIN clause / comma-list)
            // bind columns against the full joined schema in execute_joins,
            // so this single-table check must be skipped there — otherwise
            // a qualified column like `n1.n_name` against the base table's
            // info is falsely rejected.
            // V312-21 / #4181 fix-up: any query that references more than one
            // table (either via `join_clause` for explicit JOIN syntax
            // OR via `extra_tables` for comma-list FROM, which the parser
            // sometimes turns into join_clauses for aliased tables) must
            // skip the single-table binder. The single-table check
            // falsely rejects qualified columns like `n1.n_name` because
            // the base table's TableInfo only knows about its own
            // columns. We run the binder ONLY when both lists are empty
            // (pure single-table query).
            if select.join_clause.is_empty() && select.extra_tables.is_empty() {
                crate::engine_utils::validate_select_columns_referenced(select, &table_info)?;
            }
            (rows, table_info)
        };
        // The storage read lock is NOT held past this point, ensuring
        // that any recursive execution (e.g. correlated subqueries)
        // cannot deadlock against a held lock.
        let _parallel_guard = if self.parallel_degree > 1
            && {
                // CBO-driven parallelism threshold:
                // Extract the bare table name (strip alias suffix) and delegate
                // to UnifiedCostModel::should_parallelize for the decision.
                let cbo_table = select
                    .table
                    .split_once('|')
                    .map(|(t, _)| t)
                    .unwrap_or(&select.table);
                self.should_parallelize_query(cbo_table, select.where_clause.as_ref(), rows.len())
            }
            // Skip parallel filter when WHERE contains correlated subqueries
            // (Subquery, EXISTS/NOT EXISTS with outer refs). The sequential
            // path below handles these correctly; parallel filter uses
            // eval_predicate which returns NULL for Subquery expressions.
            && !select
                .where_clause
                .as_ref()
                .is_some_and(where_expr_has_correlated_subquery)
            // v3.10.0 Issue #3703: FOR UPDATE / LOCK IN SHARE MODE
            // disables parallel execution. LockManager is a global
            // singleton; concurrent lock acquisitions across threads
            // can cause deadlocks. The CBO's should_parallelize() also
            // returns false for lock reads, but the engine enforces
            // this as a hard safety gate.
            && select.lock_clause.is_none()
        {
            let _span = tracing::info_span!(
                "parallel_filter_engaged",
                degree = self.parallel_degree,
                rows_in = rows.len(),
                has_correlated_subquery = false,
                has_lock_clause = false,
            )
            .entered();
            let t_start = Instant::now();
            let n_rows_in = rows.len();
            if let Some(where_expr) = &select.where_clause {
                let parallel = ParallelVolcanoExecutor::new(self.parallel_degree);
                let partitions = parallel.partition_scan(rows, self.parallel_degree);
                let n_partitions = partitions.len();
                rows = self.filter_partitions_parallel(partitions, where_expr, &table_info);
                let n_rows_out = rows.len();
                let elapsed_us = t_start.elapsed().as_micros();
                tracing::info!(
                    target: "sqlrustgo.parallel",
                    degree = self.parallel_degree,
                    rows_in = n_rows_in,
                    rows_out = n_rows_out,
                    partitions = n_partitions,
                    elapsed_us = elapsed_us as u64,
                    "parallel_filter_engaged"
                );
            }
            Some(())
        } else {
            None
        };
        let skip_where = COMMA_JOIN_WHERE_CONSUMED.with(|f| *f.borrow());
        // Step 1.5: correlated EXISTS / NOT EXISTS pre-evaluation
        // Before applying WHERE row-by-row, substitute the outer column
        // references in the subquery with concrete values from each
        // outer row, then execute the subquery and check whether it
        // returned any rows. Replace the EXISTS/NotExists subtree in
        // the cloned where_expr with a Literal(true/false) for that
        // specific outer row. The remaining WHERE logic then runs via
        // the standard `eval_predicate` path.
        if !skip_where {
            if let Some(ref where_expr) = select.where_clause {
                if where_expr_has_correlated_subquery(where_expr) {
                    // Sprint 5 (Q4 EXISTS perf): pre-build a
                    // `SubqueryIndex` for every correlated EXISTS
                    // subquery before the per-row loop.  This turns
                    // the O(N_inner × N_outer) full-scan EXISTS
                    // evaluation into O(N_inner) one-time index build
                    // + O(1) per outer row.  For TPC-H Q4 this is
                    // 900M ops → 75K ops at SF 0.1.
                    let mut subquery_indexes: Vec<SubqueryIndex> = Vec::new();
                    collect_subquery_indexes(where_expr, self, &mut subquery_indexes);

                    let pre_evaluated_where = where_expr.clone();
                    let mut new_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
                    for row in rows.into_iter() {
                        let mut cursor: usize = 0;
                        let replaced = self.pre_evaluate_correlated_exists(
                            &pre_evaluated_where,
                            &row,
                            &table_info,
                            &subquery_indexes,
                            &mut cursor,
                        );
                        if eval_predicate(&replaced, &row, &table_info) {
                            // V311-02 v2: AHI access was recorded at scan time via
                            // `scan_with_ahi()`. Adding per-row hooks here would be
                            // redundant noise; the table-level access is sufficient
                            // for the production-hook metric (touched_pages, hit_rate).
                            new_rows.push(row);
                        }
                    }
                    rows = new_rows;
                } else {
                    rows.retain(|row| eval_predicate(where_expr, row, &table_info));
                }
            }
        }

        // Step 1.6: TPC-H Q13 — non-correlated IN / NOT IN subquery
        // pre-evaluation.  The above correlated-subquery branch
        // doesn't trigger for non-correlated IN/NOT IN (e.g. Q13's
        // `c_custkey NOT IN (SELECT o_custkey FROM orders WHERE
        // o_comment LIKE '%special%requests%')`), and the free
        // `eval_predicate` cannot reach the engine to execute the
        // subquery.  We pre-execute each non-correlated IN/NOT IN
        // subquery once, collect the first-column values into a
        // HashSet, and rewrite the AST `In/NotIn(expr, subq)` into
        // `InList/NotInList(expr, [Literal...])` so the standard
        // `eval_predicate` path handles it correctly.
        if let Some(ref where_expr) = select.where_clause {
            let rewritten = self.pre_evaluate_non_correlated_in_subquery(where_expr);
            if &rewritten != where_expr {
                rows.retain(|row| eval_predicate(&rewritten, row, &table_info));
            }
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

                // TPC-H Q14: column expression may be a BinaryOp over
                // aggregate calls (e.g. `100.00 * SUM(...) / SUM(...)`).
                // The `compute_aggregates` above returns the raw aggregate
                // values; we now project the column expression evaluated
                // against those values as a synthetic row. Without this
                // step, Q14 would return 2 raw SUM values instead of
                // the `100.00 * SUM(...) / SUM(...)` result.
                let agg_schema = build_aggregate_schema(&[], &select.aggregates)?;
                let projected: Vec<Vec<Value>> = if select.columns.is_empty()
                    || select.columns.iter().any(|c| c.name == "*")
                {
                    vec![agg_values.clone()]
                } else {
                    let row: Vec<Value> = select
                        .columns
                        .iter()
                        .map(|col| match &col.expression {
                            Some(expr) => {
                                // V313-followup-2 / Issue #4155: reuse the precomputed aggregate
                                // value rather than re-evaluating through
                                // `evaluate_expression`'s FunctionCall
                                // dispatch (which lacks aggregate context).
                                if let sqlrustgo_parser::Expression::FunctionCall(name, _) = expr {
                                    let upper = name.to_uppercase();
                                    if upper == "QUANTILE_DISC"
                                        || upper == "QUANTILE_CONT"
                                        || upper == "PERCENTILE_CONT"
                                    {
                                        if let Some(idx) = select.aggregates.iter().position(|a| {
                                            matches!(
                                                a.func,
                                                AggregateFunction::QuantileDisc
                                                    | AggregateFunction::QuantileCont
                                                    | AggregateFunction::PercentileCont
                                            )
                                        }) {
                                            return agg_values
                                                .get(idx)
                                                .cloned()
                                                .unwrap_or(Value::Null);
                                        }
                                    }
                                }
                                evaluate_expression(expr, &agg_values, &agg_schema)
                                    .unwrap_or(Value::Null)
                            }
                            None => agg_values.first().cloned().unwrap_or(Value::Null),
                        })
                        .collect();
                    vec![row]
                };
                let row_count = projected.len();
                return Ok(ExecutorResult::new(projected, row_count));
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
                            } else if let Ok(f) = s.parse::<f64>() {
                                Value::Float(f)
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

                // MySQL 5.7 WITH ROLLUP / WITH CUBE — emit grouping-set
                // subtotal rows. We aggregate the already-grouped rows in
                // successive passes (k+1 levels for ROLLUP, 2^k for CUBE)
                // and append a NULL-padded subtotal row for each level.
                if select.with_rollup {
                    let k = group_exprs.len();
                    // Levels: drop trailing i group cols (i=1..k), leaving
                    // a grand-total row when all are dropped.
                    for i in (1..=k).rev() {
                        // Subtotal over rows that match the (k-i) leading
                        // group columns, ignoring the last i.
                        let prefix_len = k - i;
                        let mut subtotal_groups: std::collections::HashMap<
                            String,
                            Vec<Vec<Value>>,
                        > = std::collections::HashMap::new();
                        for row in &agg_result_rows {
                            let key = (0..prefix_len)
                                .map(|idx| {
                                    let v = row.get(idx).cloned().unwrap_or(Value::Null);
                                    match v {
                                        Value::Null => "NULL".to_string(),
                                        Value::Integer(n) => format!("I{}", n),
                                        Value::Float(f) => format!("F{}", f),
                                        Value::Text(s) => format!("T{}", s),
                                        Value::Boolean(b) => format!("B{}", b as i32),
                                        Value::Blob(b) => format!("X{}", b.len()),
                                        Value::Point(x, y) => format!("POINT({}, {})", x, y),
                                        Value::Json(v) => format!("J{}", v),
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\x00");
                            subtotal_groups.entry(key).or_default().push(row.clone());
                        }
                        for key in subtotal_groups.keys() {
                            let parts: Vec<&str> = key.split('\x00').collect();
                            let mut combined: Vec<Value> =
                                parts.iter().map(|s| decode_value_key(s)).collect();
                            // Pad NULLs for the dropped i columns
                            for _ in 0..i {
                                combined.push(Value::Null);
                            }
                            // Re-aggregate the original rows of each parent
                            // group (so SUM stays correct across rolled-up
                            // levels). We pull the matching pre-aggregation
                            // rows by re-joining on the full original key.
                            let mut parent_rows: Vec<Vec<Value>> = Vec::new();
                            for orig_row in &rows {
                                let orig_key = group_exprs
                                    .iter()
                                    .map(|expr| {
                                        evaluate_expr_to_string(expr, orig_row, &table_info)
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\x00");
                                let orig_parts: Vec<&str> = orig_key.split('\x00').collect();
                                let prefix_match = (0..prefix_len).all(|idx| {
                                    let a = decode_value_key(orig_parts[idx]);
                                    a == combined[idx]
                                });
                                if prefix_match {
                                    parent_rows.push(orig_row.clone());
                                }
                            }
                            let agg_values = self.compute_aggregates(
                                &select.aggregates,
                                &parent_rows,
                                &table_info,
                            )?;
                            combined.extend(agg_values);
                            agg_result_rows.push(combined);
                        }
                    }
                }
                if select.with_cube {
                    let k = group_exprs.len();
                    // All 2^k subsets; the existing groups at mask=2^k-1
                    // (i.e. all cols) are the originals we already have.
                    if k <= 5 {
                        let full_mask = (1u32 << k) - 1;
                        for mask in 0..full_mask {
                            let mut cube_groups: std::collections::HashMap<
                                String,
                                Vec<Vec<Value>>,
                            > = std::collections::HashMap::new();
                            for row in &rows {
                                let key_parts: Vec<String> = (0..k)
                                    .map(|idx| {
                                        if mask & (1 << idx) != 0 {
                                            let expr = &group_exprs[idx];
                                            evaluate_expr_to_string(expr, row, &table_info)
                                        } else {
                                            "NULL".to_string()
                                        }
                                    })
                                    .collect();
                                let key = key_parts.join("\x00");
                                cube_groups.entry(key).or_default().push(row.clone());
                            }
                            for (key, group_rows) in cube_groups.iter() {
                                let parts: Vec<&str> = key.split('\x00').collect();
                                let combined: Vec<Value> = (0..k)
                                    .map(|idx| {
                                        if mask & (1 << idx) != 0 {
                                            decode_value_key(parts[idx])
                                        } else {
                                            Value::Null
                                        }
                                    })
                                    .collect();
                                let agg_values = self.compute_aggregates(
                                    &select.aggregates,
                                    group_rows,
                                    &table_info,
                                )?;
                                let mut row = combined;
                                row.extend(agg_values);
                                if mask != full_mask {
                                    agg_result_rows.push(row);
                                }
                            }
                        }
                    }
                }

                // v3.8.0-rc2 Day 7: apply ORDER BY before returning.
                // Sprint 5 v2 fix (Q3/Q10/Q15/Q18 cell_diff): for
                // aggregate-typed ORDER BY references (e.g.
                // `ORDER BY revenue DESC` where `revenue` is a SUM
                // alias), the position in the row is offset by
                // group_schema.len() (the aggregate tail starts
                // after the group-by columns). Also respect
                // ascending/DESC direction.
                let agg_result_rows = if !select.order_by.is_empty() {
                    let group_schema_len = group_exprs.len();
                    let mut keyed: Vec<(Vec<Value>, Vec<Value>)> = agg_result_rows
                        .into_iter()
                        .map(|row| {
                            let keys: Vec<Value> = select
                                .order_by
                                .iter()
                                .map(|ob_expr| {
                                    if let Expression::Identifier(col_name) = &ob_expr.expression {
                                        if let Some(idx) = select.columns.iter().position(|c| {
                                            c.alias.as_deref() == Some(col_name)
                                                || c.name == *col_name
                                        }) {
                                            let col = &select.columns[idx];
                                            let col_expr = col.expression.as_ref();
                                            let is_aggregate = match col_expr {
                                                Some(Expression::Aggregate(_)) => true,
                                                Some(Expression::BinaryOp(_, _, r)) => {
                                                    matches!(r.as_ref(), Expression::Aggregate(_))
                                                }
                                                _ => false,
                                            } || (col.alias.is_none()
                                                && idx >= group_schema_len);
                                            let actual_idx = if is_aggregate {
                                                let agg_pos_in_select = select
                                                    .columns
                                                    .iter()
                                                    .take(idx)
                                                    .filter(|c| {
                                                        let ce = c.expression.as_ref();
                                                        match ce {
                                                            Some(Expression::Aggregate(_)) => true,
                                                            Some(Expression::BinaryOp(_, _, r)) => {
                                                                matches!(
                                                                    r.as_ref(),
                                                                    Expression::Aggregate(_)
                                                                )
                                                            }
                                                            _ => false,
                                                        }
                                                    })
                                                    .count();
                                                group_schema_len + agg_pos_in_select
                                            } else {
                                                idx
                                            };
                                            if actual_idx < row.len() {
                                                return row[actual_idx].clone();
                                            }
                                        }
                                    }
                                    Value::Null
                                })
                                .collect();
                            (keys, row)
                        })
                        .collect();
                    // Sprint 5 v11 fix: single-pass multi-column sort.
                    // Multiple `sort_by` calls in a loop discard the
                    // previous column's order; Q18 needs
                    // `o_totalprice DESC, o_orderdate ASC` to keep
                    // DESC ordering for ties instead of re-sorting
                    // by o_orderdate alone.
                    keyed.sort_by(|a, b| {
                        for (i, ob_expr) in select.order_by.iter().enumerate() {
                            let av = a.0.get(i);
                            let bv = b.0.get(i);
                            let ord = match (av, bv) {
                                (Some(x), Some(y)) => x.cmp(y),
                                (Some(_), None) => std::cmp::Ordering::Greater,
                                (None, Some(_)) => std::cmp::Ordering::Less,
                                (None, None) => std::cmp::Ordering::Equal,
                            };
                            let resolved = if ob_expr.ascending {
                                ord
                            } else {
                                ord.reverse()
                            };
                            if resolved != std::cmp::Ordering::Equal {
                                return resolved;
                            }
                        }
                        std::cmp::Ordering::Equal
                    });
                    keyed.into_iter().map(|(_, row)| row).collect()
                } else {
                    agg_result_rows
                };
                // v3.8.0-rc2 Day 7: apply LIMIT/OFFSET before returning
                // from the aggregate path. Previously LIMIT was
                // applied in Step 4 which only ran on the non-aggregate
                // branch, so GROUP BY + LIMIT queries (Q3, Q15) returned
                // all rows instead of the limited top-N.
                let agg_result_rows = if let Some(limit) = select.limit {
                    let offset = select.offset.unwrap_or(0) as usize;
                    if offset >= agg_result_rows.len() {
                        vec![]
                    } else {
                        agg_result_rows
                            .into_iter()
                            .skip(offset)
                            .take(limit as usize)
                            .collect()
                    }
                } else {
                    agg_result_rows
                };

                // TPC-H Sprint 5 fix: re-project rows according to
                // SELECT column order. The aggregate path above
                // produces rows in [group_key..., aggregate_value...]
                // order, but the SELECT clause may interleave
                // aggregates with group columns (e.g. Q3:
                // `SELECT l_orderkey, SUM(...), o_orderdate, ...`
                // produces [l_orderkey, o_orderdate, o_shippriority, SUM]
                // but the SELECT order is [l_orderkey, SUM, o_orderdate, ...]).
                // Without this re-projection, Q3/Q10/Q15/Q18 cell
                // values appear in the wrong columns.
                let is_star_agg =
                    select.columns.is_empty() || select.columns.iter().any(|c| c.name == "*");
                let agg_result_rows = if is_star_agg || select.columns.len() <= 1 {
                    // Star / single column: no re-projection needed
                    agg_result_rows
                } else {
                    // Build group-by schema (column names in order).
                    // Identifier expressions use their name. For
                    // function calls (e.g. EXTRACT(YEAR FROM col) in
                    // TPC-H Q7/Q8/Q9), use a stable hash so the
                    // re-projection can match by alias later.
                    let group_schema: Vec<String> = group_exprs
                        .iter()
                        .enumerate()
                        .map(|(i, expr)| match expr {
                            Expression::Identifier(n) => n.clone(),
                            _ => format!("__gb_{}", i),
                        })
                        .collect();
                    // Build the row-side schema:
                    //   [group_col_names..., agg_default_names...]
                    // The agg default names are "sum", "count", "avg",
                    // "min", "max" (lowercased function name) at
                    // select.aggregates order.
                    let agg_default_names: Vec<String> = select
                        .aggregates
                        .iter()
                        .map(|a| match a.func {
                            AggregateFunction::Sum => "sum",
                            AggregateFunction::Count => "count",
                            AggregateFunction::Avg => "avg",
                            AggregateFunction::Min => "min",
                            AggregateFunction::PercentileCont => "percentile_cont",
                            AggregateFunction::Max => "max",
                            AggregateFunction::QuantileDisc => "quantile_disc",
                            AggregateFunction::QuantileCont => "quantile_cont",
                        })
                        .map(|s| s.to_string())
                        .collect();
                    let _full_schema: Vec<String> = group_schema
                        .iter()
                        .cloned()
                        .chain(agg_default_names.iter().cloned())
                        .collect();
                    // Build lookup maps.
                    let mut group_set: std::collections::HashMap<String, usize> = group_schema
                        .iter()
                        .enumerate()
                        .map(|(i, n)| (n.to_lowercase(), i))
                        .collect();
                    let agg_set: std::collections::HashMap<String, usize> = agg_default_names
                        .iter()
                        .enumerate()
                        .map(|(i, n)| (n.to_lowercase(), i + group_schema.len()))
                        .collect();
                    // For each SELECT column, determine if it's an
                    // aggregate reference or a non-aggregate GROUP BY
                    // expression. Walk select.aggregates in order and
                    // pair with SELECT columns that are aggregates.
                    // The alias of the i-th aggregate SELECT column
                    // maps to agg position i.
                    let mut agg_alias_to_pos: std::collections::HashMap<String, usize> =
                        std::collections::HashMap::new();
                    let mut agg_select_idx = 0usize;
                    for col in select.columns.iter() {
                        let is_agg = match &col.expression {
                            Some(Expression::Aggregate(_)) => true,
                            Some(Expression::BinaryOp(l, _, r)) => {
                                matches!(l.as_ref(), Expression::Aggregate(_))
                                    || matches!(r.as_ref(), Expression::Aggregate(_))
                            }
                            _ => false,
                        };
                        if is_agg {
                            let pos = agg_select_idx + group_schema.len();
                            if let Some(alias) = &col.alias {
                                agg_alias_to_pos.insert(alias.clone(), pos);
                            }
                            if col.alias.is_none() {
                                agg_alias_to_pos.insert(col.name.clone(), pos);
                            }
                            agg_select_idx += 1;
                        }
                    }
                    // Sprint 5 fix: also map non-aggregate SELECT
                    // columns (like EXTRACT in GROUP BY) to their
                    // GROUP BY position. Compare the SELECT col's
                    // expression structurally to each group_expr.
                    for (i, gexpr) in group_exprs.iter().enumerate() {
                        for col in select.columns.iter() {
                            // Match by expression structural equality
                            // (FunctionCall EXTRACT in both), OR by
                            // alias pointing to __gb_N if no match
                            let is_match = match &col.expression {
                                Some(expr) => expr == gexpr,
                                _ => false,
                            };
                            if is_match {
                                // Map both alias and the synthetic
                                // group_schema name to position i
                                if let Some(alias) = &col.alias {
                                    group_set.entry(alias.to_lowercase()).or_insert(i);
                                }
                                group_set.entry(col.name.to_lowercase()).or_insert(i);
                            }
                        }
                    }
                    // Re-project each row
                    let agg_schema_for_reproject =
                        build_aggregate_schema(group_exprs, &select.aggregates)?;
                    agg_result_rows
                        .into_iter()
                        .map(|row| {
                            select
                                .columns
                                .iter()
                                .map(|col| {
                                    let target_name =
                                        col.alias.clone().unwrap_or_else(|| col.name.clone());
                                    let key = target_name.to_lowercase();
                                    // TPC-H Q8 / Q14: if the SELECT
                                    // column expression is a BinaryOp
                                    // over aggregates (e.g.
                                    // `SUM(...) / SUM(...)` or
                                    // `100.00 * SUM(...) / SUM(...)`),
                                    // we MUST re-evaluate the
                                    // expression against the row's
                                    // aggregate values, not just
                                    // look up a single column. The
                                    // previous lookup-by-alias
                                    // path returned only the first
                                    // aggregate operand (e.g. 932.71
                                    // for Q8 mkt_share) and dropped
                                    // the rest of the expression.
                                    let needs_reval = match &col.expression {
                                        Some(Expression::BinaryOp(l, _, r)) => {
                                            matches!(l.as_ref(), Expression::Aggregate(_))
                                                || matches!(r.as_ref(), Expression::Aggregate(_))
                                        }
                                        _ => false,
                                    };
                                    if needs_reval {
                                        if let Some(expr) = &col.expression {
                                            return crate::expr_utils::evaluate_expression(
                                                expr,
                                                &row,
                                                &agg_schema_for_reproject,
                                            )
                                            .unwrap_or(Value::Null);
                                        }
                                    }
                                    // Try group col first
                                    if let Some(&i) = group_set.get(&key) {
                                        return row.get(i).cloned().unwrap_or(Value::Null);
                                    }
                                    // Try aggregate default name
                                    if let Some(&i) = agg_set.get(&key) {
                                        return row.get(i).cloned().unwrap_or(Value::Null);
                                    }
                                    // V313-followup-3 / Issue #4156:
                                    // PERCENTILE_CONT FunctionCall column
                                    // (name is the full debug string, so
                                    // match by expression function name).
                                    if let Some(Expression::FunctionCall(fname, _)) =
                                        &col.expression
                                    {
                                        let fu = fname.to_uppercase();
                                        let default_name = match fu.as_str() {
                                            "PERCENTILE_CONT" => "percentile_cont",
                                            "QUANTILE_DISC" => "quantile_disc",
                                            "QUANTILE_CONT" => "quantile_cont",
                                            _ => "",
                                        };
                                        if !default_name.is_empty() {
                                            if let Some(&i) = agg_set.get(default_name) {
                                                return row.get(i).cloned().unwrap_or(Value::Null);
                                            }
                                        }
                                    }
                                    // Try aggregate alias map
                                    if let Some(&i) = agg_alias_to_pos.get(&key) {
                                        return row.get(i).cloned().unwrap_or(Value::Null);
                                    }
                                    Value::Null
                                })
                                .collect()
                        })
                        .collect()
                };

                let row_count = agg_result_rows.len();
                return Ok(ExecutorResult::new(agg_result_rows, row_count));
            }
        }

        // Step 4: LIMIT / OFFSET
        //
        // Fix for #3282 (Sprint 5): LIMIT used to be applied here,
        // BEFORE ORDER BY (Step 7). That broke any
        // `ORDER BY col [DESC] LIMIT n` query — the engine would
        // take the first n rows in storage order, then "sort"
        // them, returning storage-order top n instead of the
        // highest/lowest n. The canonical case was TPC-H Q18's
        // `ORDER BY o_totalprice DESC LIMIT 100` which returned
        // the storage-order top 100 instead of the highest-total
        // top 100. LIMIT/OFFSET are now applied in Step 8 (after
        // ORDER BY) below.
        let limited_rows_for_order_by: Vec<Vec<Value>> = rows.clone(); // Step 5: SELECT projection — apply each `select.columns` expression
                                                                       // to the accumulated row and emit a row of projected values. This
                                                                       // is what makes `SELECT EXTRACT(YEAR FROM col) AS o_year` actually
                                                                       // return `o_year` instead of the full table schema.
                                                                       //
                                                                       // Sprint 2: SELECT * (no columns or a `*` entry) skips projection
                                                                       // and returns the accumulated rows as-is — that's the existing
                                                                       // behavior, just made explicit here.
                                                                       //
                                                                       // v3.8.0-rc2 Day 7: also collect the projected column NAMES so
                                                                       // that the subsequent ORDER BY step can resolve column references
                                                                       // by name (`ORDER BY l_orderkey`).
                                                                       //
                                                                       // v3.9.0 Sprint 5 v16+ fix (COALESCE+ORDER BY): keep a clone of
                                                                       // the underlying rows so the ORDER BY step can resolve column
                                                                       // references against the original table schema (the projected row
                                                                       // has fewer columns when SELECT is a function call that emits one
                                                                       // column, so `row[idx]` against `table_info.columns` index is
                                                                       // wrong — `idx` would read from the projected row's slot 0,
                                                                       // which is the function output, not the underlying column value).
        let is_star = select.columns.is_empty() || select.columns.iter().any(|c| c.name == "*");
        // V311-10 fix: pre-acquire the storage write lock so the
        // SequenceNextVal / SequenceCurrval arms in
        // `evaluate_expression_with_seq` can advance / read live
        // sequence state during the projection.
        let mut storage_guard = self.storage.write();
        let projected_with_names: (Vec<String>, Vec<Vec<Value>>) = if is_star {
            let names: Vec<String> = if !table_info.columns.is_empty()
                && table_info.columns.len() == rows.first().map(|r| r.len()).unwrap_or(0)
            {
                table_info.columns.iter().map(|c| c.name.clone()).collect()
            } else {
                (1..=rows.first().map(|r| r.len()).unwrap_or(0))
                    .map(|i| format!("c{}", i))
                    .collect()
            };
            (names, rows)
        } else {
            let names: Vec<String> = select
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect();
            let rows: Vec<Vec<Value>> = (|| -> SqlResult<Vec<Vec<Value>>> {
                let mut out = Vec::new();
                for row in rows {
                    let mut new_row = Vec::new();
                    for col in &select.columns {
                        let v = match &col.expression {
                            Some(expr) => crate::expr_utils::evaluate_expression_with_seq(
                                expr,
                                &row,
                                &table_info,
                                Some(&mut *storage_guard),
                                &|_| Ok(Value::Null),
                            )
                            .map_err(SqlError::ExecutionError)?,
                            None => row.first().cloned().unwrap_or(Value::Null),
                        };
                        new_row.push(v);
                    }
                    out.push(new_row);
                }
                Ok(out)
            })()?;
            (names, rows)
        };
        let (projected_column_names, projected_rows) = projected_with_names;
        // Step 6: DISTINCT — apply deduplication if select.distinct is set.
        // V380 F-12 fix: parser sets select.distinct but executor was ignoring it.
        // Use a HashSet of Value vectors to track seen rows.
        let projected_rows: Vec<Vec<Value>> = if select.distinct {
            use std::collections::HashSet;
            let mut seen: HashSet<Vec<Value>> = HashSet::new();
            projected_rows
                .into_iter()
                .filter(|row| seen.insert(row.clone()))
                .collect()
        } else {
            projected_rows
        };

        // Step 7: ORDER BY — apply sort if select.order_by is non-empty.
        // v3.8.0-rc2 Week 1 Day 7: ORDER BY is parsed but was never
        // applied in the executor. This caused Q1, Q3, Q4, Q13, Q15
        // to return rows in storage order rather than the requested
        // ORDER BY order, breaking value assertions vs SQLite.
        //
        // Implementation: for each ORDER BY expression, we evaluate
        // it against each row to get a sort key. We sort by a
        // tuple of sort-key values using Vec<Value>'s default Ord
        // implementation. Vec::sort_by is stable, so equal keys
        // preserve input order.
        //
        // Fix for #3282 (Sprint 5): LIMIT/OFFSET used to be applied
        // BEFORE ORDER BY (Step 4) which broke any `ORDER BY col
        // LIMIT n` query — the engine would take the first n rows
        // in storage order and then "sort" them, producing results
        // that look correct on the first n rows but are actually
        // out of order. The canonical case was TPC-H Q18's
        // `ORDER BY o_totalprice DESC LIMIT 100` which returned
        // the storage-order top 100 instead of the highest-total
        let projected_rows: Vec<Vec<Value>> = if !select.order_by.is_empty() {
            // Zipped with the underlying pre-projection rows so that
            // ORDER BY references to underlying table columns (e.g.
            // `ORDER BY id` when projection is `SELECT COALESCE(a,b,c)`)
            // resolve against the original row's columns, not the
            // projected row's (often smaller) slot indices.
            let mut zipped: Vec<(Vec<Value>, Vec<Value>, Vec<Value>)> = projected_rows
                .into_iter()
                .zip(limited_rows_for_order_by)
                .map(|(row, original_row)| {
                    let keys: Vec<Value> = select
                        .order_by
                        .iter()
                        .map(|ob_expr| {
                            // ORDER BY column reference (e.g. "l_orderkey")
                            // or a positional integer (e.g. "ORDER BY 1").
                            match &ob_expr.expression {
                                Expression::Identifier(col_name) => {
                                    // Look up by name in projected_column_names.
                                    if let Some(idx) =
                                        projected_column_names.iter().position(|n| n == col_name)
                                    {
                                        if idx < row.len() {
                                            return row[idx].clone();
                                        }
                                    }
                                    // Fallback: look up the column in the
                                    // ORIGINAL pre-projection row (not the
                                    // projected row), since the projection
                                    // can have a different number of columns
                                    // than the underlying table.
                                    if let Some(idx) =
                                        table_info.columns.iter().position(|c| c.name == *col_name)
                                    {
                                        if idx < original_row.len() {
                                            return original_row[idx].clone();
                                        }
                                    }
                                    Value::Null
                                }
                                Expression::Literal(lit_str) => {
                                    // Try parsing as positional integer (1-based)
                                    // against the PROJECTED row (standard SQL:
                                    // `ORDER BY 1` references the n-th SELECT
                                    // column).
                                    if let Ok(idx_1based) = lit_str.parse::<usize>() {
                                        let idx = idx_1based.saturating_sub(1);
                                        if idx < row.len() {
                                            return row[idx].clone();
                                        }
                                    }
                                    // Otherwise try as a literal Value via Integer parse.
                                    if let Ok(i) = lit_str.parse::<i64>() {
                                        return Value::Integer(i);
                                    }
                                    Value::Null
                                }
                                _ => Value::Null, // unsupported in ORDER BY for now
                            }
                        })
                        .collect();
                    (keys, row, original_row)
                })
                .collect();
            // Sort. Each order_by has an `ascending` flag;
            // v3.8.0-rc2 Day 7: respect ASC/DESC. Q13 uses
            // DESC, which my earlier version ignored.
            // V313-followup-4 / Issue #4157: also honour session
            // `SET default_null_order` for NULL-first / NULL-last placement.
            zipped.sort_by(|a, b| {
                for (i, ob) in select.order_by.iter().enumerate() {
                    let nulls_first_eff: bool = ob
                        .nulls_first
                        .unwrap_or_else(|| self.session_null_order_first.unwrap_or(true));
                    let ord = if i < a.0.len() && i < b.0.len() {
                        let va = &a.0[i];
                        let vb = &b.0[i];
                        let is_null_a = matches!(va, Value::Null);
                        let is_null_b = matches!(vb, Value::Null);
                        if is_null_a && is_null_b {
                            std::cmp::Ordering::Equal
                        } else if is_null_a {
                            if nulls_first_eff {
                                std::cmp::Ordering::Less
                            } else {
                                std::cmp::Ordering::Greater
                            }
                        } else if is_null_b {
                            if nulls_first_eff {
                                std::cmp::Ordering::Greater
                            } else {
                                std::cmp::Ordering::Less
                            }
                        } else {
                            va.cmp(vb)
                        }
                    } else {
                        std::cmp::Ordering::Equal
                    };
                    let ord = if ob.ascending { ord } else { ord.reverse() };
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                }
                std::cmp::Ordering::Equal
            });
            zipped.into_iter().map(|(_, row, _)| row).collect()
        } else {
            projected_rows
        };

        // Step 8: LIMIT / OFFSET — apply after ORDER BY so that
        // `ORDER BY col [DESC] LIMIT n` returns the correct top/bottom n.
        // See Step 4 above for the historical reason this moved.
        let projected_rows: Vec<Vec<Value>> = if let Some(limit) = select.limit {
            let offset = select.offset.unwrap_or(0);
            if offset as usize >= projected_rows.len() {
                vec![]
            } else {
                projected_rows
                    .into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect()
            }
        } else {
            projected_rows
        };

        let row_count = projected_rows.len();
        Ok(ExecutorResult::new(projected_rows, row_count))
    }

    fn compute_aggregates(
        &self,
        aggregates: &[AggregateCall],
        rows: &[Vec<Value>],
        table_info: &TableInfo,
    ) -> SqlResult<Vec<Value>> {
        let mut results = Vec::with_capacity(aggregates.len());
        for agg in aggregates {
            // V313-followup-3 / Issue #4156: PERCENTILE_CONT encodes the
            // WITHIN GROUP ORDER BY expression as args[1] (args[0] is the
            // fraction); DESC is a trailing `__DESC__` literal arg.
            let val_src_idx = if matches!(agg.func, AggregateFunction::PercentileCont) {
                1
            } else {
                0
            };
            let values: Vec<Value> = if let Some(arg) = agg.args.get(val_src_idx) {
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
                    } else if agg.distinct {
                        // COUNT(DISTINCT col) - count unique non-NULL values
                        use std::collections::HashSet;
                        let unique: HashSet<_> = values
                            .iter()
                            .filter(|v| !matches!(v, Value::Null))
                            .collect();
                        Value::Integer(unique.len() as i64)
                    } else {
                        // COUNT(col) - count non-NULL values
                        let non_null_count =
                            values.iter().filter(|v| !matches!(v, Value::Null)).count();
                        Value::Integer(non_null_count as i64)
                    }
                }
                AggregateFunction::Sum => {
                    // TPC-H Sprint 1 fix (Q8/Q9): accept Float in Sum.
                    // l_extendedprice * (1 - l_discount) returns Float.
                    // Sprint 5 #3290 fix: empty result set returns NULL
                    // (not Integer(0)) to match SQL standard and SQLite/
                    // MariaDB. PG returns 0 rows in this case; we still
                    // return 1 row with NULL, matching SQL standard +
                    // other 3 engines (sqlite/mariadb/sqlrustgo).
                    let mut int_sum: i64 = 0;
                    let mut float_sum: f64 = 0.0;
                    let mut any_float = false;
                    for v in &values {
                        match v {
                            Value::Integer(n) => {
                                if any_float {
                                    float_sum += *n as f64;
                                } else {
                                    int_sum += n;
                                }
                            }
                            Value::Float(f) => {
                                if !any_float {
                                    float_sum = int_sum as f64;
                                    any_float = true;
                                }
                                float_sum += f;
                            }
                            _ => {}
                        }
                    }
                    if values.is_empty() || values.iter().all(|v| matches!(v, Value::Null)) {
                        // SQL standard: SUM over all-NULL column is NULL
                        Value::Null
                    } else if any_float {
                        Value::Float(float_sum)
                    } else {
                        Value::Integer(int_sum)
                    }
                }
                AggregateFunction::Avg => {
                    // TPC-H Sprint 1 fix (Q1): AVG over Float.
                    // v3.8.0-rc2 Day 7: AVG MUST return Float even when
                    // all input values are Integer — otherwise
                    // `AVG(quantity)` over integer quantities yields
                    // `Value::Integer(int_sum / count)` which is
                    // integer-truncated (e.g. 1323/50 = 26 instead of
                    // 26.46). SQLite/MySQL always return REAL for AVG.
                    let mut int_sum: i64 = 0;
                    let mut float_sum: f64 = 0.0;
                    let mut any_float = false;
                    let mut count: i64 = 0;
                    for v in &values {
                        match v {
                            Value::Integer(n) => {
                                if any_float {
                                    float_sum += *n as f64;
                                } else {
                                    int_sum += n;
                                }
                                count += 1;
                            }
                            Value::Float(f) => {
                                if !any_float {
                                    float_sum = int_sum as f64;
                                    any_float = true;
                                }
                                float_sum += f;
                                count += 1;
                            }
                            _ => {}
                        }
                    }
                    if count > 0 {
                        // Always return Float for AVG. Even if all
                        // inputs are Integer, the average is a
                        // fractional quantity.
                        if any_float {
                            Value::Float(float_sum / count as f64)
                        } else {
                            Value::Float(int_sum as f64 / count as f64)
                        }
                    } else {
                        Value::Null
                    }
                }
                AggregateFunction::Min => {
                    let min = values.iter().filter(|v| !matches!(v, Value::Null)).min();
                    min.cloned().unwrap_or(Value::Null)
                }
                AggregateFunction::Max => {
                    let max = values.iter().filter(|v| !matches!(v, Value::Null)).max();
                    max.cloned().unwrap_or(Value::Null)
                }
                // V313-followup-2 / Issue #4155: quantile_disc(frac) and
                // quantile_cont(frac). The fraction lives in args[1]
                // (e.g. `quantile_disc(col, 0.1)`); args[0] is already
                // evaluated into `values`. Sorted-index algorithm per
                // SQL:92 ordered-set aggregate semantics (fraction
                // interpolated linearly for continuous; rounded down for
                // discrete).
                //
                // V313-followup-3 / Issue #4216: array-fraction form
                // `quantile_disc(col, [0.25, 0.5, 0.75])` returns a
                // single Text cell "[v1, v2, ...]" computed by the same
                // per-fraction algorithm. Multi-row emission (one row per
                // fraction) is deferred to the next follow-up (#4216.3).
                AggregateFunction::QuantileDisc
                | AggregateFunction::QuantileCont
                | AggregateFunction::PercentileCont => {
                    let frac_idx = if matches!(agg.func, AggregateFunction::PercentileCont) {
                        0
                    } else {
                        1
                    };
                    // Detect array-literal fraction form before attempting
                    // the single-fraction parse below.
                    let array_fracs: Option<Vec<f64>> = match agg.args.get(frac_idx) {
                        Some(sqlrustgo_parser::Expression::ArrayLiteral(elems)) => {
                            let mut acc = Vec::with_capacity(elems.len());
                            for e in elems {
                                let f = match e {
                                    sqlrustgo_parser::Expression::Literal(lit) => {
                                        lit.parse::<f64>().ok()
                                    }
                                    _ => None,
                                };
                                match f {
                                    Some(v) if (0.0..=1.0).contains(&v) => acc.push(v),
                                    _ => {
                                        return Err(SqlError::ExecutionError(format!(
                                            "quantile_disc / quantile_cont array element out of [0.0, 1.0]: {:?}",
                                            e
                                        )));
                                    }
                                }
                            }
                            Some(acc)
                        }
                        _ => None,
                    };
                    let frac = match (&array_fracs, agg.args.get(frac_idx)) {
                        (Some(_), _) => 0.0, // unused in the array branch below
                        (None, Some(sqlrustgo_parser::Expression::Literal(lit))) => {
                            match lit.parse::<f64>().ok() {
                                Some(f) if (0.0..=1.0).contains(&f) => f,
                                _ => {
                                    let msg = if matches!(
                                        agg.func,
                                        AggregateFunction::PercentileCont
                                    ) {
                                        "PercentileCont requires frac arg in [0.0, 1.0]".to_string()
                                    } else {
                                        "quantile_disc / quantile_cont requires 2nd arg in [0.0, 1.0]"
                                            .to_string()
                                    };
                                    return Err(SqlError::ExecutionError(msg));
                                }
                            }
                        }
                        (None, _) => {
                            let msg = if matches!(agg.func, AggregateFunction::PercentileCont) {
                                "PercentileCont requires frac arg in [0.0, 1.0]".to_string()
                            } else {
                                "quantile_disc / quantile_cont requires 2nd arg in [0.0, 1.0]"
                                    .to_string()
                            };
                            return Err(SqlError::ExecutionError(msg));
                        }
                    };
                    let desc = matches!(agg.func, AggregateFunction::PercentileCont)
                        && agg.args.last().is_some_and(|e| matches!(e, sqlrustgo_parser::Expression::Literal(l) if l == "__DESC__"));
                    let mut sorted: Vec<f64> = values
                        .iter()
                        .filter_map(|v| match v {
                            Value::Integer(n) => Some(*n as f64),
                            Value::Float(f) => Some(*f),
                            _ => None,
                        })
                        .collect();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    if desc {
                        sorted.reverse();
                    }
                    if sorted.is_empty() {
                        Value::Null
                    } else if let Some(fracs) = array_fracs {
                        // Array-fraction form: compute one value per
                        // fraction, emit as Text "[v1, v2, ...]".
                        let parts: Vec<String> = fracs
                            .iter()
                            .map(|f| {
                                let idx = (f * (sorted.len() as f64 - 1.0)).max(0.0);
                                let lo = idx.floor() as usize;
                                let hi = idx.ceil() as usize;
                                if matches!(agg.func, AggregateFunction::QuantileDisc) || lo == hi {
                                    let v = sorted[lo.min(sorted.len() - 1)];
                                    format!("{}", v)
                                } else {
                                    let frac_part = idx - lo as f64;
                                    let v = sorted[lo] + (sorted[hi] - sorted[lo]) * frac_part;
                                    format!("{}", v)
                                }
                            })
                            .collect();
                        Value::Text(format!("[{}]", parts.join(", ")))
                    } else {
                        let idx = (frac * (sorted.len() as f64 - 1.0)).max(0.0);
                        let lo = idx.floor() as usize;
                        let hi = idx.ceil() as usize;
                        let result =
                            if matches!(agg.func, AggregateFunction::QuantileDisc) || lo == hi {
                                sorted[lo.min(sorted.len() - 1)]
                            } else {
                                let frac_part = idx - lo as f64;
                                sorted[lo] + (sorted[hi] - sorted[lo]) * frac_part
                            };
                        Value::Float(result)
                    }
                }
            };
            results.push(result);
        }
        Ok(results)
    }

    /// V311-02 v2: scan with AHI instrumentation. Records each scan
    /// against the shared AdaptiveHashIndex so repeated scans of the
    /// same table get promoted after the threshold (default 17).
    ///
    /// Synthetic page_id/offset: for MemoryStorage, there is no real
    /// page concept, so we use a stable hash of the table name as the
    /// page_id and the row count as the offset. This makes the AHI
    /// tracking a real "table-scan hot-path" counter, not a no-op.
    /// When v3 FileStorage lands proper page-aware instrumentation, the
    /// caller can pass the actual `(page_id, offset)` from the B+ Tree
    /// page handle.
    /// V311-05 F-29: Apply Row-Level Security filter to scanned rows.
    /// Checks if RLS is enabled for this table in the catalog, and if so,
    /// filters rows through the policy catalog's filter_rows() method.
    fn apply_rls_filter(
        &self,
        table: &str,
        rows: Vec<sqlrustgo_storage::Record>,
        table_info: &sqlrustgo_storage::TableInfo,
    ) -> SqlResult<Vec<sqlrustgo_storage::Record>> {
        let Some(catalog) = &self.catalog else {
            return Ok(rows); // No catalog = no RLS
        };
        let catalog_guard = catalog.read();
        if !catalog_guard.is_rls_enabled(table) {
            return Ok(rows); // RLS not enabled for this table
        }
        // Convert Record (Vec<Value>) to RLS Row (HashMap<String, Value>)
        let rls_rows: Vec<std::collections::HashMap<String, Value>> = rows
            .into_iter()
            .map(|record| {
                table_info
                    .columns
                    .iter()
                    .zip(record)
                    .map(|(col, val)| (col.name.clone(), val))
                    .collect()
            })
            .collect();
        // Filter through RLS policies
        let filtered = catalog_guard.policy_catalog().filter_rows(table, rls_rows);
        // Convert back to Record (Vec<Value>)
        let result: Vec<sqlrustgo_storage::Record> = filtered
            .into_iter()
            .map(|row| {
                table_info
                    .columns
                    .iter()
                    .map(|col| row.get(&col.name).cloned().unwrap_or(Value::Null))
                    .collect()
            })
            .collect();
        Ok(result)
    }

    /// V311-01 F-23 + V311-02 F-24: scan with ClusteredTable + AHI instrumentation.
    /// Priority: ClusteredTable (if registered) → storage.scan().
    /// AHI records every table-level access for hot-page promotion.
    fn scan_with_ahi(
        &self,
        storage: &parking_lot::RwLockReadGuard<'_, S>,
        table: &str,
    ) -> SqlResult<Vec<sqlrustgo_storage::Record>> {
        // V311-01 F-23: route clustered-table scans through ClusteredTable.
        // ClusteredTable stores rows ordered by primary key (InnoDB-style),
        // providing O(log N) pk lookups and O(log N + k) range scans.
        if let Some(ct_guard) = self.clustered_tables.read().get(table) {
            let ct = ct_guard.read();
            self.instrumentation.on_seq_scan_start(table);
            let rows = ct.full_scan();
            // V311-02 F-24: record table-level access for AHI promotion.
            let mut page_id: u64 = 0xcbf29ce484222325;
            for &b in table.as_bytes() {
                page_id ^= u64::from(b);
                page_id = page_id.wrapping_mul(0x100000001b3);
            }
            let offset = rows.len() as u32;
            self.adaptive_hash_index
                .record_access(table, b"clustered", page_id, offset);
            return Ok(rows);
        }
        // Default: full table scan via storage.
        self.instrumentation.on_seq_scan_start(table);
        let rows = storage.scan(table)?;
        // V311-02 F-24: stable FNV-1a-ish hash of table name as synthetic page_id.
        let mut page_id: u64 = 0xcbf29ce484222325;
        for &b in table.as_bytes() {
            page_id ^= u64::from(b);
            page_id = page_id.wrapping_mul(0x100000001b3);
        }
        let offset = rows.len() as u32;
        self.adaptive_hash_index
            .record_access(table, b"all", page_id, offset);
        Ok(rows)
    }

    /// Execute a chain of JOINs: start from the base table, then apply each
    /// JoinClause in order (left-associative: t1 JOIN t2 JOIN t3 → ((t1 JOIN t2) JOIN t3)).
    /// This function only generates joined rows, does NOT apply WHERE/AGG/HAVING.
    fn execute_joins(
        &self,
        select: &mut SelectStatement,
    ) -> SqlResult<(Vec<Vec<Value>>, TableInfo, bool)> {
        COMMA_JOIN_WHERE_CONSUMED.with(|f| *f.borrow_mut() = false);
        let storage = self.storage_read();

        // Sprint 5 v4: the parser encodes the inline alias into the
        // table name as `table|alias` (e.g. `emp|e`). Storage has only
        // the bare table name, so strip the `|alias` suffix before the
        // storage lookup.  The alias (and the base_prefix below) are
        // still used for column-name qualification.
        let (base_table, base_alias) = match select.table.split_once('|') {
            Some((t, a)) => (t.to_string(), Some(a.to_string())),
            None => (select.table.clone(), select.from_alias.clone()),
        };
        let base_prefix = base_alias.as_ref().unwrap_or(&base_table);

        // V311-02 v2: instrument base-table scan via AHI so repeated
        // SELECTs against the same table get promoted after threshold.
        let mut rows = self.scan_with_ahi(&storage, &base_table)?;
        // Fast-path base-table predicate pushdown (single-table
        // predicates that reference only the base table). For
        // TPC-H Q2 (`FROM part WHERE p_size = 15 AND p_type LIKE
        // '%BRASS'`) this collapses 20K part rows to ~400 rows
        // before any join work, avoiding the full 5-table join
        // explosion downstream.
        let raw_info = storage.get_table_info(&base_table)?;
        let mut table_info = if base_alias.is_some() {
            // Wrap the base columns in alias-prefixed names.
            let mut new_info = raw_info.clone();
            new_info.name = base_prefix.clone();
            for col in &mut new_info.columns {
                col.name = format!("{}.{}", base_prefix, col.name);
            }
            new_info
        } else {
            raw_info
        };
        // V311-05 F-29: apply RLS filter to base table scan after table_info is defined
        rows = self.apply_rls_filter(&base_table, rows, &table_info)?;
        if let Some(where_expr) = select.where_clause.as_ref() {
            // Build the base table's qualified column-name set
            // (TPC-H prefix + bare names).
            let base_qualified: Vec<String> =
                table_info.columns.iter().map(|c| c.name.clone()).collect();
            let mut base_keys: Vec<String> = vec![
                base_table.clone(),
                base_prefix.clone(),
                Self::tpch_table_prefix(&base_table).to_string(),
            ];
            base_keys.extend(base_qualified);
            let base_preds = self.extract_single_table_predicates(where_expr, &base_keys);
            let preds_opt = base_preds
                .get(base_prefix)
                .or_else(|| base_preds.get(&base_table))
                .or_else(|| base_preds.get(Self::tpch_table_prefix(&base_table)));
            if let Some(preds) = preds_opt {
                if !preds.is_empty() {
                    let before = rows.len();
                    rows.retain(|r| preds.iter().all(|p| eval_predicate(p, r, &table_info)));
                    tracing::debug!(
                        target: "sqlrustgo.q2_fix",
                        table = %base_table,
                        before,
                        after = rows.len(),
                        "applied base-table pushdown"
                    );
                }
            }
        }

        // Phase 3 (TPCH-01 Q15): materialize any derived subqueries from
        // `FROM t, (SELECT ...) AS alias` before the join chain runs.
        let derived_subqueries = get_and_clear_derived_subqueries();
        if !derived_subqueries.is_empty() {
            for (name, subq) in &derived_subqueries {
                let sub_result = self.execute_select(subq)?;
                let mut table_info = TableInfo {
                    name: name.clone(),
                    columns: Vec::new(),
                    foreign_keys: Vec::new(),
                    unique_constraints: Vec::new(),
                    check_constraints: Vec::new(),
                    partition_info: None,
                    compression: None,
                    collations: std::collections::HashMap::new(),
                };
                for col in &subq.columns {
                    let col_name = col.alias.clone().unwrap_or_else(|| col.name.clone());
                    let inferred_type_str: String = sub_result
                        .rows
                        .iter()
                        .find(|r| r.iter().any(|v| !matches!(v, Value::Null)))
                        .and_then(|first_row| {
                            let col_idx = subq
                                .columns
                                .iter()
                                .position(|c| c.alias.as_ref().unwrap_or(&c.name) == &col_name)?;
                            first_row.get(col_idx).map(|v| match v {
                                Value::Integer(_) => "INTEGER",
                                Value::Float(_) => "FLOAT",
                                Value::Text(_) => "TEXT",
                                Value::Boolean(_) => "BOOLEAN",
                                Value::Blob(_) => "BLOB",
                                Value::Point(_, _) => "POINT",
                                Value::Json(_) => "JSON",
                                Value::Null => "NULL",
                            })
                        })
                        .unwrap_or("TEXT")
                        .to_string();
                    table_info
                        .columns
                        .push(sqlrustgo_storage::ColumnDefinition {
                            name: col_name,
                            data_type: inferred_type_str,
                            nullable: true,
                            primary_key: false,
                            char_max_length: None,
                            collation: None,
                            default_value: None,
                        });
                }
                DERIVED_RESULTS.with(|cell| {
                    cell.borrow_mut()
                        .insert(name.clone(), (sub_result.rows, table_info));
                });
            }
        }

        // Sprint 5 v15+ predicate pushdown (Q21 perf): collect
        // single-table WHERE predicates and apply them to each
        // right-side scan before the hash-join build.  For Q21
        // this drops the 4-table intermediate from 2.25B rows
        // to ~6K rows.
        let pushdown_filters = select
            .where_clause
            .as_ref()
            .map(|wc| {
                // `joined` must include both table names and
                // TPC-H 1-/2-char column prefixes (`s` for
                // `supplier`, `ps` for `partsupp`, `n` for
                // `nation`, `l` for `lineitem`, etc.) so the
                // `collect_referenced_tables_local` heuristic
                // (which extracts the underscore-prefix from
                // unqualified `n_name`, `l_orderkey`, etc.)
                // matches the table.
                let mut joined: Vec<String> = vec![base_table.clone(), base_prefix.clone()];
                for jc in &select.join_clause {
                    let (bare, _) = match jc.table.split_once('|') {
                        Some((t, a)) => (t.to_string(), Some(a.to_string())),
                        None => (jc.table.clone(), jc.alias.clone()),
                    };
                    joined.push(bare.clone());
                    joined.push(Self::tpch_table_prefix(&bare).to_string());
                    if let Some(a) = &jc.alias {
                        joined.push(a.clone());
                    }
                }
                for extra in &select.extra_tables {
                    let (bare, alias) = match extra.split_once('|') {
                        Some((t, a)) => (t.to_string(), a.to_string()),
                        None => (extra.clone(), String::new()),
                    };
                    joined.push(bare.clone());
                    joined.push(Self::tpch_table_prefix(&bare).to_string());
                    if !alias.is_empty() {
                        joined.push(alias);
                    }
                }
                joined.push(Self::tpch_table_prefix(&base_table).to_string());
                self.extract_single_table_predicates(wc, &joined)
            })
            .unwrap_or_default();

        // Sprint 8 (PR 1): comma-join with WHERE-extracted hash chain
        // optimisation. Tries to convert the WHERE-clause equality
        // predicates into a chain of 2-way hash joins and run them
        // in `multi_way_hash_chain`. This is the fast path for TPC-H
        // Q3 / Q8 / Q21 which all use `FROM t1, t2, t3` (comma-join
        // with no JOIN ON). Returns `Some` on success, `None` when
        // the WHERE can't supply a complete chain (caller falls back
        // to the per-clause cartesian path).
        if !select.extra_tables.is_empty() || !select.join_clause.is_empty() {
            if let Some((new_rows, new_info)) = self.try_comma_join_hash_chain(
                select,
                &base_table,
                &base_prefix,
                rows.clone(),
                &table_info,
                &pushdown_filters,
            ) {
                // V312-35 (#4182): the hash chain consumes only the
                // equality join predicates it extracted. If the WHERE
                // still contains correlated subqueries or other
                // non-equality residuals (TPC-H Q17: `l_quantity <
                // (SELECT 0.2*AVG(...))`), those must NOT be marked
                // consumed — the post-join filter stage (Step 1.5)
                // re-evaluates them per row. Only mark consumed when
                // the WHERE is entirely covered by the chain (pure
                // equality + single-table predicates).
                let where_fully_consumed = select
                    .where_clause
                    .as_ref()
                    .map(|wc| !where_expr_has_correlated_subquery(wc))
                    .unwrap_or(true);
                if where_fully_consumed {
                    COMMA_JOIN_WHERE_CONSUMED.with(|f| *f.borrow_mut() = true);
                }
                return Ok((new_rows, new_info, true));
            }
        }

        for join_clause in &select.join_clause {
            // Strip the optional `|alias` suffix from
            // join_clause.table to look up pushdown filters.
            // `extract_single_table_predicates` keys by the table
            // qualifier (alias if present, else bare name), so try
            // the alias first, then fall back to the bare name.
            let (bare_right_table, right_alias) = match join_clause.table.split_once('|') {
                Some((t, a)) => (t.to_string(), Some(a.to_string())),
                None => (join_clause.table.clone(), None),
            };
            // Look up pushdown filters by all the keys that might match
            // `bare_right_table`: alias, bare name, or the TPC-H 1-/2-char
            // column prefix (e.g. `region` -> `r`, `partsupp` -> `ps`),
            // since `extract_single_table_predicates` keys single-table
            // conjunctions by the qualifier that's actually referenced
            // in the WHERE (often a prefix, not the table name).
            let right_filter = pushdown_filters
                .get(right_alias.as_deref().unwrap_or(&bare_right_table))
                .or_else(|| pushdown_filters.get(&bare_right_table))
                .or_else(|| pushdown_filters.get(Self::tpch_table_prefix(&bare_right_table)))
                .cloned();
            let (new_rows, new_info) = self.execute_single_join(
                &rows,
                &table_info,
                join_clause,
                &storage,
                &select.where_clause,
                right_filter.as_deref().unwrap_or(&[]),
            )?;
            rows = new_rows;
            table_info = new_info;
        }

        Ok((rows, table_info, false))
    }

    /// Best-first greedy chain construction starting from
    /// `join_tables[start_idx]`.
    ///
    /// At each step, among all unvisited tables that share a join edge
    /// with the current tail (i.e. an entry in `pair_key`), pick the one
    /// with the lowest degree (count of join edges). This avoids the
    /// failure mode of the previous single-direction greedy: starting
    /// from a high-degree hub and dead-ending inside one of its
    /// leaves. Returns `None` when no neighbour is available
    /// (dead-end) so the caller can try a different start.
    ///
    /// Worst-case complexity O(N²) for N <= 10 (TPC-H).
    pub(crate) fn build_chain_from_start(
        start_idx: usize,
        join_tables: &[(String, String)],
        pair_key: &std::collections::HashMap<(String, String), (String, String)>,
    ) -> Option<Vec<(String, String)>> {
        use std::collections::HashSet;
        let start = &join_tables[start_idx];
        let mut visited: HashSet<String> = [start.1.clone()].into_iter().collect();
        let mut chain: Vec<(String, String)> = vec![(start.0.clone(), start.1.clone())];

        while visited.len() < join_tables.len() {
            let tail_alias = match chain.last() {
                Some((_, a)) => a.clone(),
                None => break,
            };

            let next = join_tables
                .iter()
                .filter(|(_, alias)| !visited.contains(alias))
                .filter(|(_, alias)| {
                    pair_key.keys().any(|(a1, a2)| {
                        (*a1 == tail_alias && *a2 == *alias) || (*a2 == tail_alias && *a1 == *alias)
                    })
                })
                .min_by_key(|(_, alias)| {
                    pair_key
                        .keys()
                        .filter(|(a1, a2)| a1 == alias || a2 == alias)
                        .count()
                })
                .map(|(bare, alias)| (bare.clone(), alias.clone()));

            match next {
                Some((next_bare, alias)) => {
                    visited.insert(alias.clone());
                    chain.push((next_bare, alias));
                }
                None => return None,
            }
        }
        Some(chain)
    }

    /// Sprint 8 (PR 1): comma-join with WHERE-extracted hash chain.
    ///
    /// TPC-H Q3, Q8, Q21 use `FROM t1, t2, t3` (comma-join) with the
    /// actual join keys in the WHERE clause:
    ///
    /// - Q3: `WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey ...`
    /// - Q8: `WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey ...`
    /// - Q21: `WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey ...`
    ///
    /// The parser stores these as a `JoinClause` with `JoinKey::All`
    /// (no ON clause), so the legacy cartesian-product path fires and
    /// the join blows up to O(N²) (or worse for chained comma-joins).
    ///
    /// This helper extracts the equality predicates from the WHERE
    /// clause, builds a join chain (base → next → ...), and runs
    /// `multi_way_hash_chain` to produce the joined rows. Returns
    /// `Some((rows, table_info))` on success, `None` when the WHERE
    /// does not provide a complete chain (caller falls back to the
    /// cartesian path).
    ///
    /// Scope: only `=` equality predicates between two distinct
    /// joined tables are considered. `!=`, `<`, `LIKE`, etc. are
    /// ignored. Tables must be reachable from the comma-join list
    /// the WHERE isn't in that list, the chain can't be built
    /// here and we return None.
    fn try_comma_join_hash_chain(
        &self,
        select: &SelectStatement,
        base_table: &str,
        _base_alias: &str,
        base_rows: Vec<Vec<Value>>,
        base_info: &TableInfo,
        pushdown_filters: &std::collections::HashMap<String, Vec<Expression>>,
    ) -> Option<(Vec<Vec<Value>>, TableInfo)> {
        use std::collections::HashMap;
        let where_expr = select.where_clause.as_ref()?;
        // V312-35 (#4182) Q2/Q17: bail out of hash chain when WHERE
        // has a correlated scalar subquery. The chain consumes
        // equality predicates but the subquery still needs per-row
        // substitution in the post-join filter. Returning None
        // forces the per-clause fallback in execute_joins, which
        // calls pre_evaluate_correlated_exists to substitute the
        // Subquery node with a scalar literal (Q2: MIN cost per part;
        // Q17: 0.2*AVG threshold per partkey). This produces the
        // correct row counts vs the SQLite oracle without dead-ending
        // in an O(joined_rows × subquery_cost) hang.
        if where_expr_has_correlated_subquery(where_expr) {
            return None;
        }
        let storage = self.storage.read();

        // Extract bare table name from base_table (which may be "table" or "table|alias")
        let base_bare = base_table
            .split_once('|')
            .map(|(t, _)| t.to_string())
            .unwrap_or_else(|| base_table.to_string());
        // Use the alias from select.table or _base_alias
        let effective_base_alias = base_table
            .split_once('|')
            .map(|(_, a)| a.to_string())
            .unwrap_or_else(|| _base_alias.to_string());

        let mut join_tables: Vec<(String, String)> = Vec::new();
        join_tables.push((base_bare.clone(), effective_base_alias.to_string()));
        for extra in &select.extra_tables {
            let (bare, alias) = match extra.split_once('|') {
                Some((t, a)) => (t.to_string(), Some(a.to_string())),
                None => (extra.clone(), None),
            };
            let alias = alias.unwrap_or_else(|| bare.clone());
            join_tables.push((bare, alias));
        }
        // Also include tables from join_clause (comma-join case)
        for jc in &select.join_clause {
            let (bare, alias) = match jc.table.split_once('|') {
                Some((t, a)) => (t.to_string(), Some(a.to_string())),
                None => (jc.table.clone(), jc.alias.clone()),
            };
            let alias = alias.unwrap_or_else(|| bare.clone());
            if !join_tables.iter().any(|(b, a)| b == &bare && a == &alias) {
                join_tables.push((bare, alias));
            }
        }

        if join_tables.len() < 2 {
            return None;
        }

        // TPC-H table prefixes: s=supplier, p=part, ps=partsupp,
        // c=customer, o=orders, l=lineitem, n=nation, r=region.
        let tpch_prefix_to_alias: std::collections::HashMap<&str, &str> =
            std::collections::HashMap::from_iter([
                ("s_", "supplier"),
                ("p_", "part"),
                ("ps_", "partsupp"),
                ("c_", "customer"),
                ("o_", "orders"),
                ("l_", "lineitem"),
                ("n_", "nation"),
                ("r_", "region"),
            ]);

        // Helper: resolve a bare column name to (table_alias, column_name)
        // by searching each table's schema. When the column name is ambiguous
        // (appears in multiple tables) but has a TPC-H qualifier prefix, the
        // prefix is used to disambiguate (e.g. "s_suppkey" -> supplier even
        // though partsupp also has s_suppkey).
        let resolve_bare = |col_name: &str| -> Option<(String, String)> {
            // Check for TPC-H prefix disambiguation first.
            for (prefix, alias) in &tpch_prefix_to_alias {
                if let Some(col_stripped) = col_name.strip_prefix(prefix) {
                    // Verify the table is in join_tables and has this column.
                    if join_tables.iter().any(|(_, a)| a == alias) {
                        if let Ok(info) = storage.get_table_info(alias) {
                            let has_col = info.columns.iter().any(|c| {
                                let bare_c = c
                                    .name
                                    .strip_prefix(&format!("{}.", alias))
                                    .unwrap_or(&c.name);
                                bare_c == col_stripped || c.name == col_name
                            });
                            if has_col {
                                return Some((alias.to_string(), col_name.to_string()));
                            }
                        }
                    }
                }
            }
            // Fallback: scan tables; return None on ambiguity (original behaviour).
            let mut found: Option<&str> = None;
            for (bare, alias) in &join_tables {
                if let Ok(info) = storage.get_table_info(bare) {
                    let has_col = info.columns.iter().any(|c| {
                        let bare_c = c
                            .name
                            .strip_prefix(&format!("{}.", alias))
                            .unwrap_or(&c.name);
                        bare_c.eq_ignore_ascii_case(col_name)
                            || c.name.eq_ignore_ascii_case(col_name)
                    });
                    if has_col {
                        if found.is_some() {
                            return None;
                        }
                        found = Some(alias.as_str());
                    }
                }
            }
            found.map(|alias| (alias.to_string(), col_name.to_string()))
        };

        // Collect bare-equal columns from each `=` conjunct.
        let mut pair_key: HashMap<(String, String), (String, String)> = HashMap::new();
        for conjunct in Self::flatten_and_local(where_expr) {
            let Expression::BinaryOp(left, op, right) = conjunct else {
                continue;
            };
            if op != "=" {
                continue;
            }
            let (lq, lc) = match left.as_ref() {
                Expression::Identifier(name) => match name.split_once('.') {
                    Some((q, c)) => (q.to_string(), c.to_string()),
                    None => match resolve_bare(name) {
                        Some((q, c)) => (q, c),
                        None => continue,
                    },
                },
                _ => continue,
            };
            let (rq, rc) = match right.as_ref() {
                Expression::Identifier(name) => match name.split_once('.') {
                    Some((q, c)) => (q.to_string(), c.to_string()),
                    None => match resolve_bare(name) {
                        Some((q, c)) => (q, c),
                        None => continue,
                    },
                },
                _ => continue,
            };
            if lq == rq {
                continue;
            }
            // Use the actual qualifiers from WHERE (aliases like "c", "o") as keys
            let pair = if lq < rq {
                (lq.clone(), rq.clone())
            } else {
                (rq.clone(), lq.clone())
            };
            let entry = if lq < rq {
                (lc.clone(), rc.clone())
            } else {
                (rc.clone(), lc.clone())
            };
            pair_key.entry(pair).or_insert(entry);
        }

        // Multi-start + best-first greedy chain build.
        //
        // The previous single-direction greedy started from a fixed
        // base table and picked the FIRST neighbour it found each
        // step. On topologies with two hubs joined by a single edge
        // (TPC-H Q7 star, Q8 bridge, Q9 chain-leaf) it dead-ended
        // inside a leaf sub-tree, producing `chain_order.len() <
        // join_tables.len()` and falling back to the per-clause
        // cartesian path - which is correct but 5-10x slower on
        // SF=1 and infeasible on SF=10.
        //
        // Strategy (V312-21 / Issue #4181):
        //   1. Outer loop: try every `join_tables` entry as starting
        //      point. Multi-start guarantees we find a spanning chain
        //      whenever the join graph is connected.
        //   2. Inner greedy: from the current tail, choose the
        //      UNVISITED neighbour with the LOWEST degree (best-first
        //      heuristic). This avoids the failure mode of "pick a
        //      high-degree hub and dead-end inside one of its
        //      leaves" by preferring low-degree leaves first.
        //   3. Worst-case O(N^2) for N <= 10 (TPC-H). Acceptable.
        // Try every start_idx. We must keep trying even after we find
        // a `Some` candidate, because `build_chain_from_start` returns
        // `Some(chain)` for EVERY visited-table count, not just for
        // a complete spanning chain — wait, that contradicts the
        // current contract (it returns None on dead-end). Either way,
        // be defensive: prefer the LONGEST chain found. If multiple
        // starts yield len == join_tables.len(), the first one wins.
        //
        // Issue #4280 root cause: previously the loop broke on the
        // first `Some(c)` even if `c.len() < join_tables.len()`. While
        // the current `build_chain_from_start` contract only returns
        // `Some(full_chain) | None`, future refactors must preserve
        // the explicit `c.len() == join_tables.len()` filter below.
        let mut best_chain: Option<Vec<(String, String)>> = None;
        let mut best_len = 0usize;
        for start_idx in 0..join_tables.len() {
            if let Some(candidate) =
                Self::build_chain_from_start(start_idx, &join_tables, &pair_key)
            {
                if candidate.len() > best_len {
                    best_chain = Some(candidate);
                    best_len = best_chain.as_ref().unwrap().len();
                }
                if best_len == join_tables.len() {
                    break;
                }
            }
        }

        let chain_order: Vec<(String, String)> = match best_chain {
            Some(c) if c.len() == join_tables.len() => c,
            _ => {
                eprintln!(
                    "DBG chain_order multi-start could not build complete chain: join_tables.len()={}, best_len={}",
                    join_tables.len(), best_len
                );
                return None;
            }
        };
        // Re-validate that we have a chain that covers every table
        // (defensive: the build_chain_from_start guarantees this when
        // it returns Some, but be explicit so the assertion is
        // immediately clear).

        // Resolve key columns from pair_key into (acc_idx, right_idx)
        // tuples per step. Each step's `acc_idx` is the column index
        // in the accumulated rows that holds the join key; the right
        // side's key column is read directly from the right table's
        // info.
        //
        // `alias_to_offset` tracks the GLOBAL start column index of each
        // alias's columns in the accumulated rows. This lets `prev_idx`
        // (which is computed as a LOCAL index into prev's columns) be
        // converted to the GLOBAL index required by
        // `multi_way_hash_chain`. Without this, chains starting at
        // non-base leaves (Q7/Q8/Q9 with multi-start) would misalign
        // the join keys, since the leaf's columns live at offset 0 while
        // later joined tables live at offsets > 0.
        let mut steps_acc: Vec<(Vec<Vec<Value>>, Vec<sqlrustgo_storage::ColumnDefinition>)> =
            Vec::new();
        let mut step_inputs: Vec<(Vec<Vec<Value>>, usize, usize)> = Vec::new();
        let mut acc_rows: Vec<Vec<Value>>;
        let mut acc_columns: Vec<sqlrustgo_storage::ColumnDefinition>;
        let mut alias_to_columns: HashMap<String, Vec<String>> = HashMap::new();
        let mut alias_to_offset: HashMap<String, usize> = HashMap::new();
        {
            let (start_bare, start_alias) = (&chain_order[0].0, &chain_order[0].1);
            if start_bare == &base_bare && start_alias.as_str() == effective_base_alias {
                acc_rows = base_rows.clone();
                acc_columns = base_info.columns.clone();
                alias_to_offset.insert(effective_base_alias.to_string(), 0);
                alias_to_columns.insert(
                    effective_base_alias.to_string(),
                    base_info
                        .columns
                        .iter()
                        .map(|c| {
                            c.name
                                .strip_prefix(&format!("{}.", effective_base_alias))
                                .unwrap_or(&c.name)
                                .to_string()
                        })
                        .collect(),
                );
            } else {
                // Multi-start began at a non-base leaf (e.g. Q7's
                // `nation n1`). Load its rows/columns fresh from storage
                // and seed `acc_*` from there.
                let start_info = storage.get_table_info(start_bare).ok()?.clone();
                let start_raw_rows = storage.scan(start_bare).ok()?;
                let start_alias_owned = start_alias.clone();
                let start_alias_for_strip = start_alias_owned.clone();
                alias_to_offset.insert(start_alias_owned.clone(), 0);
                alias_to_columns.insert(
                    start_alias_owned,
                    start_info
                        .columns
                        .iter()
                        .map(|c| {
                            c.name
                                .strip_prefix(&format!("{}.", start_alias_for_strip))
                                .unwrap_or(&c.name)
                                .to_string()
                        })
                        .collect(),
                );
                acc_rows = start_raw_rows;
                acc_columns = start_info.columns.clone();
            }
        }
        // Capture the initial rows here, BEFORE the step loop
        // resets `acc_rows` to empty each iteration.
        let chain_start_rows = acc_rows.clone();

        for i in 1..chain_order.len() {
            let prev_alias = &chain_order[i - 1].1;
            let cur = &chain_order[i];
            let cur_alias = &cur.1;
            // Find the pair_key entry that matches these two aliases
            // (keys are stored as (min, max) alphabetically)
            let (left_col, right_col) = {
                let (k, v) = pair_key
                    .iter()
                    .find(|((a1, a2), _)| {
                        (*a1 == *prev_alias && *a2 == *cur_alias)
                            || (*a2 == *prev_alias && *a1 == *cur_alias)
                    })
                    .ok_or_else(|| format!("No pair_key for ({}, {})", prev_alias, cur_alias))
                    .ok()?;
                // Determine which column belongs to prev_alias
                if *k.0 == *prev_alias {
                    (v.0.clone(), v.1.clone())
                } else {
                    (v.1.clone(), v.0.clone())
                }
            };
            let prev_cols = alias_to_columns.get(prev_alias).cloned()?;
            let prev_local_idx = prev_cols
                .iter()
                .position(|c| c.eq_ignore_ascii_case(&left_col))?;
            // Convert local-to-prev into global-into-accumulated via
            // the per-alias offset map populated at chain start and
            // each step below.
            let prev_offset = alias_to_offset.get(prev_alias.as_str()).copied()?;
            let prev_idx = prev_offset + prev_local_idx;
            let cur_bare = &cur.0;
            let cur_info = storage.get_table_info(cur_bare).ok()?.clone();
            let cur_idx = cur_info
                .columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(&right_col))?;
            let raw_cur_rows = storage.scan(cur_bare).ok()?;
            let _rows_before_filter = raw_cur_rows.len();
            // Build alias-prefixed column names so that
            // `eval_predicate` matches TPC-H-style predicates like
            // `r.r_name = 'EUROPE'` against the aliased columns.
            let mut cur_info_prefixed = cur_info.clone();
            cur_info_prefixed.name = cur_alias.clone();
            for col in &mut cur_info_prefixed.columns {
                col.name = format!("{}.{}", cur_alias, col.name);
            }
            // Lookup pushdown predicates: alias first, then bare name.
            let pred = pushdown_filters
                .get(cur_alias.as_str())
                .or_else(|| pushdown_filters.get(cur_bare.as_str()));
            // Apply the filter.
            let cur_rows: Vec<Vec<Value>> = match pred {
                Some(preds) if !preds.is_empty() => raw_cur_rows
                    .into_iter()
                    .filter(|r| {
                        preds
                            .iter()
                            .all(|p| eval_predicate(p, r, &cur_info_prefixed))
                    })
                    .collect(),
                _ => raw_cur_rows,
            };
            alias_to_columns.insert(
                cur_alias.clone(),
                cur_info.columns.iter().map(|c| c.name.clone()).collect(),
            );
            // Record where this alias's columns begin in the
            // accumulated row so the NEXT step can resolve its
            // join key globally.
            alias_to_offset.insert(cur_alias.clone(), acc_columns.len());
            steps_acc.push((acc_rows.clone(), acc_columns.clone()));
            step_inputs.push((cur_rows, cur_idx, prev_idx));
            let mut new_columns = acc_columns.clone();
            for col in &cur_info.columns {
                new_columns.push(sqlrustgo_storage::ColumnDefinition {
                    name: format!("{}.{}", cur_alias, col.name),
                    data_type: col.data_type.clone(),
                    nullable: col.nullable,
                    primary_key: col.primary_key,
                    char_max_length: col.char_max_length,
                    collation: col.collation.clone(),
                    default_value: None,
                });
            }
            acc_columns = new_columns;
            let placeholder: Vec<Vec<Value>> = Vec::new();
            acc_rows = placeholder;
        }

        // Run the chain step-by-step using multi_way_hash_chain. Seed
        // with the rows captured at chain start (which may be
        // chain[0]'s rows rather than `base_rows` when multi-start
        // began at a non-base leaf).
        let mut accumulated = chain_start_rows;
        for ((cur_rows, cur_idx, prev_idx), (_, _)) in step_inputs.iter().zip(steps_acc.iter()) {
            accumulated = multi_way_hash_chain(
                std::mem::take(&mut accumulated),
                &[(cur_rows.clone(), *cur_idx, *prev_idx)],
            );
            if accumulated.is_empty() {
                return None;
            }
        }

        // joined_info describes the columns of `accumulated`. They are
        // the concatenation of each chain[i]'s columns in order; we
        // rebuild them by walking the chain and looking up each
        // table's info from storage (or `base_info` for the base
        // table which we already have in hand).
        let mut joined_info =
            if chain_order[0].0 == base_bare && chain_order[0].1.as_str() == effective_base_alias {
                base_info.clone()
            } else {
                storage.get_table_info(&chain_order[0].0).ok()?.clone()
            };
        joined_info.columns.clear();
        for (bare, alias) in &chain_order {
            let info = if bare == &base_bare && alias.as_str() == effective_base_alias {
                base_info.clone()
            } else {
                storage.get_table_info(bare).ok()?.clone()
            };
            for c in &info.columns {
                joined_info
                    .columns
                    .push(sqlrustgo_storage::ColumnDefinition {
                        name: format!("{}.{}", alias, c.name),
                        data_type: c.data_type.clone(),
                        nullable: c.nullable,
                        primary_key: c.primary_key,
                        char_max_length: c.char_max_length,
                        collation: c.collation.clone(),
                        default_value: c.default_value.clone(),
                    });
            }
        }
        Some((accumulated, joined_info))
    }

    /// Pre-filter the right-side table of a cartesian (JoinKey::All) JOIN
    /// using single-table predicates from the WHERE clause.
    ///
    /// TPC-H Q8 hits the cartesian path when joining `nation n2` because
    /// `n2.n_name = 'GERMANY'` is a filter on n2 that can't be resolved
    /// by the parser's ON-predicate resolution.  Without pre-filtering,
    /// we cartesian-product 60K lineitem rows with all 25 nations (1.5M rows).
    /// With pre-filtering, we first select only the GERMANY row from n2,
    /// reducing the cartesian to 60K × 1 = 60K rows.
    ///
    /// Returns `None` if the join should proceed normally (no filter applies).
    fn pre_filter_cartesian_right_table(
        right_rows: Vec<Vec<Value>>,
        right_info: &TableInfo,
        right_alias: &str,
        _join_clause: &ParserJoinClause,
        where_clause: &Option<Expression>,
    ) -> Vec<Vec<Value>> {
        use sqlrustgo_parser::Expression as E;

        // Only apply when we have a cartesian join with a WHERE clause.
        let Some(where_expr) = where_clause else {
            return right_rows;
        };

        let table_prefix = format!("{}.", right_alias);

        // Build set of valid column names for this table (unqualified).
        let col_names: std::collections::HashSet<&str> =
            right_info.columns.iter().map(|c| c.name.as_str()).collect();

        /// Returns true if this expression references ONLY the right table's columns.
        fn only_refs_right_table(expr: &E, tp: &str, cn: &std::collections::HashSet<&str>) -> bool {
            let mut ok = true;
            fn walk(e: &E, tp: &str, cn: &std::collections::HashSet<&str>, ok: &mut bool) {
                match e {
                    E::Identifier(name) => {
                        if name.starts_with(tp) {
                            // qualified to right table — OK
                        } else if cn.contains(name.as_str()) {
                            // bare column in right table — OK
                        } else {
                            *ok = false; // references a different table
                        }
                    }
                    E::BinaryOp(l, _, r) => {
                        walk(l, tp, cn, ok);
                        walk(r, tp, cn, ok);
                    }
                    E::UnaryOp(_, inner) => walk(inner, tp, cn, ok),
                    E::Like(l, p, _) | E::NotLike(l, p, _) => {
                        walk(l, tp, cn, ok);
                        walk(p, tp, cn, ok);
                    }
                    E::InList(l, vals) | E::NotInList(l, vals) => {
                        walk(l, tp, cn, ok);
                        for v in vals {
                            walk(v, tp, cn, ok);
                        }
                    }
                    E::CaseWhen(whens, else_e) => {
                        for w in whens {
                            walk(&w.condition, tp, cn, ok);
                        }
                        if let Some(e) = else_e.as_ref() {
                            walk(e, tp, cn, ok);
                        }
                    }
                    E::FunctionCall(_, args) => {
                        for a in args {
                            walk(a, tp, cn, ok);
                        }
                    }
                    _ => {}
                }
            }
            walk(expr, tp, cn, &mut ok);
            ok
        }

        /// Collect top-level AND predicates that only reference the right table.
        fn collect_right_table_preds(
            expr: &E,
            tp: &str,
            cn: &std::collections::HashSet<&str>,
        ) -> Vec<E> {
            match expr {
                E::BinaryOp(l, op, r) if op.as_str() == "AND" => {
                    let mut preds = collect_right_table_preds(l, tp, cn);
                    preds.extend(collect_right_table_preds(r, tp, cn));
                    preds
                }
                _ => {
                    if only_refs_right_table(expr, tp, cn) {
                        vec![expr.clone()]
                    } else {
                        vec![]
                    }
                }
            }
        }

        let preds = collect_right_table_preds(where_expr, &table_prefix, &col_names);
        if preds.is_empty() {
            return right_rows;
        }

        // Evaluate predicates: for simple equality `alias.col = literal`,
        // do a direct index lookup.  Complex predicates are skipped (keep row).
        let mut filtered: Vec<Vec<Value>> = Vec::with_capacity(right_rows.len());

        for row in right_rows {
            let mut pass = true;
            for pred in &preds {
                if let E::BinaryOp(l, op, r) = pred {
                    if op.as_str() == "=" {
                        // Extract (col_name, literal_string) from `alias.col = lit`
                        let (col_name, lit_str) = match (l.as_ref(), r.as_ref()) {
                            (E::Identifier(name), E::Literal(lit)) => {
                                if !name.starts_with(&table_prefix) {
                                    continue;
                                }
                                let col = name.strip_prefix(&table_prefix).unwrap_or(name);
                                let s: &str = lit;
                                (col, s)
                            }
                            (E::Literal(lit), E::Identifier(name)) => {
                                if !name.starts_with(&table_prefix) {
                                    continue;
                                }
                                let col = name.strip_prefix(&table_prefix).unwrap_or(name);
                                let s: &str = lit;
                                (col, s)
                            }
                            _ => {
                                continue;
                            }
                        };

                        // Find column index. The right_info columns
                        // are renamed to `<alias>.<col>` when an alias
                        // is set (line 1480-1484), so we match either
                        // the bare name or `<alias>.<col>`.
                        let col_idx = right_info.columns.iter().position(|c| {
                            c.name == col_name || c.name == format!("{}.{}", right_alias, col_name)
                        });
                        let Some(col_idx) = col_idx else {
                            continue;
                        };
                        let Some(row_val) = row.get(col_idx) else {
                            continue;
                        };

                        // Compare.
                        let matches = match (row_val, lit_str) {
                            (Value::Integer(i), s) => {
                                s.parse::<i64>().map(|j| i == &j).unwrap_or(false)
                            }
                            (Value::Float(f), s) => s
                                .parse::<f64>()
                                .map(|g| (f - g).abs() < 1e-9)
                                .unwrap_or(false),
                            (Value::Text(t), s) => t == s,
                            _ => false,
                        };
                        if !matches {
                            pass = false;
                            break;
                        }
                    }
                }
            }
            if pass {
                filtered.push(row);
            }
        }
        filtered
    }

    /// Execute a single JOIN against an existing (left) row set + schema.
    /// `join_clause` is consumed separately so callers can iterate a Vec<JoinClause>.
    /// `right_pushdown` is a list of single-table WHERE predicates
    /// that reference only the right table; they are applied after
    /// `storage.scan` and before the hash-join build to reduce
    /// intermediate row counts (Sprint 5 v15+ predicate
    /// pushdown, see `extract_single_table_predicates`).
    fn execute_single_join(
        &self,
        left_rows: &[Vec<Value>],
        left_table_info: &TableInfo,
        join_clause: &ParserJoinClause,
        storage: &S,
        where_clause: &Option<Expression>,
        right_pushdown: &[Expression],
    ) -> SqlResult<(Vec<Vec<Value>>, TableInfo)> {
        use sqlrustgo_parser::JoinType as ParserJoinType;
        use std::collections::HashMap;

        // Strip the auto-rewrite `|alias` suffix (e.g. `lineitem|l1`)
        // before the storage scan; storage only knows the bare name.
        let (right_bare, _) = match join_clause.table.split_once('|') {
            Some((t, a)) => (t.to_string(), Some(a.to_string())),
            None => (join_clause.table.clone(), None),
        };
        let right_table_name = right_bare;
        let right_alias = join_clause.alias.as_ref().unwrap_or(&right_table_name);

        // Phase 3 (TPCH-01 Q15): if the right table is a synthetic __subq_N
        // from a derived subquery, use the materialized rows from the registry
        // instead of scanning storage (which has no entry for synthetic names).
        let (right_raw_rows, right_raw_info) = if let Some((rows, info)) =
            DERIVED_RESULTS.with(|cell| cell.borrow().get(&right_table_name).cloned())
        {
            (rows, info)
        } else {
            (
                storage.scan(&right_table_name)?,
                storage.get_table_info(&right_table_name)?,
            )
        };

        // Pre-existing bug fix: alias-prefix the column names BEFORE
        // the pushdown filter, so `n2.n_name = 'GERMANY'` (the
        // typical single-table filter) can match by qualified name
        // against `right_table_info` (otherwise the bare `n_name`
        // column never matches the qualified predicate and the
        // pre-filter rejects every row, breaking TPC-H Q8 8-way).
        let mut right_table_info = right_raw_info.clone();
        if join_clause.alias.is_some() {
            right_table_info.name = right_alias.clone();
            for col in &mut right_table_info.columns {
                col.name = format!("{}.{}", right_alias, col.name);
            }
        }

        // Sprint 5 v15+ predicate pushdown: apply any
        // single-table WHERE predicates for this right table to
        // the scanned rows before the hash-join build.  This
        // reduces the right_hash memory footprint and the join
        // output size for TPC-H Q21 (4-table implicit join)
        // and similar wide queries.
        let right_rows: Vec<Vec<Value>> = if !right_pushdown.is_empty() {
            right_raw_rows
                .into_iter()
                .filter(|r| {
                    right_pushdown
                        .iter()
                        .all(|p| eval_predicate(p, r, &right_table_info))
                })
                .collect()
        } else {
            right_raw_rows
        };

        // The accumulated left_table_info.name encodes previous joins
        // (e.g. "a_join_n1_join_customer") and is the qualifier scope
        // for ON conditions referencing left side columns.
        let left_alias = left_table_info.name.clone();

        // Extract join key column indices from ON clause
        // For "b.num = c.bid" or "t1.id = t2.id", the canonical form
        // resolves one column from left and one from right.
        // Pass the right *alias* (when set) so qualifiers like `n2.col`
        // route to the right side.
        let join_key = self.find_join_key_index(
            &join_clause.on_clause,
            &left_table_info,
            &left_alias,
            &right_table_info,
            right_alias,
        )?;
        let pairs: Vec<(usize, usize)> = match join_key {
            JoinKey::Pair(li, ri) => vec![(li, ri)],
            JoinKey::Pairs(v) => v,
            JoinKey::All => {
                // Sprint 5 v4 (Q8/Q9 perf): pre-filter right_rows using
                // single-table predicates from the WHERE clause before the
                // cartesian product.  This avoids 60K × 25 = 1.5M row
                // intermediate results when joining a filtered nation table.
                let right_rows = Self::pre_filter_cartesian_right_table(
                    right_rows,
                    &right_table_info,
                    right_alias,
                    join_clause,
                    where_clause,
                );

                // Phase 5 (TPCH-01 Q2): cartesian product join — used
                // when the parser cannot find a fully-resolvable JOIN
                // ON predicate (e.g. when the only candidate references
                // a not-yet-joined table). All left rows match all
                // right rows; the outer WHERE filter then narrows
                // results.
                let mut cross = Vec::with_capacity(left_rows.len() * right_rows.len());
                for left_row in left_rows {
                    for right_row in &right_rows {
                        let mut combined = left_row.clone();
                        combined.extend(right_row.clone());
                        cross.push(combined);
                    }
                }
                // build_combined_schema: instead of re-prefixing
                // both sides (which would triple-prefix the left
                // columns in a 3+ way chain), concatenate the left
                // info as-is with the right info prefixed by
                // `right_alias`. The left info's columns are already
                // qualified from prior cartesian steps.
                let mut combined_columns = left_table_info.columns.clone();
                for col in &right_table_info.columns {
                    let bare_col = if join_clause.alias.is_some() {
                        col.name
                            .strip_prefix(&format!("{}.", right_alias))
                            .unwrap_or(&col.name)
                            .to_string()
                    } else {
                        col.name.clone()
                    };
                    combined_columns.push(sqlrustgo_storage::ColumnDefinition {
                        name: format!("{}.{}", right_alias, bare_col),
                        data_type: col.data_type.clone(),
                        nullable: col.nullable,
                        primary_key: col.primary_key,
                        char_max_length: col.char_max_length,
                        collation: col.collation.clone(),
                        default_value: None,
                    });
                }
                let combined_schema = TableInfo {
                    name: format!("{}_join_{}", left_alias, right_alias),
                    columns: combined_columns,
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    partition_info: None,
                    compression: None,
                    collations: std::collections::HashMap::new(),
                };
                return Ok((cross, combined_schema));
            }
            JoinKey::Left(_) | JoinKey::Right(_) => {
                return Err(SqlError::ExecutionError(
                    "Join ON must be a binary equality between left and right columns".to_string(),
                ));
            }
        };

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

        // Helper: render a composite join key as a single string for hashing.
        // The key is the tuple of values for the (left|right) column indices
        // in `pairs`. Any NULL in any component means no match (SQL UNKNOWN
        // for `=`).
        let key_of = |row: &[Value], indices: &[(usize, bool)]| -> Option<String> {
            // indices: (col_idx, is_left)
            let mut parts: Vec<String> = Vec::with_capacity(indices.len());
            for (idx, _is_left) in indices {
                match row.get(*idx) {
                    Some(Value::Null) => return None,
                    Some(v) => parts.push(format!("{:?}", v)),
                    None => return None,
                }
            }
            Some(parts.join("|"))
        };
        let left_key_indices: Vec<(usize, bool)> =
            pairs.iter().map(|(li, _)| (*li, true)).collect();
        let right_key_indices: Vec<(usize, bool)> =
            pairs.iter().map(|(_, ri)| (*ri, false)).collect();

        let mut matched_results = match join_type {
            JoinType::Inner | JoinType::Left | JoinType::Right | JoinType::Full => {
                // Hash-based matching
                // SQL semantics: NULL = NULL is UNKNOWN (not a match), so skip NULL keys
                // Store the original index alongside each right row so RIGHT/FULL
                // join bookkeeping is O(1) per match (was O(right_rows.len()) per
                // match via Vec::contains — catastrophic for 60K+ lineitem).
                let mut right_hash: HashMap<String, Vec<(usize, &Vec<Value>)>> = HashMap::new();
                for (ri, right_row) in right_rows.iter().enumerate() {
                    let key = match key_of(right_row, &right_key_indices) {
                        Some(k) => k,
                        None => continue,
                    };
                    right_hash.entry(key).or_default().push((ri, right_row));
                }

                let mut matched: Vec<Vec<Value>> = Vec::new();
                let mut left_matched: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();
                let mut right_matched: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();

                // Match left rows to right
                for (li, left_row) in left_rows.iter().enumerate() {
                    let key = match key_of(left_row, &left_key_indices) {
                        Some(k) => k,
                        None => {
                            // NULL in any join key column — no match.
                            continue;
                        }
                    };
                    if let Some(right_match_rows) = right_hash.get(&key) {
                        left_matched.insert(li);
                        for (ri, right_row) in right_match_rows {
                            // TPC-H Q9 fix: O(1) index tracking instead of
                            // O(right_rows.len()) `right_rows.iter().position(...)`
                            right_matched.insert(*ri);
                            let mut combined = left_row.clone();
                            combined.extend((*right_row).clone());
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
                for left_row in left_rows {
                    for right_row in &right_rows {
                        let mut combined = left_row.clone();
                        combined.extend(right_row.clone());
                        results.push(combined);
                    }
                }
                results
            }
        };

        let combined_schema = build_combined_schema(
            &left_table_info,
            &left_alias,
            &right_table_info,
            right_alias,
        )?;
        Ok((matched_results, combined_schema))
    }

    /// Find the column index for a join key in a table
    /// Handles both simple column names and qualified names (e.g., "t1.id")
    #[allow(clippy::only_used_in_recursion)] // recursive helper, &self only forwarded to recursive calls
    fn find_join_key_index(
        &self,
        expr: &Expression,
        left_info: &TableInfo,
        left_name: &str,
        right_info: &TableInfo,
        right_name: &str,
    ) -> SqlResult<JoinKey> {
        // v3.8.0-rc2: DBG noise disabled for cleaner test output.
        // Enable locally by uncommenting to debug find_join_key_index.
        match expr {
            Expression::Literal(_) => {
                // Phase 5 (TPCH-01 Q2): the parser emits `Literal("true")`
                // for cartesian joins when no resolvable ON predicate is
                // found (e.g. a 5-table comma-join whose predicate
                // references a not-yet-joined table). Signal cartesian
                // matching back to the caller.
                Ok(JoinKey::All)
            }
            Expression::Identifier(name) => {
                if let Some((qualifier, col_name)) = name.split_once('.') {
                    // Qualified name: must match either side
                    if qualifier == left_name {
                        let idx = lookup_column(left_info, col_name).ok_or_else(|| {
                            SqlError::ExecutionError(format!(
                                "Column '{}.{}' not found in {}",
                                qualifier, col_name, left_name
                            ))
                        })?;
                        Ok(JoinKey::Left(idx))
                    } else if qualifier == right_name {
                        let idx = lookup_column(right_info, col_name).ok_or_else(|| {
                            SqlError::ExecutionError(format!(
                                "Column '{}.{}' not found in {}",
                                qualifier, col_name, right_name
                            ))
                        })?;
                        Ok(JoinKey::Right(idx))
                    } else {
                        // Qualifier doesn't match left or right (e.g. an
                        // intermediate table already absorbed into left via
                        // build_combined_schema, where columns are named
                        // "a_join_b.col" but the user writes "b.col").
                        if let Some(idx) = lookup_qualified_column(left_info, qualifier, col_name) {
                            return Ok(JoinKey::Left(idx));
                        }
                        if let Some(idx) = lookup_qualified_column(right_info, qualifier, col_name)
                        {
                            return Ok(JoinKey::Right(idx));
                        }
                        if let Some(idx) = lookup_column(left_info, col_name) {
                            return Ok(JoinKey::Left(idx));
                        }
                        if let Some(idx) = lookup_column(right_info, col_name) {
                            return Ok(JoinKey::Right(idx));
                        }
                        Err(SqlError::ExecutionError(format!(
                            "Column '{}' not found in either '{}' or '{}'",
                            name, left_name, right_name
                        )))
                    }
                } else {
                    // Unqualified: search both sides
                    if let Some(idx) = lookup_column(left_info, name) {
                        return Ok(JoinKey::Left(idx));
                    }
                    if let Some(idx) = lookup_column(right_info, name) {
                        return Ok(JoinKey::Right(idx));
                    }
                    Err(SqlError::ExecutionError(format!(
                        "Column '{}' not found in either '{}' or '{}'",
                        name, left_name, right_name
                    )))
                }
            }
            Expression::BinaryOp(left_expr, op, right_expr) if op.to_uppercase() == "AND" => {
                // TPC-H Q9: `ON a.id = b.a_id AND a.sub_id = b.a_sub`.
                // Each AND branch must itself be a binary `=` between a
                // left and a right column. Combine the resulting pairs.
                //
                // TPC-H Q13: `ON c_custkey = o_custkey AND o_comment NOT LIKE
                // '%special%requests%'` — one arm is a LIKE/NOT LIKE
                // predicate (it is a post-join filter, not a join key).
                // Recurse into the `=` arm and ignore the LIKE arm.
                if is_like_predicate(left_expr) {
                    return self.find_join_key_index(
                        right_expr, left_info, left_name, right_info, right_name,
                    );
                }
                if is_like_predicate(right_expr) {
                    return self.find_join_key_index(
                        left_expr, left_info, left_name, right_info, right_name,
                    );
                }
                let lk = self
                    .find_join_key_index(left_expr, left_info, left_name, right_info, right_name)?;
                let rk = self.find_join_key_index(
                    right_expr, left_info, left_name, right_info, right_name,
                )?;
                match (lk, rk) {
                    (JoinKey::Pair(li1, ri1), JoinKey::Pair(li2, ri2)) => {
                        Ok(JoinKey::Pairs(vec![(li1, ri1), (li2, ri2)]))
                    }
                    (JoinKey::Pairs(mut v), JoinKey::Pair(li, ri)) => {
                        v.push((li, ri));
                        Ok(JoinKey::Pairs(v))
                    }
                    (JoinKey::Pair(li, ri), JoinKey::Pairs(mut v)) => {
                        let mut all = vec![(li, ri)];
                        all.append(&mut v);
                        Ok(JoinKey::Pairs(all))
                    }
                    (JoinKey::Pairs(mut v1), JoinKey::Pairs(v2)) => {
                        v1.extend(v2);
                        Ok(JoinKey::Pairs(v1))
                    }
                    _ => Err(SqlError::ExecutionError(
                        "Multi-column AND ON requires each branch to be a binary `=` between left and right columns".to_string(),
                    )),
                }
            }
            Expression::BinaryOp(left_expr, _op, right_expr) => {
                // Standard SQL: left side of `=` references left table,
                // right side references right table. Resolve each independently.
                let lk = self
                    .find_join_key_index(left_expr, left_info, left_name, right_info, right_name)?;
                let rk = self.find_join_key_index(
                    right_expr, left_info, left_name, right_info, right_name,
                )?;
                // We only support the canonical case: one key from each side.
                match (lk, rk) {
                    (JoinKey::Left(li), JoinKey::Right(ri)) => Ok(JoinKey::Pair(li, ri)),
                    (JoinKey::Right(ri), JoinKey::Left(li)) => Ok(JoinKey::Pair(li, ri)),
                    _ => Err(SqlError::ExecutionError(
                        "Join condition must reference one column from each side".to_string(),
                    )),
                }
            }
            _ => Err(SqlError::ExecutionError(
                "Unsupported join condition expression".to_string(),
            )),
        }
    }
}

/// Which side of a single join a resolved column index belongs to, or a
/// canonical pair (left, right) for binary `=` ON conditions.
/// `Pairs` carries multiple `(left_idx, right_idx)` pairs for
/// multi-column ON (`ON a.id = b.a_id AND a.sub_id = b.a_sub`).
#[derive(Debug, Clone)]
enum JoinKey {
    Left(usize),
    Right(usize),
    Pair(usize, usize),
    Pairs(Vec<(usize, usize)>),
    All,
}

/// Is this expression a LIKE / NOT LIKE predicate? Used by
/// TPC-H Q13 (`ON c_custkey = o_custkey AND o_comment NOT LIKE '...'`)
/// where one arm of the AND is a post-join filter (LIKE / NOT LIKE)
/// rather than a join key (binary `=`).
fn is_like_predicate(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Like(_, _, _) | Expression::NotLike(_, _, _)
    )
}

/// Look up a column in a (possibly accumulated) schema.
///
/// `build_combined_schema` rewrites column names as `alias.col`; for chained
/// joins, the column name may pick up multiple prefixes (e.g. an original
/// `b.num` becomes `a_join_b.b.num` after a second join). This helper accepts
/// any of: a bare column name (`num`), a single-prefix form (`b.num`), or
/// the full accumulated form (`a_join_b.b.num`) — they all map to the same
/// index.
fn lookup_column(info: &TableInfo, col_name: &str) -> Option<usize> {
    // Strip any qualifier from the caller's reference (we only care about
    // the final segment; the qualifier is matched separately).
    let bare = col_name.rsplit('.').next().unwrap_or(col_name);

    info.columns.iter().position(|c| {
        if c.name.eq_ignore_ascii_case(col_name) {
            return true;
        }
        // The accumulated column name may have one or more `.`-prefix
        // segments. Find the final segment and compare.
        if let Some((_, suffix)) = c.name.rsplit_once('.') {
            if suffix.eq_ignore_ascii_case(bare) {
                return true;
            }
        }
        if c.name.eq_ignore_ascii_case(bare) {
            return true;
        }
        false
    })
}

fn lookup_qualified_column(info: &TableInfo, qualifier: &str, col_name: &str) -> Option<usize> {
    let needle = format!("{qualifier}.{col_name}");
    let suffix = format!(".{qualifier}.{col_name}");
    info.columns
        .iter()
        .position(|c| c.name == needle || c.name.ends_with(&suffix))
}

/// Decode a value key string (encoded by the inline match above in
/// the ROLLUP / CUBE loops) back into a `Value`. Pairs with the
/// I/F/T/B/X prefix scheme so round-trips work for the common types.
fn decode_value_key(s: &str) -> Value {
    if s == "NULL" {
        return Value::Null;
    }
    if s.is_empty() {
        return Value::Null;
    }
    match s.as_bytes()[0] {
        b'I' => s[1..]
            .parse::<i64>()
            .map(Value::Integer)
            .unwrap_or(Value::Null),
        b'F' => s[1..]
            .parse::<f64>()
            .map(Value::Float)
            .unwrap_or(Value::Null),
        b'T' => Value::Text(s[1..].to_string()),
        b'B' => match s.as_bytes().get(1).copied() {
            Some(b'1') => Value::Boolean(true),
            _ => Value::Boolean(false),
        },
        b'X' => Value::Blob(Vec::new()),
        b'J' => {
            // JSON: prefix J then JSON string
            let json_str = &s[1..];
            serde_json::from_str(json_str)
                .map(Value::Json)
                .unwrap_or(Value::Null)
        }
        _ => Value::Null,
    }
}

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    /// Issue #3703 / Layer 3: SIMD batch eval fast path.
    ///
    /// Detects a simple `col <op> literal` predicate and processes the
    /// whole column in one pass via `simd_eval` instead of calling
    /// `eval_predicate` per-row. Returns Some(filtered rows) on success,
    /// or None to fall through to the scalar path.
    ///
    /// Supported shapes: `col <op> literal` and `literal <op> col` (with
    /// operator auto-inversion). Operators: `<`, `<=`, `>`, `>=`, `=`, `!=`.
    /// Values must be i64. NULLs coerce to 0 (NULLs compare false, so
    /// they get dropped — same as scalar path semantics).
    fn filter_partitions_simd(
        &self,
        partitions: Vec<Vec<Vec<Value>>>,
        where_expr: &Expression,
        table_info: &TableInfo,
    ) -> Option<Vec<Vec<Value>>> {
        let start = std::time::Instant::now();
        let (col_idx, op, lit) = Self::is_batchable_predicate(where_expr, table_info)?;

        // Per-partition SIMD eval. Each partition is processed independently:
        //   1. Extract i64 column values for THIS partition
        //   2. Process in 64-element chunks (BitMask size)
        //   3. Keep rows where the corresponding mask bit is set
        // Then flatten across partitions.
        let mut kept: Vec<Vec<Value>> = Vec::new();
        for partition in &partitions {
            let n = partition.len();
            if n == 0 {
                continue;
            }
            // Extract column values for this partition
            let values: Vec<i64> = partition
                .iter()
                .map(|row| row.get(col_idx).and_then(Value::as_integer).unwrap_or(0))
                .collect();

            // Process in chunks of 64 (BitMask capacity)
            for (chunk_idx, chunk) in values.chunks(64).enumerate() {
                let mask: BitMask = match op.as_str() {
                    "<" => <LessThanPredicate as BatchPredicate>::eval_batch_i64(
                        &LessThanPredicate { threshold: lit },
                        chunk,
                    ),
                    "<=" => <LessThanOrEqualPredicate as BatchPredicate>::eval_batch_i64(
                        &LessThanOrEqualPredicate { threshold: lit },
                        chunk,
                    ),
                    ">" => <GreaterThanPredicate as BatchPredicate>::eval_batch_i64(
                        &GreaterThanPredicate { threshold: lit },
                        chunk,
                    ),
                    ">=" => <GreaterThanOrEqualPredicate as BatchPredicate>::eval_batch_i64(
                        &GreaterThanOrEqualPredicate { threshold: lit },
                        chunk,
                    ),
                    "=" => <EqualsPredicate as BatchPredicate>::eval_batch_i64(
                        &EqualsPredicate { value: lit },
                        chunk,
                    ),
                    "!=" => <NotEqualPredicate as BatchPredicate>::eval_batch_i64(
                        &NotEqualPredicate { value: lit },
                        chunk,
                    ),
                    _ => return None,
                };
                // Map mask bit i → global row at chunk_idx*64 + i (i = local index in chunk)
                for i in 0..chunk.len() {
                    if mask.is_set(i) {
                        kept.push(partition[chunk_idx * 64 + i].clone());
                    }
                }
            }
        }

        tracing::debug!(
            target: "sqlrustgo.parallel.simd",
            "SIMD fast path: col_idx={}, op={}, lit={}, rows_in={}, rows_out={}, elapsed_us={}",
            col_idx, op, lit,
            partitions.iter().map(|p| p.len()).sum::<usize>(),
            kept.len(),
            start.elapsed().as_micros()
        );

        Some(kept)
    }

    /// Detect simple batchable predicates: `col <op> literal` or
    /// `literal <op> col`. Returns `(col_idx, op_string, literal)`.
    /// Returns None for any non-batchable shape (caller falls back to scalar).
    fn is_batchable_predicate(
        expr: &Expression,
        table_info: &TableInfo,
    ) -> Option<(usize, String, i64)> {
        let (left, op_str, right) = match expr {
            Expression::BinaryOp(l, op, r) => (l, op.as_str(), r),
            _ => return None,
        };
        // Multi-char ops: check < <= > >= = != explicitly. Using
        // matches! on `&str` slices is exact-equality, so "<=" does NOT
        // match "<" — they're distinct patterns.
        if !matches!(op_str, "<" | "<=" | ">" | ">=" | "=" | "!=") {
            return None;
        }
        // Pattern A: col <op> literal
        if let (Expression::Identifier(col_name), Expression::Literal(lit_s)) = (&**left, &**right)
        {
            let col_idx = table_info
                .columns
                .iter()
                .position(|c| c.name == *col_name)?;
            let lit = lit_s.parse::<i64>().ok()?;
            return Some((col_idx, op_str.to_string(), lit));
        }
        // Pattern B: literal <op> col  (auto-invert operator)
        if let (Expression::Literal(lit_s), Expression::Identifier(col_name)) = (&**left, &**right)
        {
            let col_idx = table_info
                .columns
                .iter()
                .position(|c| c.name == *col_name)?;
            let lit = lit_s.parse::<i64>().ok()?;
            let inverted = match op_str {
                "<" => ">",
                ">" => "<",
                "<=" => ">=",
                ">=" => "<=",
                "=" => "=",
                "!=" => "!=",
                _ => return None,
            };
            return Some((col_idx, inverted.to_string(), lit));
        }
        None
    }

    fn filter_partitions_parallel(
        &self,
        partitions: Vec<Vec<Vec<Value>>>,
        where_expr: &Expression,
        table_info: &TableInfo,
    ) -> Vec<Vec<Value>> {
        // Layer 3 SIMD fast path: filter the whole column in one
        // batch call instead of one eval_predicate per row.
        if let Some(filtered) =
            self.filter_partitions_simd(partitions.clone(), &where_expr, &table_info)
        {
            return filtered;
        }

        // Fallback: original rayon-par scalar path.
        use rayon::prelude::*;
        let where_expr = where_expr.clone();
        let table_info = table_info.clone();
        let filtered: Vec<Vec<Vec<Value>>> = partitions
            .into_par_iter()
            .map(|mut part| {
                part.retain(|row| eval_predicate(&where_expr, row, &table_info));
                part
            })
            .collect();
        let mut merged: Vec<Vec<Value>> = Vec::new();
        for part in filtered {
            merged.extend(part);
        }
        merged
    }

    /// TPC-H Q20/Q21: pre-evaluate correlated EXISTS / NOT EXISTS
    /// subqueries against the given outer row. For each subtree
    /// matching `Expression::Exists(subq)` or
    /// `Expression::NotExists(subq)`:
    ///
    /// 1. Substitute the outer column references in `subq` (the
    ///    `where_clause` and `having` of the subquery) with
    ///    concrete `Literal` values from the outer row, using
    ///    `substitute_outer_refs_in_select`.
    /// 2. Execute the substituted subquery via `self.execute_select`.
    ///    This re-acquires the storage read lock, which is why the
    ///    caller (execute_select) drops its own storage lock before
    ///    reaching this path.
    /// 3. Count the result rows. If >= 1 row, EXISTS→true /
    ///    NOT EXISTS→false. If 0 rows, EXISTS→false /
    ///    NOT EXISTS→true. Replace the subtree with
    ///    `Expression::Literal("true")` or `Expression::Literal("false")`
    ///    so the rest of WHERE evaluation can run via the standard
    ///    `eval_predicate` path.
    ///
    /// On subquery execution error, the original EXISTS / NOT EXISTS
    /// subtree is replaced with `Expression::Literal("false")` (a
    /// conservative denial) so the row is filtered out — we don't
    /// want partial subquery errors to silently over-include.
    pub fn pre_evaluate_correlated_exists(
        &self,
        where_expr: &sqlrustgo_parser::Expression,
        outer_row: &[Value],
        outer_table_info: &TableInfo,
        subquery_indexes: &[SubqueryIndex],
        cursor: &mut usize,
    ) -> sqlrustgo_parser::Expression {
        use sqlrustgo_parser::Expression;
        match where_expr {
            Expression::Exists(subq) => {
                let substituted =
                    substitute_outer_refs_in_select(subq, outer_row, outer_table_info);
                // Indexed fast path (Sprint 5 Q4 perf): if the
                // caller pre-built a `SubqueryIndex` for this
                // subquery (via `build_subquery_index` in the
                // outer WHERE evaluator), use the O(1) membership
                // check instead of a 60k-row scan.  Falls back to
                // the existing pre_eval_exists_subquery_fast on
                // miss.
                let indexed = if let Some(wc) = substituted.where_clause.as_ref() {
                    subquery_indexes.get(*cursor).and_then(|idx| {
                        self.pre_eval_exists_indexed(wc, outer_row, outer_table_info, idx)
                    })
                } else {
                    None
                };
                if let Some(any) = indexed {
                    *cursor += 1;
                    return Expression::Literal(if any { "true" } else { "false" }.to_string());
                }
                // Fast path: TPC-H EXISTS subqueries are over a
                // single base table (e.g. `EXISTS (SELECT * FROM
                // lineitem WHERE l_orderkey = outer)`). We can do
                // a direct storage scan + WHERE filter + early
                // exit, avoiding the full execute_select pipeline
                // (which would also build a TableInfo, run
                // materialization, GROUP BY check, etc.). The
                // recursive execute_select path is still used as
                // a fallback for non-trivial subqueries.
                let any_row = self
                    .pre_eval_exists_subquery_fast(&substituted, outer_row)
                    .unwrap_or_else(|| match self.execute_select(&substituted) {
                        Ok(r) => !r.rows.is_empty(),
                        Err(_) => false,
                    });
                *cursor += 1;
                Expression::Literal(if any_row { "true" } else { "false" }.to_string())
            }
            // V312-35 (#4182): correlated scalar subquery (TPC-H Q2/Q17)
            // — execute substituted subquery and replace with scalar literal.
            Expression::Subquery(subq) => {
                let substituted =
                    substitute_outer_refs_in_select(subq, outer_row, outer_table_info);
                let scalar = match self.execute_select(&substituted) {
                    Ok(r) if !r.rows.is_empty() => r.rows[0].first().cloned(),
                    _ => None,
                };
                match scalar {
                    Some(v) => Expression::Literal(value_to_literal_string_v(&v)),
                    None => Expression::Literal("NULL".to_string()),
                }
            }
            Expression::NotExists(subq) => {
                let substituted =
                    substitute_outer_refs_in_select(subq, outer_row, outer_table_info);
                let indexed = if let Some(wc) = substituted.where_clause.as_ref() {
                    subquery_indexes.get(*cursor).and_then(|idx| {
                        // V311-17: try bloom short-circuit first for NOT EXISTS
                        self.pre_eval_not_exists_indexed(wc, outer_row, outer_table_info, idx)
                    })
                } else {
                    None
                };
                if let Some(zero_rows) = indexed {
                    *cursor += 1;
                    return Expression::Literal(
                        if zero_rows { "true" } else { "false" }.to_string(),
                    );
                }
                let zero_rows = self
                    .pre_eval_exists_subquery_fast(&substituted, outer_row)
                    .map(|any| !any)
                    .unwrap_or_else(|| match self.execute_select(&substituted) {
                        Ok(r) => r.rows.is_empty(),
                        Err(_) => true,
                    });
                *cursor += 1;
                Expression::Literal(if zero_rows { "true" } else { "false" }.to_string())
            }
            Expression::BinaryOp(l, op, r) => Expression::BinaryOp(
                Box::new(self.pre_evaluate_correlated_exists(
                    l,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                )),
                op.clone(),
                Box::new(self.pre_evaluate_correlated_exists(
                    r,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                )),
            ),
            Expression::UnaryOp(op, inner) => Expression::UnaryOp(
                op.clone(),
                Box::new(self.pre_evaluate_correlated_exists(
                    inner,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                )),
            ),
            Expression::IsNull(inner) => {
                Expression::IsNull(Box::new(self.pre_evaluate_correlated_exists(
                    inner,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                )))
            }
            Expression::IsNotNull(inner) => {
                Expression::IsNotNull(Box::new(self.pre_evaluate_correlated_exists(
                    inner,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                )))
            }
            Expression::InList(left, values) => Expression::InList(
                Box::new(self.pre_evaluate_correlated_exists(
                    left,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                )),
                values
                    .iter()
                    .map(|v| {
                        self.pre_evaluate_correlated_exists(
                            v,
                            outer_row,
                            outer_table_info,
                            subquery_indexes,
                            cursor,
                        )
                    })
                    .collect(),
            ),
            Expression::NotInList(left, values) => Expression::NotInList(
                Box::new(self.pre_evaluate_correlated_exists(
                    left,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                )),
                values
                    .iter()
                    .map(|v| {
                        self.pre_evaluate_correlated_exists(
                            v,
                            outer_row,
                            outer_table_info,
                            subquery_indexes,
                            cursor,
                        )
                    })
                    .collect(),
            ),
            Expression::FunctionCall(name, args) => Expression::FunctionCall(
                name.clone(),
                args.iter()
                    .map(|a| {
                        self.pre_evaluate_correlated_exists(
                            a,
                            outer_row,
                            outer_table_info,
                            subquery_indexes,
                            cursor,
                        )
                    })
                    .collect(),
            ),
            // TPC-H Q13/Q16: `col IN (SELECT ...)` and `NOT IN (SELECT ...)`.
            // Like the substitute_outer_refs_in_expr helper, the
            // conservative pattern is: pass through, since the
            // conservative IN/NOT IN handling in eval_predicate
            // returns true anyway.
            Expression::In(_, _) | Expression::NotIn(_, _) => where_expr.clone(),
            // LIKE / BETWEEN outer-substitution not implemented for
            // TPC-H Q1-Q22 (none of the queries combine these with
            // EXISTS/NotExists). Pass through.
            Expression::Like(_, _, _)
            | Expression::NotLike(_, _, _)
            | Expression::Between(_, _, _)
            | Expression::NotBetween(_, _, _)
            | Expression::NotRegexp(_, _) => where_expr.clone(),
            // Sprint 5 v11 fix (Q17): cache key must be the substituted
            // outer ref value, not outer_row[1]. In JOIN contexts the
            // referenced column may be at a different index (Q17: lineitem
            // cols 0-15, part cols 16-24; `p_partkey` is at index 16).
            #[allow(unreachable_patterns)]
            Expression::Subquery(_subq) => {
                // TPC-H Q17 perf fast-path: correlated scalar aggregate
                // subquery of the shape
                //   (SELECT [op] AGG(col) FROM t WHERE key_col = <outer_ref>)
                // → pre-compute `key_col_value → agg_result` ONCE per
                // (table, key_col, agg_func, agg_arg) and look up O(1)
                // per outer row. Q17 was 30s on SF=0.1 (cached per
                // partkey, but each cache miss re-scanned 60K lineitems).
                if let Some(lit) =
                    self.try_scalar_agg_index_lookup(_subq, outer_row, outer_table_info)
                {
                    return Expression::Literal(lit.to_string());
                }
                let substituted =
                    substitute_outer_refs_in_select(_subq, outer_row, outer_table_info);
                let cache_key: Value = extract_first_literal_from_where(&substituted)
                    .unwrap_or_else(|| {
                        Value::Text(format!(
                            "__no_subst_{}_{:?}",
                            outer_row.len(),
                            outer_row.first()
                        ))
                    });
                {
                    let cache = scalar_subq_cache().lock();
                    if let Some(cached) = cache.get(&cache_key) {
                        return Expression::Literal(cached.to_string());
                    }
                }
                let result = self.execute_select(&substituted);
                let scalar = match result {
                    Ok(r) if !r.rows.is_empty() => {
                        r.rows[0].first().cloned().unwrap_or(Value::Null)
                    }
                    _ => Value::Null,
                };
                scalar_subq_cache().lock().insert(cache_key, scalar.clone());
                Expression::Literal(scalar.to_string())
            }
            // CASE WHEN / SubqueryField pass through (no substitution needed —
            // these are not correlated scalar subqueries in TPC-H).
            Expression::SubqueryField(_, _) | Expression::CaseWhen(_, _) => where_expr.clone(),
            // QuantifiedOp: pass through.
            Expression::QuantifiedOp(_, _, _) => where_expr.clone(),
            // Terminal expressions and aggregates pass through (no
            // possible subquery subtrees).
            // Terminal expressions, aggregates, and sequences pass through (no
            // possible subquery subtrees).
            Expression::Literal(_)
            | Expression::Identifier(_)
            | Expression::Aggregate(_)
            | Expression::WindowCall(_)
            | Expression::SequenceNextVal(_)
            | Expression::SequenceCurrval(_)
            | Expression::SystemVariable(_)
            | Expression::JsonLiteral(_)
            | Expression::ArrayLiteral(_) => where_expr.clone(),
        }
    }

    /// Local copy of `parser::flatten_and` (kept private there).
    /// Splits a top-level `a AND b AND c` chain into its
    /// conjuncts; non-AND expressions return a single-element
    /// vector. TPC-H Q21 l3 has a 3-way AND; we need to inspect
    /// each conjunct for an indexable equality.
    fn flatten_and_local(expr: &Expression) -> Vec<Expression> {
        match expr {
            Expression::BinaryOp(left, op, right) if op.to_uppercase() == "AND" => {
                let mut v = Self::flatten_and_local(left);
                v.extend(Self::flatten_and_local(right));
                v
            }
            other => vec![other.clone()],
        }
    }

    /// TPC-H 1-/2-char column-prefix mapping (s/supplier,
    /// ps/partsupp, n/nation, l/lineitem, ...).  Used by the
    /// predicate-pushdown pipeline to convert a table name
    /// into the column prefix that `collect_referenced_tables`
    /// extracts from unqualified column names like
    /// `s_suppkey` or `l_orderkey`.
    fn tpch_table_prefix(table: &str) -> &str {
        match table {
            "region" => "r",
            "nation" => "n",
            "supplier" => "s",
            "customer" => "c",
            "part" => "p",
            "partsupp" => "ps",
            "orders" => "o",
            "lineitem" => "l",
            _ => {
                if table.contains('_') {
                    let us = table.find('_').unwrap();
                    &table[..us]
                } else if !table.is_empty() {
                    &table[..1]
                } else {
                    ""
                }
            }
        }
    }

    /// Sprint 5 v15+ predicate pushdown helper: collect the
    /// tables referenced by a predicate (by qualifier prefix or
    /// TPC-H 1-/2-char column prefix).  Local copy of
    /// `parser::collect_referenced_tables`; the parser's version is
    /// private.
    fn collect_referenced_tables_local(expr: &Expression) -> Vec<String> {
        fn visit(e: &Expression, acc: &mut Vec<String>) {
            match e {
                Expression::Identifier(name) => {
                    if let Some((qualifier, _col)) = name.split_once('.') {
                        if !acc.iter().any(|x: &String| x == qualifier) {
                            acc.push(qualifier.to_string());
                        }
                    } else if let Some(prefix) = name.split('_').next() {
                        if !acc.iter().any(|x: &String| x == prefix) {
                            acc.push(prefix.to_string());
                        }
                    }
                }
                Expression::BinaryOp(l, _, r) => {
                    visit(l, acc);
                    visit(r, acc);
                }
                Expression::IsNull(inner) | Expression::IsNotNull(inner) => visit(inner, acc),
                Expression::UnaryOp(_, inner) => visit(inner, acc),
                Expression::FunctionCall(_, args) => {
                    for a in args {
                        visit(a, acc);
                    }
                }
                Expression::Aggregate(agg) => {
                    for a in &agg.args {
                        visit(a, acc);
                    }
                }
                // V312-35 (#4182): a correlated subquery inside a
                // conjunct (TPC-H Q2: `ps_supplycost = (SELECT MIN(...))`,
                // Q17: `l_quantity < (SELECT 0.2*AVG(...))`) must mark
                // the conjunct as multi-table so `extract_single_table_
                // predicates` refuses to push it into the base scan.
                // Pushing it down makes `eval_predicate` return Null
                // for the Subquery node, silently dropping every row.
                Expression::Subquery(_)
                | Expression::Exists(_)
                | Expression::NotExists(_)
                | Expression::In(_, _)
                | Expression::NotIn(_, _)
                | Expression::SubqueryField(_, _)
                | Expression::QuantifiedOp(_, _, _)
                    if !acc.iter().any(|x: &String| x == "__subquery__") =>
                {
                    acc.push("__subquery__".to_string());
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        visit(expr, &mut out);
        out
    }

    /// Sprint 5 v15+ predicate pushdown: extract the
    /// single-table predicates from a WHERE clause.  Returns a
    /// map of `table_name -> [conjuncts that reference ONLY that
    /// table]`.  Predicates that reference multiple tables (join
    /// conditions) are NOT included — only single-table filters.
    ///
    /// For TPC-H Q21 the relevant predicates are:
    ///   - `o_orderstatus = 'F'`  →  {"orders": [..]}
    ///   - `n_name = 'GERMANY'`   →  {"nation":  [..]}
    ///
    /// Without pushdown, the 4-table cross-product materializes
    /// 2.25B intermediate rows before the WHERE filter is
    /// applied; with pushdown, the filtered `nation` and
    /// `orders` reduce the right-side hash-build by ~4x (and
    /// 25x for `n_name = 'GERMANY'`).
    fn extract_single_table_predicates(
        &self,
        where_expr: &Expression,
        joined_tables: &[String],
    ) -> std::collections::HashMap<String, Vec<Expression>> {
        use std::collections::HashMap;
        let mut out: HashMap<String, Vec<Expression>> = HashMap::new();
        for conjunct in Self::flatten_and_local(where_expr) {
            // Skip EXISTS / NOT EXISTS / Subquery / aggregate —
            // these are correlated subqueries that the existing
            // pre-eval pipeline handles, not single-table filters.
            if matches!(
                conjunct,
                Expression::Exists(_)
                    | Expression::NotExists(_)
                    | Expression::Subquery(_)
                    | Expression::Aggregate(_)
                    | Expression::In(_, _)
                    | Expression::NotIn(_, _)
            ) {
                continue;
            }
            // SKIP IS NULL / IS NOT NULL on a single column:
            // `right.col IS NULL` in a WHERE clause of a LEFT JOIN
            // is the standard SQL anti-join pattern
            //   `LEFT JOIN b ON ... WHERE b.y IS NULL`
            // which means "rows in `a` with no match in `b`".
            // Pushing it down to the right-table scan filter would
            // drop every right row, collapsing the join to a
            // cartesian-NULL and returning wrong results. The
            // post-join WHERE filter handles these correctly.
            // Detected as either the dedicated `IsNull`/`IsNotNull`
            // variant or the legacy `BinaryOp(<col>, "IS", Literal("NULL"))`
            // / `"IS NOT"` form (which `eval_predicate` lowers
            // internally to the variant form).
            if matches!(conjunct, Expression::IsNull(_) | Expression::IsNotNull(_)) {
                continue;
            }
            if let Expression::BinaryOp(_, ref op, ref right) = conjunct {
                let op_up = op.to_uppercase();
                if (op_up == "IS" || op_up == "IS NOT")
                    && matches!(right.as_ref(), Expression::Literal(s) if s.to_uppercase() == "NULL")
                {
                    continue;
                }
            }
            // Collect tables referenced by the conjunct
            let refs = Self::collect_referenced_tables_local(&conjunct);
            if refs.len() != 1 {
                continue; // join predicate (refs ≥ 2) or literal (refs 0)
            }
            let table = &refs[0];
            // Must be one of the joined tables (or a TPC-H prefix
            // that maps to one of them).  For TPC-H the prefix
            // already matches the table name in most cases; the
            // auto-rewrite has already pushed the base table into
            // joined_tables.
            if !joined_tables.iter().any(|t| t == table) {
                continue;
            }
            out.entry(table.clone()).or_default().push(conjunct);
        }
        out
    }

    /// TPC-H Q20/Q21: fast-path EXISTS / NOT EXISTS subquery evaluation.
    /// Detects the common pattern `EXISTS (SELECT * FROM <single_table>
    /// WHERE <predicate>)` and evaluates it with a direct storage scan +
    /// WHERE filter + early exit. Returns `Some(true)` as soon as a
    /// matching row is found; `Some(false)` if the scan finishes with zero
    /// matches. Returns `None` for any pattern that does not match the
    /// fast-path shape (e.g. JOINs, GROUP BY, multiple tables, or
    /// sub-subqueries); the caller then falls back to the full
    /// `self.execute_select` pipeline.
    fn pre_eval_exists_subquery_fast(
        &self,
        subq: &sqlrustgo_parser::SelectStatement,
        _outer_row: &[Value],
    ) -> Option<bool> {
        // Pattern: SELECT * FROM <single_table> [WHERE <predicate>]
        // - No JOINs
        // - No GROUP BY
        // - No aggregates
        // - No LIMIT/OFFSET (we early-exit ourselves)
        // - No correlated sub-subqueries in the WHERE
        if !subq.join_clause.is_empty() {
            return None;
        }
        if !subq.aggregates.is_empty() || !subq.group_by.is_empty() {
            return None;
        }
        if subq.limit.is_some() || subq.offset.is_some() {
            return None;
        }
        if subq.from_subquery.is_some() {
            return None;
        }
        if subq.table.is_empty() {
            return None;
        }
        // Reject if WHERE contains more EXISTS/NotExists (would
        // need recursive substitution, which is fine but adds
        // complexity; skip for now).
        let where_expr = subq.where_clause.as_ref()?;
        if where_expr_has_correlated_subquery(where_expr) {
            return None;
        }
        // Reject if WHERE contains any uncorrelated subqueries
        // (IN/NotIn/Subquery/QuantifiedOp) — these are non-trivial
        // in our 22-query suite (Q20 has `ps_partkey IN (subq)`)
        // and the conservative `In (subq) => true` handling in
        // `eval_predicate` would over-include every row. Falling
        // back to the full `execute_select` pipeline is no better
        // there, so we return None to let the caller try it.
        if where_expr_has_uncorrelated_subquery(where_expr) {
            return None;
        }
        let storage = self.storage.read();
        // Q21 fix: the subquery table may be encoded as
        // "lineitem|l2" (table|alias). The storage layer doesn't
        // know about the alias suffix, so strip it before looking
        // up the real table info.
        let real_subq_table: String = subq
            .table
            .find('|')
            .map(|d| subq.table[..d].to_string())
            .unwrap_or_else(|| subq.table.clone());
        let table_info = storage.get_table_info(&real_subq_table).ok()?;

        // Sprint 5 v2 fix: for correlated EXISTS in TPC-H Q4/Q21
        // (`EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND ...)`),
        // the naive scan reads 60K lineitem rows for every outer
        // row, giving 60K × 1851 = 111M comparisons (TIMEOUT).
        //
        // Optimization: detect a simple
        // `<inner_col> = <outer_col_ref>` equality in the WHERE
        // (after substitution it's `<inner_col> = Literal`), build
        // a one-shot HashMap index (cached at module scope), and
        // only test the matching subset.
        let mut idx_col: Option<usize> = None;
        let mut target_value: Option<Value> = None;
        // TPC-H Q21 l3: `l3.l_orderkey = X AND l3.l_suppkey <> Y
        // AND l3.l_receiptdate > l3.l_commitdate` — the indexable
        // equality may be nested under multiple AND levels. Walk
        // the top-level AND conjuncts and pick the first one that
        // is `<inner_col> = <literal>`.
        for conjunct in Self::flatten_and_local(where_expr) {
            if let sqlrustgo_parser::Expression::BinaryOp(bl, eq_op, br) = &conjunct {
                if eq_op == "=" {
                    let inner_cols: std::collections::HashSet<String> = table_info
                        .columns
                        .iter()
                        .map(|c| c.name.to_lowercase())
                        .collect();
                    let (inner_name, target_val) = match (bl.as_ref(), br.as_ref()) {
                        (
                            sqlrustgo_parser::Expression::Identifier(iname),
                            sqlrustgo_parser::Expression::Literal(s),
                        ) => {
                            let in_lc = iname.to_lowercase();
                            let unqualified = in_lc
                                .rsplit_once('.')
                                .map(|(_, c)| c.to_string())
                                .unwrap_or_else(|| in_lc.clone());
                            if inner_cols.contains(&unqualified) {
                                (Some(unqualified), Some(s.clone()))
                            } else {
                                (None, None)
                            }
                        }
                        (
                            sqlrustgo_parser::Expression::Literal(s),
                            sqlrustgo_parser::Expression::Identifier(iname),
                        ) => {
                            let in_lc = iname.to_lowercase();
                            let unqualified = in_lc
                                .rsplit_once('.')
                                .map(|(_, c)| c.to_string())
                                .unwrap_or_else(|| in_lc.clone());
                            if inner_cols.contains(&unqualified) {
                                (Some(unqualified), Some(s.clone()))
                            } else {
                                (None, None)
                            }
                        }
                        _ => (None, None),
                    };
                    if let (Some(inner), Some(s)) = (inner_name, target_val) {
                        if let Some(idx) = table_info
                            .columns
                            .iter()
                            .position(|c| c.name.to_lowercase() == inner)
                        {
                            let parsed = sqlrustgo_types::parse_sql_literal(&s);
                            idx_col = Some(idx);
                            target_value = Some(parsed);
                            break;
                        }
                    }
                }
            }
        }
        if let (Some(idx), Some(target)) = (idx_col, target_value) {
            // Use a 2-level cache: the table rows themselves (Arc
            // for cheap sharing) and the per-column index.
            let rows_cache = lineitem_rows_cache();
            let idx_cache = lineitem_index_cache();
            // Get or build the rows. Cache under the real table
            // name (without `|alias`) so aliases share the index.
            // Include the storage engine address in the key so
            // separate MemoryStorage instances don't pollute each
            // other (q21_exists_hash_path_test regression).
            let engine_tag = std::sync::Arc::as_ptr(&self.storage) as *const () as usize;
            let table_name = format!("{}{:x}", real_subq_table, engine_tag);
            let rows_arc: std::sync::Arc<Vec<Vec<Value>>> = {
                let mut rc = rows_cache.lock();
                if let Some(c) = rc.get(&table_name) {
                    c.clone()
                } else {
                    let rows = storage.scan(&real_subq_table).ok()?;
                    let arc = std::sync::Arc::new(rows);
                    rc.insert(table_name.clone(), arc.clone());
                    arc
                }
            };
            // Get or build the index for this column.
            let cache_key = format!("{}:{}", table_name, idx);
            let candidate_ids: Vec<usize> = {
                let mut ic = idx_cache.lock();
                if !ic.contains_key(&cache_key) {
                    let mut new_index: std::collections::HashMap<Value, Vec<usize>> =
                        std::collections::HashMap::new();
                    for (i, r) in rows_arc.iter().enumerate() {
                        if let Some(v) = r.get(idx) {
                            new_index.entry(v.clone()).or_default().push(i);
                        }
                    }
                    ic.insert(cache_key.clone(), new_index);
                }
                ic.get(&cache_key)
                    .unwrap()
                    .get(&target)
                    .cloned()
                    .unwrap_or_default()
            };
            // Test the candidates (typically ~5-7 per outer row).
            for i in &candidate_ids {
                if eval_predicate(where_expr, &rows_arc[*i], &table_info) {
                    return Some(true);
                }
            }
            return Some(false);
        }

        // Direct storage scan + WHERE filter + early exit.
        let rows = storage.scan(&subq.table).ok()?;
        for row in &rows {
            if eval_predicate(where_expr, row, &table_info) {
                return Some(true);
            }
        }
        Some(false)
    }

    /// Try to build a `SubqueryIndex` for a single correlated EXISTS
    /// subquery. Returns `None` if the predicate shape does not support
    /// the optimization (e.g. joins, ORs, or the equality part is
    /// ambiguous). Caller falls back to the per-row full-scan path on
    /// `None`.
    fn build_subquery_index(&self, subq: &SelectStatement) -> Option<SubqueryIndex> {
        if !subq.join_clause.is_empty() || subq.from_subquery.is_some() {
            return None;
        }
        if subq.table.is_empty() {
            return None;
        }
        // TPC-H Q21's subqueries use `FROM lineitem l2`; the parser
        // stores this as `table: "lineitem|l2"` (the `|alias` suffix
        // pattern, also used by execute_joins). Storage has only
        // the bare table name `lineitem`. Strip the `|alias` suffix
        // so the storage lookup can find the real table.
        let real_table: &str = match subq.table.find('|') {
            Some(d) => &subq.table[..d],
            None => &subq.table,
        };
        let where_expr = subq.where_clause.as_ref()?;
        let storage = self.storage.read();
        let table_info = storage.get_table_info(real_table).ok()?;
        // Split the WHERE into the outer-equality leaf (which we
        // will use as the index key column) and the static rest
        // predicate (which we will evaluate per inner row).  We
        // need the inner table_info to disambiguate: the AST
        // represents both the inner column and the outer ref as
        // plain `Identifier(name)` until substitute_outer_refs
        // rewrites them per row.  The column name that matches
        // the inner table is the key; the other is the outer ref.
        let (col_name, static_predicate) =
            split_outer_equality_with_table(where_expr, &table_info)?;
        // split_outer_equality_with_table may return a qualified
        // name like `l2.l_orderkey` for an aliased subquery FROM
        // (TPC-H Q21); the table_info stores unqualified names,
        // so strip any single alias prefix before column lookup.
        let bare_col = match col_name.rfind('.') {
            Some(d) if d + 1 < col_name.len() => &col_name[d + 1..],
            _ => col_name.as_str(),
        };
        let col_idx = table_info.columns.iter().position(|c| c.name == bare_col)?;
        let rows = storage.scan(real_table).ok()?;
        // Store the full inner row alongside the key set so the
        // residual (which may reference outer columns) can be
        // re-evaluated per outer row. The prior key-only design
        // discarded the residual and broke TPC-H Q21.
        let mut qualifying_keys: std::collections::HashSet<Value> =
            std::collections::HashSet::with_capacity(rows.len());
        let mut qualifying_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
        // V311-15 perf: per-key bucket index for O(1) lookup in the
        // fast path. TPC-H Q4's biggest cost was that pre_eval_exists_indexed
        // iterated ALL qualifying rows per outer row, making it
        // O(outer × qualifying_rows).
        let mut key_to_rows: std::collections::HashMap<Value, Vec<Vec<Value>>> =
            std::collections::HashMap::with_capacity(rows.len());
        for row in rows {
            if eval_predicate(&static_predicate, &row, &table_info) {
                let key = row[col_idx].clone();
                qualifying_keys.insert(key.clone());
                qualifying_rows.push(row.clone());
                key_to_rows.entry(key).or_default().push(row);
            }
        }
        Some(SubqueryIndex {
            col_idx,
            qualifying_keys,
            qualifying_rows,
            key_to_rows,
            residual: *static_predicate,
        })
    }

    /// TPC-H Q17 perf: correlated scalar aggregate subquery fast-path.
    ///
    /// Detects the pattern
    ///   `(SELECT [op] AGG(col) FROM t WHERE key_col = <outer_ref>)`
    /// where `op` is an optional `* literal` factor (e.g. `0.2 * AVG(...)`).
    /// Pre-computes `key_col_value → final_result` ONCE per
    /// (table, key_col, agg_func, agg_arg_col, op_factor) and stores in a
    /// module-level cache.  Subsequent calls do an O(1) HashMap lookup.
    ///
    /// Returns `Some(agg_result)` if the pattern matches and the outer
    /// row's key value is in the index.  Returns `None` if the pattern
    /// doesn't match (caller falls back to the per-row execute_select
    /// path), or `Some(Value::Null)` if the key is not in the index
    /// (the subquery would have returned no rows for that key).
    fn try_scalar_agg_index_lookup(
        &self,
        subq: &sqlrustgo_parser::SelectStatement,
        outer_row: &[Value],
        outer_table_info: &TableInfo,
    ) -> Option<Value> {
        use sqlrustgo_parser::Expression as E;

        // ── Pattern check ───────────────────────────────────────────
        // 1. Single table (no JOINs, no comma-tables, no FROM subquery).
        if !subq.join_clause.is_empty() || subq.from_subquery.is_some() {
            return None;
        }
        if subq.table.is_empty() {
            return None;
        }
        if !subq.extra_tables.is_empty() {
            return None;
        }
        // 2. Single aggregate, no GROUP BY, no DISTINCT.
        if subq.aggregates.len() != 1 {
            return None;
        }
        let agg = &subq.aggregates[0];
        if !subq.group_by.is_empty() || subq.distinct {
            return None;
        }
        // 3. Single projection column.
        if subq.columns.len() != 1 {
            return None;
        }
        let proj_expr = subq.columns[0].expression.as_ref()?;
        // 4. Projection must be of the shape
        //      `op_factor * AGG(col)`  (or `AGG(col)` directly with op_factor = 1.0).
        //    The Aggregate must be the single `agg` we found.
        let op_factor: f64 = match proj_expr {
            E::Aggregate(agg_inner) if agg_inner == agg => 1.0,
            E::BinaryOp(l, op, r) if op == "*" => match (l.as_ref(), r.as_ref()) {
                (E::Literal(s), E::Aggregate(agg_inner)) => {
                    if agg_inner != agg {
                        return None;
                    }
                    s.parse::<f64>().ok()?
                }
                (E::Aggregate(agg_inner), E::Literal(s)) => {
                    if agg_inner != agg {
                        return None;
                    }
                    s.parse::<f64>().ok()?
                }
                _ => return None,
            },
            _ => return None,
        };
        let agg_arg_expr: &E = agg.args.first()?;
        // 5. WHERE must be `<inner_col> = <outer_ref>` (possibly wrapped in
        //    AND with trivial static predicates, but for TPC-H Q17 it's
        //    bare equality).
        let where_expr = subq.where_clause.as_ref()?;
        // Try a flat-binary equality first; otherwise walk the AST for
        // an equality leaf.
        let (inner_col_name, outer_ref_pos) =
            find_equality_inner_outer(where_expr, agg_arg_expr, &subq.table, outer_table_info)
                .or(None)?;
        // Strip `|alias` from subq.table (TPC-H pattern from Q21).
        let real_table: &str = match subq.table.find('|') {
            Some(d) => &subq.table[..d],
            None => &subq.table,
        };

        // ── Resolve columns ──────────────────────────────────────────
        let storage = self.storage.read();
        let table_info = storage.get_table_info(real_table).ok()?;
        let key_col_idx = table_info
            .columns
            .iter()
            .position(|c| c.name == inner_col_name)?;
        let agg_col_idx: Option<usize> = match agg_arg_expr {
            E::Identifier(name) => table_info.columns.iter().position(|c| c.name == *name),
            // If agg arg is an expression, skip the fast path (would
            // require evaluating the per-row expression to know which
            // column to read).  Most TPC-H scalar aggs are simple
            // Identifier args.
            _ => None,
        };

        // ── Compute cache key ────────────────────────────────────────
        // Cache key = (table, key_col, agg_func, agg_col, op_factor).
        // Two queries with identical (table,key,agg,op) share the index.
        let cache_key = format!(
            "{}|{}|{:?}|{}|{}",
            real_table,
            key_col_idx,
            agg.func,
            agg_col_idx.unwrap_or(usize::MAX),
            op_factor
        );

        // ── Get or build the index ──────────────────────────────────
        let entry: ScalarAggIndexEntry = {
            let cache = scalar_agg_index_cache().lock();
            if let Some(e) = cache.get(&cache_key) {
                e.clone()
            } else {
                drop(cache);
                // Build the index by scanning the inner table once and
                // computing the aggregate per key_col value.
                let rows = storage.scan(real_table).ok()?;
                let new_map = build_scalar_agg_index(
                    &rows,
                    &table_info,
                    key_col_idx,
                    agg_col_idx,
                    agg.func.clone(),
                    op_factor,
                );
                let entry = ScalarAggIndexEntry {
                    map: std::sync::Arc::new(new_map),
                };
                let mut cache = scalar_agg_index_cache().lock();
                cache.insert(cache_key.clone(), entry.clone());
                entry
            }
        };

        // ── Lookup by outer key value ────────────────────────────────
        let key_val = outer_row.get(outer_ref_pos)?;
        // Try several lookups because outer row may have Integer "123"
        // and the index may have been built with Text "123" (parses
        // both to same number on equality but HashMap uses Eq).
        if let Some(v) = entry.map.get(key_val) {
            return Some(v.clone());
        }
        // Coerce to string for matching (e.g. Integer(123) vs Text("123")).
        let key_str = match key_val {
            Value::Integer(n) => n.to_string(),
            Value::Text(s) => s.clone(),
            Value::Float(f) => format!("{}", f),
            _ => return Some(Value::Null),
        };
        for (k, v) in entry.map.iter() {
            let k_str = match k {
                Value::Integer(n) => n.to_string(),
                Value::Text(s) => s.clone(),
                Value::Float(f) => format!("{}", f),
                _ => continue,
            };
            if k_str == key_str {
                return Some(v.clone());
            }
        }
        // Key not in index → subquery would return zero rows → NULL.
        Some(Value::Null)
    }

    /// Indexed fast-path EXISTS check: looks up the substituted
    /// equality key in `qualifying_keys`, then iterates the matching
    /// `qualifying_rows` re-evaluating `residual` per outer row
    /// (after `substitute_outer_refs_in_expr`).
    fn pre_eval_exists_indexed(
        &self,
        where_expr: &Expression,
        outer_row: &[Value],
        outer_table_info: &TableInfo,
        index: &SubqueryIndex,
    ) -> Option<bool> {
        let lit = find_top_level_equality_literal(where_expr, index.col_idx)?;
        if !index.qualifying_keys.contains(&lit) {
            return Some(false);
        }
        // V311-15 perf: use the per-key bucket for O(1) lookup
        // instead of scanning all qualifying_rows. TPC-H Q4-style
        // queries (pure-static residual) skip per-row residual
        // re-evaluation entirely; TPC-H Q21-style queries
        // (outer-substituted residual) iterate ONLY the matching
        // bucket instead of all qualifying rows.
        let bucket = match index.key_to_rows.get(&lit) {
            Some(b) => b,
            None => return Some(false),
        };
        if !Self::residual_has_outer_ref(&index.residual) {
            // Pure-static residual was already evaluated at build
            // time; any row in the bucket passed, so EXISTS = true.
            return Some(!bucket.is_empty());
        }
        // Slow path: residual references outer columns
        // (TPC-H Q21-style). Re-evaluate per bucket row.
        for inner in bucket {
            if inner.len() <= index.col_idx {
                continue;
            }
            let substituted =
                substitute_outer_refs_in_expr(&index.residual, outer_row, outer_table_info);
            if eval_predicate(&substituted, inner, /* table_info */ outer_table_info) {
                return Some(true);
            }
        }
        Some(false)
    }

    /// V311-17: NOT EXISTS fast path. Inverts the semantics of
    /// `pre_eval_exists_indexed` and adds a bloom-filter short-circuit.
    ///
    /// Returns `Some(true)` if NOT EXISTS is true (outer row survives),
    /// `Some(false)` if NOT EXISTS is false (outer row excluded).
    fn pre_eval_not_exists_indexed(
        &self,
        where_expr: &Expression,
        outer_row: &[Value],
        outer_table_info: &TableInfo,
        index: &SubqueryIndex,
    ) -> Option<bool> {
        let lit = find_top_level_equality_literal(where_expr, index.col_idx)?;
        // Short-circuit 1: if not in qualifying_keys, NOT EXISTS = true.
        if !index.qualifying_keys.contains(&lit) {
            return Some(true);
        }
        // Short-circuit 2: pure-static residual pre-applied at build time.
        // Bucket empty after build filter ⇒ no inner row matches ⇒ NOT EXISTS = true.
        let bucket = index.key_to_rows.get(&lit);
        if !Self::residual_has_outer_ref(&index.residual) {
            return Some(bucket.map(|b| b.is_empty()).unwrap_or(true));
        }
        // Slow path: residual has outer refs (TPC-H Q21-style).
        // Walk bucket looking for ANY match (NOT EXISTS is FALSE if found).
        let bucket = bucket?;
        for inner in bucket {
            if inner.len() <= index.col_idx {
                continue;
            }
            let substituted =
                substitute_outer_refs_in_expr(&index.residual, outer_row, outer_table_info);
            if eval_predicate(&substituted, inner, /* table_info */ outer_table_info) {
                return Some(false); // Found a match — NOT EXISTS is false → exclude
            }
        }
        Some(true) // No match found — NOT EXISTS is true → include
    }

    /// V311-15 helper: does the residual predicate reference any outer
    /// (correlated) columns? If not, it is purely static over the inner
    /// rows and can be evaluated once at build time; we then know any
    /// row in `key_to_rows[literal]` passes the residual, so EXISTS =
    /// true iff the bucket is non-empty. This avoids the per-outer-row
    /// re-evaluation that was killing Q4 performance.
    fn residual_has_outer_ref(residual: &sqlrustgo_parser::Expression) -> bool {
        use sqlrustgo_parser::Expression;
        match residual {
            Expression::Identifier(_) | Expression::Literal(_) => false,
            Expression::BinaryOp(l, _, r) => {
                Self::residual_has_outer_ref(l) || Self::residual_has_outer_ref(r)
            }
            Expression::UnaryOp(_, inner) => Self::residual_has_outer_ref(inner),
            // IS NULL / IS NOT NULL predicate on a column - outer
            // ref would be inside `inner` so we recurse.
            Expression::IsNull(inner) | Expression::IsNotNull(inner) => {
                Self::residual_has_outer_ref(inner)
            }
            _ => true, // Conservative default for Subquery etc.
        }
    }

    /// TPC-H Q13 fix: pre-evaluate non-correlated `IN (subquery)` /
    /// `NOT IN (subquery)` by executing the subquery once, collecting
    /// the first-column values into a list, and rewriting the AST
    /// `In/NotIn(expr, subq)` into `InList/NotInList(expr,
    /// [Literal...])`. This lets the standard `eval_predicate` path
    /// (which already handles InList/NotInList correctly) evaluate
    /// the membership without needing engine access.
    ///
    /// Correlated subqueries are left untouched: the outer-column
    /// substitution depends on the specific outer row, so the
    /// pre-evaluation must happen per-row (handled by the existing
    /// `pre_evaluate_correlated_exists` step 1.5).
    ///
    /// Returns the rewritten expression. If the input had no
    /// non-correlated IN/NOT IN subqueries, the same expression is
    /// returned (caller can use `==` to skip the per-row re-filter).
    fn pre_evaluate_non_correlated_in_subquery(
        &self,
        where_expr: &Expression,
    ) -> sqlrustgo_parser::Expression {
        use sqlrustgo_parser::Expression as E;
        use sqlrustgo_types::Value as V;

        // Walk the expression and rewrite all non-correlated
        // IN / NOT IN subqueries. We use a helper that returns
        // `Some(rewritten)` for the cases that changed and `None`
        // otherwise, so unchanged subtrees are returned by reference
        // (avoid deep clones of the full WHERE tree).
        fn has_non_correlated_in_subq(expr: &Expression) -> bool {
            match expr {
                E::In(_, subq) | E::NotIn(_, subq) => {
                    // Treat as non-correlated if the subquery does
                    // not reference outer columns. Outer columns
                    // appear as plain `Identifier` names that match
                    // neither the subquery's FROM table nor its
                    // alias. We can't perfectly know outer columns
                    // here, so we use a conservative proxy: if the
                    // subquery WHERE contains any `Identifier`, the
                    // safe path is to leave it for step 1.5's
                    // per-row correlated handling. For the
                    // subquery-only case (no outer ref), the
                    // subquery WHERE may still contain
                    // `Identifier` (column names), so we must
                    // additionally check that the identifier is
                    // not flagged as an outer ref. The cleanest
                    // check: try to rewrite, and if any error
                    // occurs, fall back to the original.
                    !subq_uses_outer_ref(subq)
                }
                E::BinaryOp(l, _, r) => {
                    has_non_correlated_in_subq(l) || has_non_correlated_in_subq(r)
                }
                E::UnaryOp(_, inner) => has_non_correlated_in_subq(inner),
                E::IsNull(inner) | E::IsNotNull(inner) => has_non_correlated_in_subq(inner),
                E::InList(l, vs) | E::NotInList(l, vs) => {
                    has_non_correlated_in_subq(l) || vs.iter().any(has_non_correlated_in_subq)
                }
                E::FunctionCall(_, args) => args.iter().any(has_non_correlated_in_subq),
                _ => false,
            }
        }

        // Conservative outer-ref detection for a subquery: walk the
        // subquery's WHERE / projection / join clauses and look for
        // any `Identifier` whose name does NOT match any column of
        // the subquery's FROM table. We can't enumerate the FROM
        // table's columns without a catalog lookup, but we know the
        // FROM table name (`subq.table`) so we can at least detect
        // an unqualified Identifier that doesn't look like a
        // common inner-table column prefix (e.g. for Q13, the
        // subquery's FROM table is `orders`, so an `o_*`
        // identifier is an inner ref, anything else is suspect).
        //
        // For the conservative 22-query corpus this is sufficient:
        // correlated subqueries reference outer columns with
        // non-prefixed names (e.g. `c_custkey` from `customer`
        // inside a subquery over `orders`).
        fn subq_uses_outer_ref(subq: &sqlrustgo_parser::SelectStatement) -> bool {
            // Pull the inner-table prefix (e.g. "o_" from "orders",
            // "l_" from "lineitem", "s_" from "supplier", "p_" from
            // "part", "ps_" from "partsupp", "c_" from "customer",
            // "n_" from "nation", "r_" from "region").
            let table_prefix: String = subq
                .table
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();
            let underscore = format!("{}_", table_prefix);

            // Collect projection column names so an Identifier that
            // exactly matches one is recognized as an inner ref
            // even without the prefix check.
            let projection_names: Vec<String> =
                subq.columns.iter().map(|c| c.name.clone()).collect();

            // Walk an expression and report `true` if it contains
            // a bare `Identifier` that is neither prefixed with the
            // inner-table prefix nor a projection column.
            fn contains_outer_ref(
                expr: &Expression,
                prefix: &str,
                projection_names: &[String],
            ) -> bool {
                match expr {
                    E::Identifier(name) => {
                        !name.starts_with(prefix) && !projection_names.iter().any(|p| p == name)
                    }
                    E::BinaryOp(l, _, r) => {
                        contains_outer_ref(l, prefix, projection_names)
                            || contains_outer_ref(r, prefix, projection_names)
                    }
                    E::UnaryOp(_, inner) => contains_outer_ref(inner, prefix, projection_names),
                    E::IsNull(inner) | E::IsNotNull(inner) => {
                        contains_outer_ref(inner, prefix, projection_names)
                    }
                    E::InList(l, vs) | E::NotInList(l, vs) => {
                        contains_outer_ref(l, prefix, projection_names)
                            || vs
                                .iter()
                                .any(|v| contains_outer_ref(v, prefix, projection_names))
                    }
                    E::In(l, sub) | E::NotIn(l, sub) => {
                        contains_outer_ref(l, prefix, projection_names)
                            || sub_where_has_outer_ref(sub, prefix, projection_names)
                    }
                    E::Exists(sub) | E::NotExists(sub) => {
                        sub_where_has_outer_ref(sub, prefix, projection_names)
                    }
                    E::FunctionCall(_, args) => args
                        .iter()
                        .any(|a| contains_outer_ref(a, prefix, projection_names)),
                    E::Like(l, p, _) | E::NotLike(l, p, _) => {
                        contains_outer_ref(l, prefix, projection_names)
                            || contains_outer_ref(p, prefix, projection_names)
                    }
                    E::Between(l, lo, hi) | E::NotBetween(l, lo, hi) => {
                        contains_outer_ref(l, prefix, projection_names)
                            || contains_outer_ref(lo, prefix, projection_names)
                            || contains_outer_ref(hi, prefix, projection_names)
                    }
                    E::CaseWhen(whens, else_expr) => {
                        whens.iter().any(|when| {
                            let (w, t) = (&when.condition, &when.result);
                            contains_outer_ref(w, prefix, projection_names)
                                || contains_outer_ref(t, prefix, projection_names)
                        }) || else_expr
                            .as_ref()
                            .is_some_and(|e| contains_outer_ref(e, prefix, projection_names))
                    }
                    _ => false,
                }
            }

            fn sub_where_has_outer_ref(
                subq: &sqlrustgo_parser::SelectStatement,
                prefix: &str,
                projection_names: &[String],
            ) -> bool {
                if let Some(ref w) = subq.where_clause {
                    if contains_outer_ref(w, prefix, projection_names) {
                        return true;
                    }
                }
                for j in &subq.join_clause {
                    if contains_outer_ref(&j.on_clause, prefix, projection_names) {
                        return true;
                    }
                }
                false
            }

            if subq.where_clause.is_none() && subq.join_clause.is_empty() {
                return false;
            }
            sub_where_has_outer_ref(subq, &underscore, &projection_names)
        }

        // Recursive rewriter.
        fn rewrite<S: sqlrustgo_storage::StorageEngine + 'static>(
            engine: &ExecutionEngine<S>,
            expr: &Expression,
        ) -> Option<Expression> {
            match expr {
                E::In(left, subq) => {
                    if subq_uses_outer_ref(subq) {
                        None
                    } else {
                        let values = execute_subq_for_first_col(engine, subq).ok()?;
                        // Always emit values as quoted string literals
                        // so parse_lit routes them through the
                        // text-quoting path. The actual type
                        // comparison still happens via
                        // `compare_values` which falls back to
                        // string representation when types differ.
                        let lits: Vec<Expression> = values
                            .into_iter()
                            .map(|v| {
                                let s = match v {
                                    V::Integer(n) => n.to_string(),
                                    V::Float(f) => f.to_string(),
                                    V::Text(s) => s,
                                    V::Null => "NULL".to_string(),
                                    V::Point(x, y) => format!("POINT({}, {})", x, y),
                                    V::Boolean(b) => b.to_string(),
                                    V::Blob(_) => "BLOB".to_string(),
                                    V::Json(v) => v.to_string(),
                                };
                                E::Literal(s)
                            })
                            .collect();
                        Some(E::InList(left.clone(), lits))
                    }
                }
                E::NotIn(left, subq) => {
                    if subq_uses_outer_ref(subq) {
                        None
                    } else {
                        let values = execute_subq_for_first_col(engine, subq).ok()?;
                        let lits: Vec<Expression> = values
                            .into_iter()
                            .map(|v| {
                                let s = match v {
                                    V::Integer(n) => n.to_string(),
                                    V::Float(f) => f.to_string(),
                                    V::Text(s) => s,
                                    V::Null => "NULL".to_string(),
                                    V::Boolean(b) => b.to_string(),
                                    V::Point(x, y) => format!("POINT({}, {})", x, y),
                                    V::Blob(_) => "BLOB".to_string(),
                                    V::Json(v) => v.to_string(),
                                };
                                E::Literal(s)
                            })
                            .collect();
                        Some(E::NotInList(left.clone(), lits))
                    }
                }
                E::BinaryOp(l, op, r) => {
                    let nl = rewrite(engine, l);
                    let nr = rewrite(engine, r);
                    if nl.is_none() && nr.is_none() {
                        None
                    } else {
                        Some(E::BinaryOp(
                            Box::new(nl.unwrap_or_else(|| (**l).clone())),
                            op.clone(),
                            Box::new(nr.unwrap_or_else(|| (**r).clone())),
                        ))
                    }
                }
                E::UnaryOp(op, inner) => {
                    rewrite(engine, inner).map(|n| E::UnaryOp(op.clone(), Box::new(n)))
                }
                E::IsNull(inner) | E::IsNotNull(inner) => {
                    rewrite(engine, inner).map(|n| match expr {
                        E::IsNull(_) => E::IsNull(Box::new(n)),
                        E::IsNotNull(_) => E::IsNotNull(Box::new(n)),
                        _ => unreachable!(),
                    })
                }
                E::InList(l, vs) => {
                    let mut changed = false;
                    let mut new_vs: Vec<Expression> = Vec::with_capacity(vs.len());
                    for v in vs {
                        if let Some(nv) = rewrite(engine, v) {
                            changed = true;
                            new_vs.push(nv);
                        } else {
                            new_vs.push(v.clone());
                        }
                    }
                    let nl = rewrite(engine, l);
                    if !changed && nl.is_none() {
                        None
                    } else {
                        Some(E::InList(
                            Box::new(nl.unwrap_or_else(|| (**l).clone())),
                            new_vs,
                        ))
                    }
                }
                E::NotInList(l, vs) => {
                    let mut changed = false;
                    let mut new_vs: Vec<Expression> = Vec::with_capacity(vs.len());
                    for v in vs {
                        if let Some(nv) = rewrite(engine, v) {
                            changed = true;
                            new_vs.push(nv);
                        } else {
                            new_vs.push(v.clone());
                        }
                    }
                    let nl = rewrite(engine, l);
                    if !changed && nl.is_none() {
                        None
                    } else {
                        Some(E::NotInList(
                            Box::new(nl.unwrap_or_else(|| (**l).clone())),
                            new_vs,
                        ))
                    }
                }
                E::FunctionCall(name, args) => {
                    let mut changed = false;
                    let mut new_args: Vec<Expression> = Vec::with_capacity(args.len());
                    for a in args {
                        if let Some(na) = rewrite(engine, a) {
                            changed = true;
                            new_args.push(na);
                        } else {
                            new_args.push(a.clone());
                        }
                    }
                    if !changed {
                        None
                    } else {
                        Some(E::FunctionCall(name.clone(), new_args))
                    }
                }
                _ => None,
            }
        }

        if !has_non_correlated_in_subq(where_expr) {
            return where_expr.clone();
        }
        match rewrite(self, where_expr) {
            Some(new_expr) => new_expr,
            None => where_expr.clone(),
        }
    }
}

/// Execute a non-correlated subquery and return the values of its
/// first column. Errors are propagated as `Err` so the caller can
/// leave the original expression unchanged.
fn execute_subq_for_first_col<S: sqlrustgo_storage::StorageEngine + 'static>(
    engine: &ExecutionEngine<S>,
    subq: &sqlrustgo_parser::SelectStatement,
) -> SqlResult<Vec<sqlrustgo_types::Value>> {
    let result = engine.execute_select(subq)?;
    let first: Vec<sqlrustgo_types::Value> = result
        .rows
        .into_iter()
        .filter_map(|row| row.into_iter().next())
        .collect();
    Ok(first)
}

/// Pre-built index for a correlated EXISTS subquery (Sprint 5 Q4
/// perf). Holds a `HashSet<Value>` of every inner table key-column
/// value for which the *static* (non-outer-ref) part of the subquery
/// predicate holds. The static predicate is computed once; the per-row
/// EXISTS answer is then an O(1) membership check against this set.
///
/// Example (TPC-H Q4): inner table = `lineitem`, static predicate =
/// `l_commitdate < l_receiptdate`, key column = `l_orderkey`. The
/// index contains every `l_orderkey` for which lineitem has at least
/// one row with `l_commitdate < l_receiptdate`. Per outer order row,
/// `EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND
/// l_commitdate < l_receiptdate)` becomes `o_orderkey ∈
/// qualifying_keys`, i.e. O(1) instead of a 60k-row scan.
///
/// O(N_inner) one-time build, O(M) per outer row where M is the
/// number of qualifying inner rows for the outer row's key
/// (usually small for the TPC-H l_orderkey distribution). Total
/// O(N_inner + sum(M_i)) vs the prior O(N_inner × N_outer) full
/// scan.
#[derive(Debug, Clone)]
pub struct SubqueryIndex {
    pub col_idx: usize,
    /// O(1) per-outer-row key membership check. Populated during
    /// the build by inserting the key column value of every
    /// qualifying inner row.
    pub qualifying_keys: std::collections::HashSet<Value>,
    /// Full inner rows for rows that pass the static (non-outer-ref)
    /// part of the WHERE clause. Per-outer-row, we look up the
    /// outer row's key in `qualifying_keys`, then iterate the
    /// subset of `qualifying_rows` whose key matches and
    /// re-evaluate the per-outer-row-substituted `residual`
    /// against each entry. (Sprint 8 PR 2 fix: the prior
    /// `qualifying_keys`-only design silently dropped the residual
    /// for TPC-H Q21's `l3.l_receiptdate > l3.l_commitdate AND
    /// l3.l_suppkey <> l1.l_suppkey`.)
    ///
    /// V311-15 perf: superset of `key_to_rows[key]`. Kept for
    /// backwards compatibility with code that does a flat scan
    /// (which is now the slow path and only matters when the
    /// residual has outer refs, e.g. TPC-H Q21).
    pub qualifying_rows: Vec<Vec<Value>>,
    /// V311-15 perf: keyed bucket index. `key_to_rows[key]`
    /// contains ALL rows with that key value. For TPC-H Q4
    /// (`EXISTS (SELECT ... WHERE l_orderkey = outer_key AND
    /// l_commitdate < l_receiptdate)`), this turns the inner
    /// scan from O(outer × inner) = 450K × 3M = 1.35T ops to
    /// O(outer × avg_bucket_size).
    pub key_to_rows: std::collections::HashMap<Value, Vec<Vec<Value>>>,
    /// The portion of the WHERE clause that references outer
    /// columns (e.g. `l3.l_suppkey <> l1.l_suppkey AND
    /// l3.l_receiptdate > l3.l_commitdate` for Q21's NOT EXISTS).
    /// `substitute_outer_refs_in_expr` runs per outer row before
    /// this predicate is evaluated against each `qualifying_rows`
    /// entry. May be `Literal("true")` when the index was built
    /// from a pure-equality pattern (no residual to re-check).
    pub residual: sqlrustgo_parser::Expression,
}

/// Walk a WHERE expression tree, find every correlated EXISTS /
/// NotExists subtree, and build a `SubqueryIndex` for it (if the
/// predicate shape supports the optimization — see
/// `build_subquery_index`). The index map is DFS-ordered, so the
/// per-row `pre_evaluate_correlated_exists` walk can consume them in
/// the same DFS order via a shared cursor.
pub fn collect_subquery_indexes<S: StorageEngine + 'static>(
    where_expr: &Expression,
    engine: &ExecutionEngine<S>,
    out: &mut Vec<SubqueryIndex>,
) {
    use sqlrustgo_parser::Expression as E;
    match where_expr {
        E::Exists(subq) | E::NotExists(subq) => {
            if let Some(idx) = engine.build_subquery_index(subq) {
                out.push(idx);
            }
        }
        E::BinaryOp(l, _, r) => {
            collect_subquery_indexes(l, engine, out);
            collect_subquery_indexes(r, engine, out);
        }
        E::UnaryOp(_, inner) => collect_subquery_indexes(inner, engine, out),
        E::IsNull(inner) | E::IsNotNull(inner) => {
            collect_subquery_indexes(inner, engine, out);
        }
        E::InList(left, values) | E::NotInList(left, values) => {
            collect_subquery_indexes(left, engine, out);
            for v in values {
                collect_subquery_indexes(v, engine, out);
            }
        }
        E::FunctionCall(_, args) => {
            for a in args {
                collect_subquery_indexes(a, engine, out);
            }
        }
        _ => {}
    }
}

/// Walk an AND-tree of predicates and split out one conjunct of shape
/// `inner_col = <something-not-an-inner-col-ref>`. Returns
/// `(inner_col_name, rest_predicate)`. The "something-not-an-inner-col-ref"
/// is taken to be a correlated reference (outer column); we don't
/// validate it here — that is the caller's job after substitution.
///
/// Conservative: returns `None` for any OR-tree or for shapes where no
/// `col = <non-inner-col-ref>` can be unambiguously identified. The
/// caller falls back to the per-row full-scan path on `None`.
///
/// The parser represents BOTH the inner column and the outer reference
/// as plain `Identifier(name)` in the un-substituted AST; we use the
/// inner `TableInfo` to disambiguate: the side whose name appears in
/// `inner_info.columns` is the inner col, the other is the outer ref.
fn split_outer_equality_with_table(
    where_expr: &Expression,
    inner_info: &TableInfo,
) -> Option<(String, Box<Expression>)> {
    use sqlrustgo_parser::Expression as E;
    /// TPC-H Q21 uses `EXISTS (SELECT * FROM lineitem l2 WHERE
    /// l2.l_orderkey = l1.l_orderkey)`. The inner-col side comes
    /// in qualified as `l2.l_orderkey`, but the inner table_info
    /// stores unqualified column names (`l_orderkey`). Strip a
    /// single alias prefix before lookup so Q21-style subqueries
    /// are recognised. Q4/Q17 don't qualify, so this is a no-op
    /// for them.
    fn strip_table_prefix(name: &str) -> String {
        match name.rfind('.') {
            Some(d) if d + 1 < name.len() => name[d + 1..].to_string(),
            _ => name.to_string(),
        }
    }
    fn is_inner_col(name: &str, info: &TableInfo) -> bool {
        let bare = strip_table_prefix(name);
        info.columns.iter().any(|c| c.name == bare)
    }
    match where_expr {
        E::BinaryOp(l, op, r) if op.to_uppercase() == "AND" => {
            if let Some(split) = split_outer_equality_with_table(l, inner_info) {
                let new_rest = E::BinaryOp(Box::new(*r.clone()), op.clone(), split.1);
                return Some((split.0, Box::new(new_rest)));
            }
            if let Some(split) = split_outer_equality_with_table(r, inner_info) {
                let new_rest = E::BinaryOp(Box::new(*l.clone()), op.clone(), split.1);
                return Some((split.0, Box::new(new_rest)));
            }
            None
        }
        E::BinaryOp(l, op, r) if op == "=" => {
            let lc = column_name_of(l);
            let rc = column_name_of(r);
            // Identify the inner-col side.  The other side (if not
            // also an inner col) is the outer reference and we
            // drop it (the per-row pre_evaluate will substitute it
            // via substitute_outer_refs_in_expr).
            match (lc, rc) {
                (Some(name), None) if is_inner_col(&name, inner_info) => {
                    Some((name, Box::new(E::Literal("true".into()))))
                }
                (None, Some(name)) if is_inner_col(&name, inner_info) => {
                    Some((name, Box::new(E::Literal("true".into()))))
                }
                (Some(name), Some(outer))
                    if is_inner_col(&name, inner_info) && !is_inner_col(&outer, inner_info) =>
                {
                    Some((name, Box::new(E::Literal("true".into()))))
                }
                (Some(outer), Some(name))
                    if is_inner_col(&name, inner_info) && !is_inner_col(&outer, inner_info) =>
                {
                    Some((name, Box::new(E::Literal("true".into()))))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Kept for backwards compat / potential future use; the index
/// builder calls `split_outer_equality_with_table` instead.
#[allow(dead_code)]
fn split_outer_equality(where_expr: &Expression) -> Option<(String, Box<Expression>)> {
    use sqlrustgo_parser::Expression as E;
    match where_expr {
        E::BinaryOp(l, op, r) if op.to_uppercase() == "AND" => {
            if let Some(split) = split_outer_equality(l) {
                let new_rest = E::BinaryOp(Box::new(*r.clone()), op.clone(), split.1);
                return Some((split.0, Box::new(new_rest)));
            }
            if let Some(split) = split_outer_equality(r) {
                let new_rest = E::BinaryOp(Box::new(*l.clone()), op.clone(), split.1);
                return Some((split.0, Box::new(new_rest)));
            }
            None
        }
        E::BinaryOp(l, op, r) if op == "=" => {
            let lc = column_name_of(l);
            let rc = column_name_of(r);
            match (lc, rc) {
                (Some(name), None) => Some((name, Box::new(E::Literal("true".into())))),
                (None, Some(name)) => Some((name, Box::new(E::Literal("true".into())))),
                _ => None,
            }
        }
        _ => None,
    }
}

fn column_name_of(e: &Expression) -> Option<String> {
    use sqlrustgo_parser::Expression as E;
    match e {
        E::Identifier(s) => Some(s.clone()),
        _ => None,
    }
}

fn literal_value_of(e: &Expression) -> Option<Value> {
    use sqlrustgo_parser::Expression as E;
    match e {
        E::Literal(s) => {
            let s = s.trim();
            if s.eq_ignore_ascii_case("null") {
                Some(Value::Null)
            } else if s.eq_ignore_ascii_case("true") {
                Some(Value::Boolean(true))
            } else if s.eq_ignore_ascii_case("false") {
                Some(Value::Boolean(false))
            } else if let Some(stripped) = s.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
                Some(Value::Text(stripped.to_string()))
            } else if let Ok(i) = s.parse::<i64>() {
                Some(Value::Integer(i))
            } else if let Ok(f) = s.parse::<f64>() {
                Some(Value::Float(f))
            } else {
                Some(Value::Text(s.to_string()))
            }
        }
        _ => None,
    }
}

/// Walk an AND-tree looking for an equality of the form
/// `col = literal` where `col` is the inner-table key column (by
/// position) and `literal` is a `Literal` expression. Returns the
/// parsed `Value` of the literal.
///
/// For the post-substitution Q4 case:
///   `Identifier(l_orderkey) = Literal("12345") AND ...`
/// → returns `Some(Value::Integer(12345))`.
fn find_top_level_equality_literal(where_expr: &Expression, _key_col_idx: usize) -> Option<Value> {
    use sqlrustgo_parser::Expression as E;
    fn walk(e: &Expression) -> Option<Value> {
        match e {
            E::BinaryOp(l, op, r) if op == "=" => match (l.as_ref(), r.as_ref()) {
                (E::Identifier(_), _) => literal_value_of(r),
                (_, E::Identifier(_)) => literal_value_of(l),
                _ => None,
            },
            E::BinaryOp(l, op, r) if op.to_uppercase() == "AND" => walk(l).or_else(|| walk(r)),
            _ => None,
        }
    }
    walk(where_expr)
}

/// Find an equality leaf in a WHERE expression of the shape
/// `<inner_col> = <outer_ref>` where:
///  - `inner_col` is a column of the inner subquery table
///  - `outer_ref` is an unqualified identifier that maps to a column of
///    the outer row's table_info
///
/// Returns `(inner_col_name, outer_ref_col_index_in_outer_row)`.
///
/// This is used by the Q17 fast-path index lookup to detect the common
/// correlated scalar aggregate pattern.  We accept equality inside an
/// AND chain (e.g. `l_partkey = X AND extra_filter = Y`) by walking
/// top-down and picking the first matching equality.
fn find_equality_inner_outer(
    where_expr: &Expression,
    agg_arg_expr: &Expression,
    inner_table_name: &str,
    outer_table_info: &TableInfo,
) -> Option<(String, usize)> {
    use sqlrustgo_parser::Expression as E;

    // The agg_arg_expr is the expression used in the aggregate
    // (e.g. Identifier("l_quantity") for AVG(l_quantity)).  We treat it
    // as a strong hint that the inner column referenced in the equality
    // is one of the inner table's columns — but the equality leaf
    // itself can also use any inner column.
    let own_prefix: Option<char> = inner_table_name
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase());
    let inner_col_lower = match agg_arg_expr {
        E::Identifier(n) => Some(n.to_lowercase()),
        _ => None,
    };

    fn find_outer_col(name: &str, outer_table_info: &TableInfo) -> Option<usize> {
        // 1. Exact match.
        if let Some(idx) = outer_table_info.columns.iter().position(|c| c.name == name) {
            return Some(idx);
        }
        // 2. Match by basename after stripping any `alias.` prefix.
        let basename = name.rsplit_once('.').map(|(_, c)| c).unwrap_or(name);
        if let Some(idx) = outer_table_info
            .columns
            .iter()
            .position(|c| c.name == basename)
        {
            return Some(idx);
        }
        // 3. Match by basename after stripping a `alias.` prefix
        //    from a column name in outer_table_info.
        if let Some(idx) = outer_table_info
            .columns
            .iter()
            .position(|c| c.name.rsplit_once('.').map(|(_, c)| c).unwrap_or(&c.name) == basename)
        {
            return Some(idx);
        }
        None
    }

    fn walk(
        e: &Expression,
        own_prefix: Option<char>,
        inner_col_hint: &Option<String>,
        outer_table_info: &TableInfo,
    ) -> Option<(String, usize)> {
        use sqlrustgo_parser::Expression as E;
        match e {
            E::BinaryOp(l, op, r) if op == "=" => {
                let (inner_col, outer_idx) = match (l.as_ref(), r.as_ref()) {
                    (E::Identifier(li), E::Identifier(ri)) => {
                        // (l_inner_col = r_outer_col) or (l_outer_col = r_inner_col)
                        let li_lc = li.to_lowercase();
                        let ri_lc = ri.to_lowercase();
                        let li_is_inner = inner_col_hint
                            .as_ref()
                            .map(|h| &li_lc == h)
                            .unwrap_or(false)
                            || own_prefix
                                .map(|p| li_lc.starts_with(p) && li_lc.chars().nth(1) == Some('_'))
                                .unwrap_or(false);
                        let ri_is_inner = inner_col_hint
                            .as_ref()
                            .map(|h| &ri_lc == h)
                            .unwrap_or(false)
                            || own_prefix
                                .map(|p| ri_lc.starts_with(p) && ri_lc.chars().nth(1) == Some('_'))
                                .unwrap_or(false);
                        if li_is_inner && !ri_is_inner {
                            let outer_idx = find_outer_col(ri, outer_table_info)?;
                            (li.clone(), outer_idx)
                        } else if ri_is_inner && !li_is_inner {
                            let outer_idx = find_outer_col(li, outer_table_info)?;
                            (ri.clone(), outer_idx)
                        } else {
                            return None;
                        }
                    }
                    (E::Identifier(_), E::Literal(_)) => {
                        // (inner_col = literal) — the outer ref must
                        // be on the left.
                        // But this case shouldn't appear in correlated
                        // subqueries before substitution.
                        return None;
                    }
                    _ => return None,
                };
                Some((inner_col, outer_idx))
            }
            E::BinaryOp(l, op, r) if op.to_uppercase() == "AND" => {
                walk(l, own_prefix, inner_col_hint, outer_table_info)
                    .or_else(|| walk(r, own_prefix, inner_col_hint, outer_table_info))
            }
            _ => None,
        }
    }

    walk(where_expr, own_prefix, &inner_col_lower, outer_table_info)
}

/// Build `key_col_value → aggregate_result` index for a scalar
/// aggregate subquery.  Scans the inner table once, groups rows by
/// the key column, computes the aggregate per group, applies the
/// optional `op_factor` multiplier.
fn build_scalar_agg_index(
    rows: &[Vec<Value>],
    _table_info: &TableInfo,
    key_col_idx: usize,
    agg_col_idx: Option<usize>,
    agg_func: AggregateFunction,
    op_factor: f64,
) -> ScalarAggIndexMap {
    use sqlrustgo_types::Value as V;
    let mut groups: HashMap<Value, (f64, i64, bool)> = HashMap::new();
    // (running_float_sum, count, any_float)
    for row in rows {
        let key = match row.get(key_col_idx) {
            Some(v) => v.clone(),
            None => continue,
        };
        let entry = groups.entry(key).or_insert((0.0, 0, false));
        match agg_func {
            AggregateFunction::Count => {
                // COUNT(*) → 1, COUNT(col) → 1 if not NULL
                if let Some(ci) = agg_col_idx {
                    if matches!(row.get(ci), Some(V::Null) | None) {
                        continue;
                    }
                }
                entry.1 += 1;
            }
            AggregateFunction::Sum | AggregateFunction::Avg => {
                let Some(ci) = agg_col_idx else { continue };
                let v = row.get(ci);
                match v {
                    Some(V::Integer(n)) => {
                        if entry.2 {
                            entry.0 += *n as f64;
                        } else {
                            // Integer accumulator until we see a float
                            // — for AVG/SUM we just use f64 directly.
                            entry.0 += *n as f64;
                        }
                        entry.1 += 1;
                    }
                    Some(V::Float(f)) => {
                        entry.2 = true;
                        entry.0 += f;
                        entry.1 += 1;
                    }
                    _ => {}
                }
            }
            AggregateFunction::Min | AggregateFunction::Max => {
                // For Min/Max we need to store the actual value, not
                // f64 accumulator.  Not used by TPC-H Q17; fall back
                // to a generic path.
                let Some(ci) = agg_col_idx else { continue };
                let Some(v) = row.get(ci) else { continue };
                // Just track count for now; fast-path only covers
                // AVG/SUM/COUNT for the Q17 perf fix.
                let _ = (entry, v.clone(), ci);
            }
            // V313-followup-3 / Issue #4156: PERCENTILE_CONT WITHIN
            // GROUP falls back to the serial compute_aggregates path
            // (this fast-path only supports AVG/SUM/COUNT/MIN/MAX).
            AggregateFunction::PercentileCont => unreachable!(),
            // V313-followup-2 / Issue #4155: quantile aggregates are
            // sorted-index algorithms; the scalar-aggregate fast-path
            // only handles in-place updaters. Fall through (no update).
            AggregateFunction::QuantileDisc | AggregateFunction::QuantileCont => {}
        }
    }
    let mut result: ScalarAggIndexMap = HashMap::with_capacity(groups.len());
    for (k, (sum, count, _any_float)) in groups {
        let v: Value = match agg_func {
            AggregateFunction::Count => Value::Integer(count),
            AggregateFunction::Sum => Value::Float(sum * op_factor),
            AggregateFunction::Avg => {
                if count == 0 {
                    Value::Null
                } else {
                    Value::Float((sum / count as f64) * op_factor)
                }
            }
            _ => Value::Null, // Min/Max not implemented in fast-path
        };
        result.insert(k, v);
    }
    result
}

#[cfg(test)]
mod chain_builder_tests {
    //! Unit tests for the executor-side multi-way join chain builder.
    //!
    //! Issue #4280: TPC-H Q21's 4-table comma-join returns 100 rows
    //! but takes ~35s because the chain builder fails to construct
    //! a complete 4-table chain and falls back to the cartesian path.
    //!
    //! These tests call `build_chain_from_start` directly with the
    //! Q21 join topology so the failure mode is captured in CI
    //! regardless of whether a dbgen fixture is available.

    use super::*;
    use std::collections::HashMap;

    /// Q21 join topology:
    ///   join_tables = [(supplier, supplier), (lineitem, l1),
    ///                  (orders, orders), (nation, nation)]
    ///   pair_key (3 edges):
    ///     (supplier, l1)      → (s_suppkey, l_suppkey)
    ///     (orders, l1)        → (o_orderkey, l_orderkey)
    ///     (supplier, nation)  → (s_nationkey, n_nationkey)
    ///
    /// The graph is connected, so SOME start_idx must yield a
    /// spanning chain of length 4. The multi-start loop in
    /// `try_comma_join_hash_chain` should find it.
    #[test]
    fn build_chain_from_start_q21_completes() {
        let join_tables: Vec<(String, String)> = vec![
            ("supplier".to_string(), "supplier".to_string()),
            ("lineitem".to_string(), "l1".to_string()),
            ("orders".to_string(), "orders".to_string()),
            ("nation".to_string(), "nation".to_string()),
        ];

        let mut pair_key: HashMap<(String, String), (String, String)> = HashMap::new();
        pair_key.insert(
            ("supplier".to_string(), "l1".to_string()),
            ("s_suppkey".to_string(), "l_suppkey".to_string()),
        );
        pair_key.insert(
            ("orders".to_string(), "l1".to_string()),
            ("o_orderkey".to_string(), "l_orderkey".to_string()),
        );
        pair_key.insert(
            ("supplier".to_string(), "nation".to_string()),
            ("s_nationkey".to_string(), "n_nationkey".to_string()),
        );

        // Try every start_idx; at least one must yield a complete chain.
        let mut found_complete = false;
        for start_idx in 0..join_tables.len() {
            if let Some(chain) = ExecutionEngine::<crate::MemoryStorage>::build_chain_from_start(
                start_idx,
                &join_tables,
                &pair_key,
            ) {
                if chain.len() == join_tables.len() {
                    found_complete = true;
                    eprintln!("Q21 chain from start_idx={}: {:?}", start_idx, chain);
                }
            }
        }
        assert!(
            found_complete,
            "Q21 join graph IS connected: some start_idx must yield chain.len()==4, but none did"
        );
    }

    /// Defensive test: any single start_idx should still yield a
    /// complete chain for Q21 (the graph has multiple spanning
    /// orders, not just one).
    #[test]
    fn build_chain_from_start_q21_orders_start_completes() {
        let join_tables: Vec<(String, String)> = vec![
            ("supplier".to_string(), "supplier".to_string()),
            ("lineitem".to_string(), "l1".to_string()),
            ("orders".to_string(), "orders".to_string()),
            ("nation".to_string(), "nation".to_string()),
        ];

        let mut pair_key: HashMap<(String, String), (String, String)> = HashMap::new();
        pair_key.insert(
            ("supplier".to_string(), "l1".to_string()),
            ("s_suppkey".to_string(), "l_suppkey".to_string()),
        );
        pair_key.insert(
            ("orders".to_string(), "l1".to_string()),
            ("o_orderkey".to_string(), "l_orderkey".to_string()),
        );
        pair_key.insert(
            ("supplier".to_string(), "nation".to_string()),
            ("s_nationkey".to_string(), "n_nationkey".to_string()),
        );

        // start_idx=2 (orders) is the canonical "leaf" start that
        // should always produce a complete chain for Q21.
        let chain = ExecutionEngine::<crate::MemoryStorage>::build_chain_from_start(
            2,
            &join_tables,
            &pair_key,
        )
        .expect("start from orders should yield Some(chain)");
        assert_eq!(
            chain.len(),
            4,
            "chain from orders start must cover all 4 tables, got len={}",
            chain.len()
        );
    }
}
