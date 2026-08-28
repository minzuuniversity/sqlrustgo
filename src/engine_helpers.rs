//! Pure (no `&self`) helper functions extracted from `execution_engine.rs`.
//!
//! These helpers do not require access to `ExecutionEngine` internals — they
//! operate on the data passed in. They were previously implemented as
//! associated functions on `ExecutionEngine<S>` and have been moved here as
//! part of the AD-001 / PR-900 file split (issue #3661) to keep
//! `execution_engine.rs` under its 1500-line target.

use sqlrustgo_executor::ast_adapter::AstAdapter;
use sqlrustgo_executor::trigger::{
    TriggerEvent as ExecTriggerEvent, TriggerExecutor, TriggerTiming as ExecTriggerTiming,
};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::UpdateStatement;
use sqlrustgo_storage::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_types::Value;

use crate::expr_utils::{evaluate_expression, expression_to_value};
use crate::{SqlError, SqlResult};

/// Convert `INSERT VALUES` expression rows to materialised `Value` records.
pub fn build_insert_records(values: &[Vec<sqlrustgo_parser::Expression>]) -> Vec<Vec<Value>> {
    values
        .iter()
        .map(|row_exprs| row_exprs.iter().map(expression_to_value).collect())
        .collect()
}

/// V313-followup-1 / Issue #4154: substitute `DEFAULT` tokens
/// (sentinel `Value::Text("DEFAULT")`) with column default_value
/// (NULL when the column has no default).
pub fn materialise_default_tokens(
    records: Vec<Vec<Value>>,
    column_names: &[String],
    table_columns: &[sqlrustgo_storage::ColumnDefinition],
) -> Vec<Vec<Value>> {
    if records.is_empty() {
        return records;
    }
    let defaults: Vec<Option<&str>> = table_columns
        .iter()
        .map(|c| c.default_value.as_deref())
        .collect();
    if column_names.is_empty() {
        // INSERT VALUES with no explicit column list — align by index.
        let mut out = records;
        for row in out.iter_mut() {
            for (col_idx, default) in defaults.iter().enumerate() {
                if col_idx < row.len() {
                    let is_default = matches!(&row[col_idx], Value::Text(t) if t == "DEFAULT");
                    if is_default {
                        row[col_idx] = match default {
                            Some(s) => parse_default_literal_in_helpers(s),
                            None => Value::Null,
                        };
                    }
                }
            }
        }
        return out;
    }

    // INSERT VALUES with explicit column list — reorder columns to match
    // table_columns order. column_names[i] corresponds to values[i] in
    // the record; we need to produce a record where position j is the
    // value for table_columns[j].
    let mut ordered_records = Vec::with_capacity(records.len());
    for record in records {
        let mut ordered = Vec::with_capacity(table_columns.len());
        for (table_idx, _) in table_columns.iter().enumerate() {
            let table_col_name = &table_columns[table_idx].name;
            let default = defaults.get(table_idx).copied().unwrap_or(None);
            let source_idx = column_names.iter().position(|c| {
                c == table_col_name || c.to_lowercase() == table_col_name.to_lowercase()
            });
            let value = match source_idx {
                Some(idx) if idx < record.len() => {
                    let v = record[idx].clone();
                    if matches!(&v, Value::Text(t) if t == "DEFAULT") {
                        match default {
                            Some(s) => parse_default_literal_in_helpers(s),
                            None => Value::Null,
                        }
                    } else {
                        v
                    }
                }
                _ => match default {
                    Some(s) => parse_default_literal_in_helpers(s),
                    None => Value::Null,
                },
            };
            ordered.push(value);
        }
        ordered_records.push(ordered);
    }
    ordered_records
}

/// V313-followup-1 / Issue #4154: local copy of the storage
/// `parse_default_literal` helper because the storage crate does not
/// re-export it. Kept in sync with `crates/storage/src/engine.rs`.
fn parse_default_literal_in_helpers(s: &str) -> Value {
    use sqlrustgo_types::Value;
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("NULL") {
        return Value::Null;
    }
    if trimmed.eq_ignore_ascii_case("TRUE") {
        return Value::Boolean(true);
    }
    if trimmed.eq_ignore_ascii_case("FALSE") {
        return Value::Boolean(false);
    }
    if let Ok(n) = trimmed.parse::<i64>() {
        return Value::Integer(n);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return Value::Float(f);
    }
    let inner = if trimmed.len() >= 2 && trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    Value::Text(inner.to_string())
}

/// Materialise a SELECT result into INSERT-shaped records, coercing each
/// value to the target column's declared type.
pub fn map_select_result_to_records(
    result: ExecutorResult,
    target_columns: &[String],
    target_table_info: &TableInfo,
) -> SqlResult<Vec<Vec<Value>>> {
    let target_col_indices: Vec<usize> = if target_columns.is_empty() {
        if !result.rows.is_empty() && result.rows[0].len() != target_table_info.columns.len() {
            return Err(SqlError::ExecutionError(format!(
                "Binder Error: table {} has {} columns but {} values were supplied",
                target_table_info.name,
                target_table_info.columns.len(),
                result.rows[0].len()
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
                record.push(coerce_value_to_column(value, target_col));
            }
            Ok(record)
        })
        .collect()
}

/// Coerce a `Value` to the declared type of `target_col` for INSERT
/// type-checking. Returns the value unchanged when the types already match.
pub fn coerce_value_to_column(value: Value, target_col: &ColumnDefinition) -> Value {
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

/// Apply `ON DUPLICATE KEY UPDATE`: update the matching row in place using
/// storage.update() with PK-based filter.
pub fn apply_odku(
    storage: &mut dyn StorageEngine,
    table_name: &str,
    table_info: &TableInfo,
    existing_row: &[Value],
    updates: &[(String, sqlrustgo_parser::Expression)],
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
    if !pk_values.is_empty() {
        if !update.is_empty() {
            storage.update(table_name, &pk_values, &update)?;
        }
    } else if !update.is_empty() {
        // #4569: table has no PRIMARY KEY — the conflict was on a UNIQUE
        // constraint. Locate the conflicting row by full-row equality
        // with the snapshot taken before the update (an exact match on
        // all columns identifies exactly one row among the scanned
        // duplicates) and update it in place.
        let snapshot: Vec<Value> = existing_row.to_vec();
        let filter: sqlrustgo_storage::RowFilter =
            Box::new(move |r: &sqlrustgo_storage::Record| *r == snapshot);
        let mutation = sqlrustgo_storage::RowMutation::new(update.clone(), 0);
        storage.update_if(table_name, &filter, &mutation)?;
    }

    Ok(())
}

/// Apply UPDATE SET-clause expressions to each matching row.
pub fn apply_set_clauses(
    rows_to_update: &[Vec<Value>],
    set_col_indices: &[(usize, &sqlrustgo_parser::Expression)],
    table_info: &TableInfo,
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

/// Check if a new record matches an existing row based on primary key columns
/// or any declared UNIQUE constraint (#4569).
/// A conflict on *any* unique key (PK or UNIQUE) counts, matching MySQL
/// semantics for duplicate detection / ON DUPLICATE KEY UPDATE.
/// NULL values in UNIQUE columns never collide (SQL standard allows
/// multiple NULLs in a UNIQUE column).
pub fn record_matches_unique_key(
    existing: &[Value],
    new: &[Value],
    table_info: &TableInfo,
) -> bool {
    find_conflicting_key(existing, new, table_info).is_some()
}

/// #4569: identify which unique key (if any) the two rows conflict on.
/// Returns `"PRIMARY"` for a primary-key collision, otherwise the UNIQUE
/// constraint's declared name (falling back to `uk_<col1>_<col2>`).
pub fn find_conflicting_key(
    existing: &[Value],
    new: &[Value],
    table_info: &TableInfo,
) -> Option<String> {
    let cells_equal = |i: usize| {
        existing
            .get(i)
            .zip(new.get(i))
            .map(|(a, b)| a == b)
            .unwrap_or(false)
    };

    // Primary key columns (by definition non-NULL after NOT NULL checks).
    let pk_idx: Vec<usize> = table_info
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.primary_key)
        .map(|(i, _)| i)
        .collect();
    if !pk_idx.is_empty() && pk_idx.iter().all(|&i| cells_equal(i)) {
        return Some("PRIMARY".to_string());
    }

    for uc in &table_info.unique_constraints {
        let idxs: Vec<usize> = uc
            .columns
            .iter()
            .filter_map(|c| {
                table_info
                    .columns
                    .iter()
                    .position(|col| col.name.eq_ignore_ascii_case(c))
            })
            .collect();
        if idxs.len() != uc.columns.len() || idxs.is_empty() {
            continue;
        }
        // Skip when the new row has NULL in any unique column — multiple
        // NULLs are permitted under a UNIQUE constraint.
        if new.get(idxs[0]).is_none_or(|v| matches!(v, Value::Null)) {
            continue;
        }
        if idxs.iter().all(|&i| cells_equal(i)) {
            return Some(
                uc.name
                    .clone()
                    .unwrap_or_else(|| format!("uk_{}", uc.columns.join("_"))),
            );
        }
    }
    None
}

/// #4569: first column value of the named key for the error message
/// (`Duplicate entry '<v>' for key '<key>'`).
pub fn key_entry_repr(table_info: &TableInfo, row: &[Value], key_name: &str) -> String {
    let cols: Vec<usize> = if key_name == "PRIMARY" {
        table_info
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.primary_key)
            .map(|(i, _)| i)
            .collect()
    } else {
        table_info
            .unique_constraints
            .iter()
            .find(|uc| {
                uc.name.as_deref() == Some(key_name)
                    || format!("uk_{}", uc.columns.join("_")) == key_name
            })
            .map(|uc| {
                uc.columns
                    .iter()
                    .filter_map(|c| {
                        table_info
                            .columns
                            .iter()
                            .position(|col| col.name.eq_ignore_ascii_case(c))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    cols.iter()
        .filter_map(|&i| row.get(i))
        .map(|v| v.to_sql_string())
        .next()
        .unwrap_or_else(|| "?".to_string())
}

/// Run BEFORE UPDATE triggers; return rows transformed by triggers
/// (or unmodified if no triggers). Extracted from `execute_update`.
pub fn run_before_update_triggers(
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
        let result = trigger_executor.execute_before_update(table_name, old_row, updated_row)?;
        modified.push(result);
    }
    Ok(modified)
}

/// Cross-validate legacy WHERE filter against IR plan; warn on mismatch.
/// No-op if IR adapter cannot construct a plan.
pub fn ir_validate_update_filter(
    all_rows: &[Vec<Value>],
    table_info: &TableInfo,
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
        tracing::debug!(
            "IR predicate mismatch: legacy={}, ir={}",
            rows_to_update.len(),
            ir_filtered.len()
        );
    }
}
