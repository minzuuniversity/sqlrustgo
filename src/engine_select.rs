//! Engine SELECT execution — extracted from execution_engine.rs (PR-900)
//!
//! Handles SELECT statement dispatch, projection, join planning, and result assembly.

use crate::engine_utils::*;
use crate::expr_utils::*;
use crate::{ExecutionEngine, ExecutorResult, SqlError, SqlResult, Value};
use sqlrustgo_executor::parallel_executor::{
    ParallelExecutor, ParallelVolcanoExecutor, PARALLEL_MIN_ROWS,
};
use sqlrustgo_parser::{
    get_and_clear_derived_subqueries, AggregateCall, AggregateFunction, Expression,
    JoinClause as ParserJoinClause, JoinType, SelectStatement,
};
use sqlrustgo_storage::{StorageEngine, TableInfo};
use std::cell::RefCell;
use std::collections::HashMap;

type DerivedResult = (Vec<Vec<Value>>, TableInfo);

// Phase 3 (TPCH-01 Q15): thread-local registry of materialized
// derived subquery results. Populated by `execute_joins` before the
// join chain runs; consumed by `execute_single_join` when it encounters
// a `__subq_N` synthetic table name.
thread_local! {
    static DERIVED_RESULTS: RefCell<HashMap<String, DerivedResult>> =
        RefCell::new(HashMap::new());
}

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    pub(crate) fn execute_select(&self, select: &SelectStatement) -> SqlResult<ExecutorResult> {
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
                        });
                }
                Some((sub_result.rows, table_info))
            } else {
                None
            };

        let storage = self.storage.read().unwrap();

        // Step 1: FROM/JOIN - get initial rows and schema
        let (mut rows, table_info) = if !select.join_clause.is_empty() {
            self.execute_joins(select)?
        } else if let Some((rows, info)) = materialized {
            (rows, info)
        } else if select.table.is_empty() {
            let empty_schema = TableInfo {
                name: String::new(),
                columns: Vec::new(),
                foreign_keys: Vec::new(),
                unique_constraints: Vec::new(),
                check_constraints: Vec::new(),
                partition_info: None,
            };
            (vec![Vec::new()], empty_schema)
        } else {
            let rows = storage.scan(&select.table)?;
            let table_info = storage.get_table_info(&select.table)?;
            (rows, table_info)
        };
        // Drop the storage read lock before running any per-row
        // correlated-subquery evaluations, since those recursive
        // `self.execute_select` calls would deadlock against a
        // held read lock. We still hold `&self` for engine access.
        drop(storage);

        if self.parallel_degree > 1
            && rows.len() >= PARALLEL_MIN_ROWS
        {
            if let Some(ref where_expr) = select.where_clause {
                let parallel = ParallelVolcanoExecutor::new(self.parallel_degree);
                let partitions = parallel.partition_scan(rows, self.parallel_degree);
                rows = self.filter_partitions_parallel(
                    partitions,
                    where_expr,
                    &table_info,
                );
            }
        }

        // Step 1.5: correlated EXISTS / NOT EXISTS pre-evaluation
        // TPC-H Q20/Q21: WHERE contains `EXISTS (subq WHERE x = outer.col)`.
        // Before applying WHERE row-by-row, substitute the outer column
        // references in the subquery with concrete values from each
        // outer row, then execute the subquery and check whether it
        // returned any rows. Replace the EXISTS/NotExists subtree in
        // the cloned where_expr with a Literal(true/false) for that
        // specific outer row. The remaining WHERE logic then runs via
        // the standard `eval_predicate` path.
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
                        new_rows.push(row);
                    }
                }
                rows = new_rows;
            } else {
                rows.retain(|row| eval_predicate(where_expr, row, &table_info));
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
                    select
                        .columns
                        .iter()
                        .map(|col| {
                            match &col.expression {
                                Some(expr) => evaluate_expression(
                                    expr,
                                    &agg_values,
                                    &agg_schema,
                                )
                                .unwrap_or(Value::Null),
                                None => agg_values.first().cloned().unwrap_or(Value::Null),
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .map(|v| vec![v])
                        .collect()
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
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\x00");
                            subtotal_groups.entry(key).or_default().push(row.clone());
                        }
                        for (key, _group_rows) in subtotal_groups.iter() {
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
                let agg_result_rows = if !select.order_by.is_empty() {
                    let mut keyed: Vec<(Vec<Value>, Vec<Value>)> = agg_result_rows
                        .into_iter()
                        .map(|row| {
                            let keys: Vec<Value> = select
                                .order_by
                                .iter()
                                .map(|ob_expr| {
                                    // ORDER BY column reference. For aggregate
                                    // path, the row is [group_key..., agg_value...].
                                    // Try alias-or-name in select.columns first.
                                    if let Expression::Identifier(col_name) = &ob_expr.expression {
                                        if let Some(idx) = select.columns.iter().position(|c| {
                                            c.alias.as_deref() == Some(col_name)
                                                || c.name == *col_name
                                        }) {
                                            if idx < row.len() {
                                                return row[idx].clone();
                                            }
                                        }
                                    }
                                    Value::Null
                                })
                                .collect();
                            (keys, row)
                        })
                        .collect();
                    keyed.sort_by(|a, b| a.0.cmp(&b.0));
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
        }; // Step 5: SELECT projection — apply each `select.columns` expression
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
        let is_star = select.columns.is_empty() || select.columns.iter().any(|c| c.name == "*");
        let projected_with_names: (Vec<String>, Vec<Vec<Value>>) = if is_star {
            let names: Vec<String> = if !table_info.columns.is_empty()
                && table_info.columns.len() == limited_rows.first().map(|r| r.len()).unwrap_or(0)
            {
                table_info.columns.iter().map(|c| c.name.clone()).collect()
            } else {
                (1..=limited_rows.first().map(|r| r.len()).unwrap_or(0))
                    .map(|i| format!("c{}", i))
                    .collect()
            };
            (names, limited_rows)
        } else {
            let names: Vec<String> = select
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect();
            let rows: Vec<Vec<Value>> = limited_rows
                .into_iter()
                .map(|row| {
                    select
                        .columns
                        .iter()
                        .map(|col| match &col.expression {
                            Some(expr) => {
                                evaluate_expression(expr, &row, &table_info).unwrap_or(Value::Null)
                            }
                            None => row.first().cloned().unwrap_or(Value::Null),
                        })
                        .collect()
                })
                .collect();
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
        let projected_rows: Vec<Vec<Value>> = if !select.order_by.is_empty() {
            let mut keyed: Vec<(Vec<Value>, Vec<Value>)> = projected_rows
                .into_iter()
                .map(|row| {
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
                                    // Fallback: try the underlying table's columns.
                                    if let Some(idx) =
                                        table_info.columns.iter().position(|c| c.name == *col_name)
                                    {
                                        if idx < row.len() {
                                            return row[idx].clone();
                                        }
                                    }
                                    Value::Null
                                }
                                Expression::Literal(lit_str) => {
                                    // Try parsing as positional integer (1-based).
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
                    (keys, row)
                })
                .collect();
            // Sort. Each order_by has an `ascending` flag;
            // v3.8.0-rc2 Day 7: respect ASC/DESC. Q13 uses
            // DESC, which my earlier version ignored.
            keyed.sort_by(|a, b| {
                for (i, ob) in select.order_by.iter().enumerate() {
                    let ord = if i < a.0.len() && i < b.0.len() {
                        a.0[i].cmp(&b.0[i])
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
            keyed.into_iter().map(|(_, row)| row).collect()
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
                    if values.is_empty() {
                        // SQL standard: SUM over empty set is NULL.
                        // PG returns 0 rows, others return 1 row with NULL.
                        // We follow the standard (NULL), not PG's quirk.
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

    /// Execute a chain of JOINs: start from the base table, then apply each
    /// JoinClause in order (left-associative: t1 JOIN t2 JOIN t3 → ((t1 JOIN t2) JOIN t3)).
    /// This function only generates joined rows, does NOT apply WHERE/AGG/HAVING.
    fn execute_joins(&self, select: &SelectStatement) -> SqlResult<(Vec<Vec<Value>>, TableInfo)> {
        let storage = self.storage.read().unwrap();

        // Seed with the base table from FROM clause. If the FROM has an
        // alias (`FROM t a`), prefix the columns with the alias so the
        // alias is queryable in subsequent JOIN ON conditions.
        let mut rows = storage.scan(&select.table)?;
        let raw_info = storage.get_table_info(&select.table)?;
        let base_prefix = select.from_alias.as_ref().unwrap_or(&select.table);
        let mut table_info = if select.from_alias.is_some() {
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
                        });
                }
                DERIVED_RESULTS.with(|cell| {
                    cell.borrow_mut()
                        .insert(name.clone(), (sub_result.rows, table_info));
                });
            }
        }

        for join_clause in &select.join_clause {
            let (new_rows, new_info) =
                self.execute_single_join(&rows, &table_info, join_clause, &storage)?;
            rows = new_rows;
            table_info = new_info;
        }

        Ok((rows, table_info))
    }

    /// Execute a single JOIN against an existing (left) row set + schema.
    /// `join_clause` is consumed separately so callers can iterate a Vec<JoinClause>.
    fn execute_single_join(
        &self,
        left_rows: &[Vec<Value>],
        left_table_info: &TableInfo,
        join_clause: &ParserJoinClause,
        storage: &S,
    ) -> SqlResult<(Vec<Vec<Value>>, TableInfo)> {
        use sqlrustgo_parser::JoinType as ParserJoinType;
        use std::collections::HashMap;

        let right_table_name = join_clause.table.clone();
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
        let right_rows = right_raw_rows;
        let mut right_table_info = right_raw_info.clone();
        if join_clause.alias.is_some() {
            right_table_info.name = right_alias.clone();
            for col in &mut right_table_info.columns {
                col.name = format!("{}.{}", right_alias, col.name);
            }
        }

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
                let combined_schema = build_combined_schema(
                    &left_table_info,
                    &left_alias,
                    &right_table_info,
                    right_alias,
                )?;
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
                let mut right_hash: HashMap<String, Vec<Vec<Value>>> = HashMap::new();
                for right_row in &right_rows {
                    let key = match key_of(right_row, &right_key_indices) {
                        Some(k) => k,
                        None => continue,
                    };
                    right_hash.entry(key).or_default().push(right_row.clone());
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
        // eprintln!("DBG find_join_key_index: left={} right={} expr={:?}", left_name, right_name, expr);
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
                        if let Some(idx) =
                            lookup_qualified_column(left_info, qualifier, col_name)
                        {
                            return Ok(JoinKey::Left(idx));
                        }
                        if let Some(idx) =
                            lookup_qualified_column(right_info, qualifier, col_name)
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
        if c.name == col_name {
            return true;
        }
        // The accumulated column name may have one or more `.`-prefix
        // segments. Find the final segment and compare.
        if let Some((_, suffix)) = c.name.rsplit_once('.') {
            if suffix == bare {
                return true;
            }
        }
        if c.name == bare {
            return true;
        }
        false
    })
}

fn lookup_qualified_column(
    info: &TableInfo,
    qualifier: &str,
    col_name: &str,
) -> Option<usize> {
    let needle = format!("{qualifier}.{col_name}");
    let suffix = format!(".{qualifier}.{col_name}");
    info.columns.iter().position(|c| {
        c.name == needle || c.name.ends_with(&suffix)
    })
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
        _ => Value::Null,
    }
}

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    fn filter_partitions_parallel(
        &self,
        partitions: Vec<Vec<Vec<Value>>>,
        where_expr: &Expression,
        table_info: &TableInfo,
    ) -> Vec<Vec<Value>> {
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
                    subquery_indexes
                        .get(*cursor)
                        .and_then(|idx| self.pre_eval_exists_indexed(wc, idx))
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
            Expression::NotExists(subq) => {
                let substituted =
                    substitute_outer_refs_in_select(subq, outer_row, outer_table_info);
                let indexed = if let Some(wc) = substituted.where_clause.as_ref() {
                    subquery_indexes
                        .get(*cursor)
                        .and_then(|idx| self.pre_eval_exists_indexed(wc, idx))
                        .map(|any| !any)
                } else {
                    None
                };
                if let Some(zero_rows) = indexed {
                    *cursor += 1;
                    return Expression::Literal(if zero_rows { "true" } else { "false" }.to_string());
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
            Expression::IsNull(inner) => Expression::IsNull(Box::new(
                self.pre_evaluate_correlated_exists(
                    inner,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                ),
            )),
            Expression::IsNotNull(inner) => Expression::IsNotNull(Box::new(
                self.pre_evaluate_correlated_exists(
                    inner,
                    outer_row,
                    outer_table_info,
                    subquery_indexes,
                    cursor,
                ),
            )),
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
            // CASE WHEN / SubqueryField pass through.
            Expression::Subquery(_) | Expression::SubqueryField(_, _) | Expression::CaseWhen(_, _) => {
                where_expr.clone()
            }
            // QuantifiedOp: pass through.
            Expression::QuantifiedOp(_, _, _) => where_expr.clone(),
            // Terminal expressions and aggregates pass through (no
            // possible subquery subtrees).
            Expression::Literal(_)
            | Expression::Identifier(_)
            | Expression::Aggregate(_)
            | Expression::WindowCall(_) => where_expr.clone(),
        }
    }

    /// TPC-H Q20/Q21: fast-path EXISTS / NOT EXISTS subquery
    /// evaluation. Detects the common pattern
    ///   `EXISTS (SELECT * FROM <single_table> WHERE <predicate>)`
    /// and evaluates it with a direct storage scan + WHERE filter
    /// + early exit (returns `Some(true)` as soon as a matching
    /// row is found; `Some(false)` if the scan finishes with
    /// zero matches). Returns `None` for any pattern that does
    /// not match the fast-path shape (e.g. JOINs, GROUP BY,
    /// multiple tables, or sub-subqueries); the caller then
    /// falls back to the full `self.execute_select` pipeline.
    fn pre_eval_exists_subquery_fast(
        &self,
        subq: &sqlrustgo_parser::SelectStatement,
        outer_row: &[Value],
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
        // Direct storage scan + WHERE filter + early exit.
        let storage = self.storage.read().ok()?;
        let rows = storage.scan(&subq.table).ok()?;
        let table_info = storage.get_table_info(&subq.table).ok()?;
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
        use sqlrustgo_parser::Expression as E;
        use sqlrustgo_storage::StorageEngine;

        if !subq.join_clause.is_empty() || subq.from_subquery.is_some() {
            return None;
        }
        if subq.table.is_empty() {
            return None;
        }
        let where_expr = subq.where_clause.as_ref()?;
        let storage = self.storage.read().ok()?;
        let table_info = storage.get_table_info(&subq.table).ok()?;
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
        let col_idx = table_info
            .columns
            .iter()
            .position(|c| c.name == col_name)?;
        let rows = storage.scan(&subq.table).ok()?;
        // Evaluate the STATIC predicate (not the full WHERE) per
        // inner row.  The outer-equality leaf references the outer
        // table's column, which is not a lineitem column, so the
        // full-WHERE evaluation would always return false on
        // lineitem rows and the index would always be empty.
        let mut qualifying: std::collections::HashSet<Value> =
            std::collections::HashSet::with_capacity(rows.len());
        for row in &rows {
            if eval_predicate(&static_predicate, row, &table_info) {
                qualifying.insert(row[col_idx].clone());
            }
        }
        Some(SubqueryIndex {
            col_idx,
            qualifying_keys: qualifying,
        })
    }

    /// Indexed fast-path EXISTS check: looks up the substituted
    /// equality key in the pre-built `SubqueryIndex.qualifying_keys`
    /// and returns whether the membership holds.  Returns `None` if
    /// the WHERE has no simple equality leaf (caller falls back to
    /// the per-row full-scan).
    fn pre_eval_exists_indexed(
        &self,
        where_expr: &Expression,
        index: &SubqueryIndex,
    ) -> Option<bool> {
        let lit = find_top_level_equality_literal(where_expr, index.col_idx)?;
        Some(index.qualifying_keys.contains(&lit))
    }
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
/// Complexity: O(N_inner) one-time, O(1) per outer row. Total
/// O(N_inner + N_outer) vs the prior O(N_inner × N_outer) full scan.
#[derive(Debug, Clone)]
pub struct SubqueryIndex {
    pub col_idx: usize,
    pub qualifying_keys: std::collections::HashSet<Value>,
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
    fn is_inner_col(name: &str, info: &TableInfo) -> bool {
        info.columns.iter().any(|c| c.name == name)
    }
    match where_expr {
        E::BinaryOp(l, op, r) if op.to_uppercase() == "AND" => {
            if let Some(split) = split_outer_equality_with_table(l, inner_info) {
                let new_rest = E::BinaryOp(
                    Box::new(*r.clone()),
                    op.clone(),
                    split.1,
                );
                return Some((split.0, Box::new(new_rest)));
            }
            if let Some(split) = split_outer_equality_with_table(r, inner_info) {
                let new_rest = E::BinaryOp(
                    Box::new(*l.clone()),
                    op.clone(),
                    split.1,
                );
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
                    if is_inner_col(&name, inner_info)
                        && !is_inner_col(&outer, inner_info) =>
                {
                    Some((name, Box::new(E::Literal("true".into()))))
                }
                (Some(outer), Some(name))
                    if is_inner_col(&name, inner_info)
                        && !is_inner_col(&outer, inner_info) =>
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
                let new_rest = E::BinaryOp(
                    Box::new(*r.clone()),
                    op.clone(),
                    split.1,
                );
                return Some((split.0, Box::new(new_rest)));
            }
            if let Some(split) = split_outer_equality(r) {
                let new_rest = E::BinaryOp(
                    Box::new(*l.clone()),
                    op.clone(),
                    split.1,
                );
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
fn find_top_level_equality_literal(
    where_expr: &Expression,
    _key_col_idx: usize,
) -> Option<Value> {
    use sqlrustgo_parser::Expression as E;
    fn walk(e: &Expression) -> Option<Value> {
        match e {
            E::BinaryOp(l, op, r) if op == "=" => match (l.as_ref(), r.as_ref()) {
                (E::Identifier(_), _) => literal_value_of(r),
                (_, E::Identifier(_)) => literal_value_of(l),
                _ => None,
            },
            E::BinaryOp(l, op, r) if op.to_uppercase() == "AND" => {
                walk(l).or_else(|| walk(r))
            }
            _ => None,
        }
    }
    walk(where_expr)
}
