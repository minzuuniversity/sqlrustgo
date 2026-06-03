//! CBO (Cost-Based Optimizer) Estimator
//!
//! Extracted from `execution_engine.rs` in SPEC-012 (PR-900 split) to reduce
//! `execution_engine.rs` line count from 1587 to <1500.
//!
//! All methods are pure functions of `(stats, table_name, ...)`. They depend
//! only on the `ExecutionStats` shared state (via `Arc<RwLock<>>`).
//!
//! Original methods were on `impl ExecutionEngine`. To preserve the public API,
//! `ExecutionEngine` retains thin forwarder methods that delegate here.

use sqlrustgo_types::SqlResult;
use std::sync::{Arc, RwLock};

use super::execution_engine::{ExecutionStats, TableStatistics};

/// Estimate row count for a table based on statistics
pub fn estimate_row_count(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str) -> u64 {
    let stats = stats.read().unwrap();
    stats
        .table_stats
        .get(table_name)
        .map(|s| s.row_count)
        .unwrap_or(1000) // Default estimate
}

/// Estimate selectivity of a predicate based on column statistics.
/// Returns a value between 0.0 and 1.0.
pub fn estimate_selectivity(
    stats: &Arc<RwLock<ExecutionStats>>,
    table_name: &str,
    column_name: &str,
) -> f64 {
    let stats = stats.read().unwrap();
    if let Some(table_stats) = stats.table_stats.get(table_name) {
        if let Some(col_stats) = table_stats.column_stats.get(column_name) {
            if col_stats.distinct_count > 0 {
                return 1.0 / col_stats.distinct_count as f64;
            }
        }
    }
    0.1 // Default: assume 10% selectivity
}

/// Estimate the cost of a sequential scan
pub fn estimate_seq_scan_cost(stats: &Arc<RwLock<ExecutionStats>>, table_name: &str) -> f64 {
    let rows = estimate_row_count(stats, table_name);
    rows as f64 * 1.0 // Each row has unit cost
}

/// Estimate the cost of an index scan
/// selectivity: fraction of rows that match the predicate
pub fn estimate_index_scan_cost(
    stats: &Arc<RwLock<ExecutionStats>>,
    table_name: &str,
    selectivity: f64,
) -> f64 {
    let rows = estimate_row_count(stats, table_name);
    // Index scan cost = index lookup cost + random I/O for matching rows
    let index_lookup_cost = 10.0; // Fixed overhead for index access
    let random_io_cost = (rows as f64 * selectivity) * 0.5; // Random I/O per match
    index_lookup_cost + random_io_cost
}

/// Estimate the benefit (cost reduction) of using an index vs sequential scan
/// Returns positive value if index is beneficial, negative if sequential scan is better
pub fn estimate_index_benefit(
    stats: &Arc<RwLock<ExecutionStats>>,
    table_name: &str,
    selectivity: f64,
) -> f64 {
    let seq_cost = estimate_seq_scan_cost(stats, table_name);
    let index_cost = estimate_index_scan_cost(stats, table_name, selectivity);
    seq_cost - index_cost
}

/// Decide whether to use index scan or sequential scan based on cost estimation
/// Returns true if index scan is recommended
pub fn should_use_index(
    stats: &Arc<RwLock<ExecutionStats>>,
    table_name: &str,
    column_name: &str,
) -> bool {
    let selectivity = estimate_selectivity(stats, table_name, column_name);
    let benefit = estimate_index_benefit(stats, table_name, selectivity);
    benefit > 0.0
}

/// Estimate the cost of a join between two tables
/// join_type: "hash", "nested_loop", "merge"
pub fn estimate_join_cost(
    stats: &Arc<RwLock<ExecutionStats>>,
    left_table: &str,
    right_table: &str,
    join_type: &str,
) -> f64 {
    let left_rows = estimate_row_count(stats, left_table);
    let right_rows = estimate_row_count(stats, right_table);

    match join_type {
        "hash" => {
            // Hash join cost = build + probe
            let build_cost = right_rows as f64 * 0.8;
            let probe_cost = left_rows as f64 * 0.8;
            build_cost + probe_cost
        }
        "merge" => {
            // Merge join cost = sort + merge
            let left_sort = left_rows as f64 * 0.5 * (left_rows as f64).log2();
            let right_sort = right_rows as f64 * 0.5 * (right_rows as f64).log2();
            left_sort + right_sort + (left_rows + right_rows) as f64 * 0.1
        }
        _ => {
            // Nested loop: outer * inner
            let outer_cost = left_rows as f64;
            let inner_cost = right_rows as f64 * 0.1; // Assuming index on inner
            outer_cost + outer_cost * inner_cost
        }
    }
}

/// Find the optimal join order using a greedy algorithm
/// Returns tables in optimal join order (smallest first)
pub fn optimize_join_order<'a>(
    stats: &Arc<RwLock<ExecutionStats>>,
    tables: &'a [&str],
) -> Vec<&'a str> {
    if tables.len() <= 1 {
        return tables.to_vec();
    }

    let mut remaining: Vec<&str> = tables.to_vec();
    let mut result: Vec<&str> = Vec::new();

    while !remaining.is_empty() {
        let candidate = if result.is_empty() {
            remaining
                .iter()
                .min_by(|a, b| estimate_row_count(stats, a).cmp(&estimate_row_count(stats, b)))
                .copied()
        } else {
            remaining
                .iter()
                .min_by(|a, b| {
                    let cost_a = estimate_join_cost(stats, result.last().unwrap(), a, "hash");
                    let cost_b = estimate_join_cost(stats, result.last().unwrap(), b, "hash");
                    cost_a.partial_cmp(&cost_b).unwrap()
                })
                .copied()
        };

        if let Some(t) = candidate {
            result.push(t);
            remaining.retain(|x| *x != t);
        } else {
            break;
        }
    }

    result
}

/// Collect statistics for a table (ANALYZE)
pub fn collect_table_stats<S: sqlrustgo_storage::StorageEngine>(
    engine: &S,
    table: &str,
) -> SqlResult<TableStatistics> {
    let rows = engine.scan(table)?;
    let row_count = rows.len() as u64;

    let table_info = engine.get_table_info(table)?;
    let column_names: Vec<String> = table_info
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect();

    use sqlrustgo_types::Value;
    let mut column_stats = std::collections::HashMap::new();
    if !rows.is_empty() {
        let num_cols = rows[0].len();
        for col_idx in 0..num_cols {
            let col_name = column_names.get(col_idx).cloned().unwrap_or_else(|| format!("col_{}", col_idx));
            let mut distinct_values = std::collections::HashSet::new();
            let mut null_count = 0u64;
            for row in &rows {
                if let Some(v) = row.get(col_idx) {
                    if matches!(v, Value::Null) {
                        null_count += 1;
                    } else {
                        distinct_values.insert(format!("{:?}", v));
                    }
                }
            }
            column_stats.insert(
                col_name,
                super::execution_engine::ColumnStatistics {
                    distinct_count: distinct_values.len() as u64,
                    null_count,
                    min_value: None,
                    max_value: None,
                },
            );
        }
    }

    Ok(TableStatistics {
        row_count,
        column_stats,
    })
}
