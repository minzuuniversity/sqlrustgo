//! DML executors extracted from `execution_engine.rs`.
//!
//! Each public function in this module is a free function that takes
//! `&mut ExecutionEngine<S>` and mirrors the legacy associated function.
//! The thin `pub fn execute_*` wrappers in `execution_engine.rs` invoke
//! these (and emit the ARCH-3 VtuGuard marker call as the first statement
//! — required by `check_arch3_no_bypass.sh`).
//!
//! Part of the AD-001 / PR-900 file split (issue #3661).

use sqlrustgo_executor::trigger::{
    TriggerEvent as ExecTriggerEvent, TriggerExecutor, TriggerTiming as ExecTriggerTiming,
};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{DeleteStatement, InsertStatement, UpdateStatement};
use sqlrustgo_parser::Expression;
use sqlrustgo_storage::{StorageEngine, TableInfo};
use sqlrustgo_types::Value;

use crate::engine_helpers::{
    apply_odku, apply_set_clauses, build_insert_records, ir_validate_update_filter,
    map_select_result_to_records, record_matches_unique_key, run_before_update_triggers,
};
use crate::engine_utils::{
    build_multi_table_combined_schema, cartesian_product, evaluate_where_clause, find_column_index,
    validate_foreign_keys,
};
use crate::expr_utils::{evaluate_expression, resolve_subqueries_in_expr};
use crate::{ExecutionEngine, SqlError, SqlResult};

/// INSERT executor body. ARCH-3 VtuGuard call lives in the `pub fn
/// execute_insert` wrapper in `execution_engine.rs` (gate requirement).
pub fn execute_insert<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    insert: &InsertStatement,
) -> SqlResult<ExecutorResult> {
    let (_tm_tx_id, started_implicit) =
        engine.begin_implicit_dml_tx("execute_insert", &insert.table)?;
    let table_name = insert.table.clone();

    // Get table info first (need it for triggers and FK validation)
    let table_info = {
        let storage = engine.storage.read().unwrap();
        storage.get_table_info(&table_name)?.clone()
    };

    let all_records: Vec<Vec<Value>> = if let Some(ref select) = insert.select {
        let select_result = engine.execute_select(select)?;
        map_select_result_to_records(select_result, &insert.columns, &table_info)?
    } else {
        build_insert_records(&insert.values)
    };

    // For REPLACE INTO: if insert.values has a unique/key conflict, delete old row first
    if insert.is_replace {
        {
            let mut storage = engine.storage.write().unwrap();
            for record in &all_records {
                // Find existing rows with matching unique key (primary key or unique index)
                let existing_rows = storage.scan(&table_name)?;
                for existing_row in existing_rows {
                    if record_matches_unique_key(&existing_row, record, &table_info) {
                        // Delete the existing row
                        storage.delete(&table_name, &[])?;
                        break;
                    }
                }
            }
        }
    }

    // Execute BEFORE INSERT triggers
    let trigger_executor = TriggerExecutor::new(engine.storage.clone());
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
        let mut storage = engine.storage.write().unwrap();
        let col_names: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();

        if !insert.is_replace && table_info.columns.iter().any(|c| c.primary_key) {
            let existing_rows = storage.scan(&table_name)?;
            let mut odku_handled_indices: std::collections::HashSet<usize> =
                std::collections::HashSet::new();
            for (new_idx, new_record) in processed_records.iter().enumerate() {
                let mut matched = false;
                for existing in &existing_rows {
                    if record_matches_unique_key(existing, new_record, &table_info) {
                        matched = true;
                        if let Some(ref updates) = insert.on_duplicate_key_update {
                            apply_odku(&mut *storage, &table_name, &table_info, existing, updates)?;
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
    engine.commit_implicit_dml_tx(started_implicit)?;

    Ok(ExecutorResult::new(vec![], all_records.len()))
}

/// UPDATE executor body.
pub fn execute_update<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    update: &UpdateStatement,
) -> SqlResult<ExecutorResult> {
    if update.tables.is_empty() {
        return Err(SqlError::ExecutionError(
            "UPDATE requires at least one table".to_string(),
        ));
    }
    if update.tables.len() > 1 {
        return execute_update_multi_table(engine, update);
    }
    let table_name = update.tables[0].name.clone();
    let (_tm_tx_id, started_implicit) =
        engine.begin_implicit_dml_tx("execute_update", &table_name)?;

    let scalar_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
        let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
        Ok(result
            .rows
            .first()
            .and_then(|r| r.first().cloned())
            .unwrap_or(Value::Null))
    };
    let list_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
        let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
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
            resolve_subqueries_in_expr(w, &scalar_eval, &list_eval)
                .map_err(|e| SqlError::ExecutionError(format!("UPDATE WHERE subquery: {}", e)))?,
        ),
        None => None,
    };
    let resolved_update = UpdateStatement {
        tables: update.tables.clone(),
        set_clauses: resolved_set,
        where_clause: resolved_where,
    };

    // If no WHERE clause, use the simple storage.update() path
    if resolved_update.where_clause.is_none() {
        let table_info = {
            let storage = engine.storage.read().unwrap();
            storage.get_table_info(&table_name)?.clone()
        };
        let sample_row: Vec<sqlrustgo_types::Value> = {
            let storage = engine.storage.read().unwrap();
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
        let mut storage = engine.storage.write().unwrap();
        let count = storage.update(&table_name, &[], &updates)?;
        drop(storage);
        engine.commit_implicit_dml_tx(started_implicit)?;
        return Ok(ExecutorResult::new(vec![], count));
    }

    // Get table info and scan rows
    let table_info = {
        let storage = engine.storage.read().unwrap();
        storage.get_table_info(&table_name)?.clone()
    };

    let all_rows = {
        let storage = engine.storage.read().unwrap();
        storage.scan(&table_name)?
    };

    let where_clause = resolved_update.where_clause.as_ref().unwrap();

    // Filter rows that match the WHERE clause
    let rows_to_update: Vec<Vec<Value>> = all_rows
        .clone()
        .into_iter()
        .filter(|row| evaluate_where_clause(where_clause, row, &table_info))
        .collect();

    ir_validate_update_filter(&all_rows, &table_info, &rows_to_update, &resolved_update);

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
    let updated_rows = apply_set_clauses(&rows_to_update, &set_col_indices, &table_info);

    // Execute BEFORE UPDATE triggers (if any)
    let trigger_executor = TriggerExecutor::new(engine.storage.clone());
    let trigger_modified_rows = run_before_update_triggers(
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
        let mut storage = engine.storage.write().unwrap();

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
    engine.commit_implicit_dml_tx(started_implicit)?;

    Ok(ExecutorResult::new(vec![], count))
}

/// DELETE executor body.
pub fn execute_delete<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    delete: &DeleteStatement,
) -> SqlResult<ExecutorResult> {
    if delete.tables.is_empty() {
        return Err(SqlError::ExecutionError(
            "DELETE requires at least one table".to_string(),
        ));
    }
    if delete.tables.len() > 1 || delete.using.is_some() {
        return execute_delete_multi_table(engine, delete);
    }
    let table_name = delete.tables[0].name.clone();
    let (_tm_tx_id, started_implicit) =
        engine.begin_implicit_dml_tx("execute_delete", &table_name)?;

    let scalar_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
        let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
        Ok(result
            .rows
            .first()
            .and_then(|r| r.first().cloned())
            .unwrap_or(Value::Null))
    };
    let list_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
        let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
        Ok(result
            .rows
            .into_iter()
            .map(|r| r.first().cloned().unwrap_or(Value::Null))
            .collect())
    };
    let resolved_where: Option<Expression> = match &delete.where_clause {
        Some(w) => Some(
            resolve_subqueries_in_expr(w, &scalar_eval, &list_eval)
                .map_err(|e| SqlError::ExecutionError(format!("DELETE WHERE subquery: {}", e)))?,
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
        let mut storage = engine.storage.write().unwrap();
        let count = storage.delete(&table_name, &[])?;
        drop(storage);
        // INT-1: Autocommit — commit the implicit TX so WAL/MVCC see this.
        engine.commit_implicit_dml_tx(started_implicit)?;
        return Ok(ExecutorResult::new(vec![], count));
    }

    // Scan all rows from the table
    let all_rows = {
        let storage = engine.storage.read().unwrap();
        storage.scan(&table_name)?
    };

    // Get table info to find column indices
    let table_info = {
        let storage = engine.storage.read().unwrap();
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
    let trigger_executor = TriggerExecutor::new(engine.storage.clone());
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
        let storage = engine.storage.read().unwrap();
        let all_rows = storage.scan(&table_name)?;
        all_rows
            .into_iter()
            .filter(|row| !evaluate_where_clause(where_clause, row, &table_info))
            .collect()
    };

    {
        let mut storage = engine.storage.write().unwrap();
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
    engine.commit_implicit_dml_tx(started_implicit)?;

    Ok(ExecutorResult::new(vec![], count))
}

/// Multi-table UPDATE executor body (`UPDATE t1, t2 SET ... WHERE ...`).
fn execute_update_multi_table<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    update: &UpdateStatement,
) -> SqlResult<ExecutorResult> {
    let scalar_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
        let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
        Ok(result
            .rows
            .first()
            .and_then(|r| r.first().cloned())
            .unwrap_or(Value::Null))
    };
    let list_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
        let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
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
            resolve_subqueries_in_expr(w, &scalar_eval, &list_eval)
                .map_err(|e| SqlError::ExecutionError(format!("UPDATE WHERE subquery: {}", e)))?,
        ),
        None => None,
    };

    let table_refs = &update.tables;
    let mut per_table_rows: Vec<Vec<Vec<Value>>> = Vec::with_capacity(table_refs.len());
    let mut per_table_info: Vec<TableInfo> = Vec::with_capacity(table_refs.len());
    let mut per_table_prefix: Vec<String> = Vec::with_capacity(table_refs.len());
    {
        let storage = engine.storage.read().unwrap();
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

    apply_multi_table_updates(engine, table_refs, per_table_updates, total_count)
}

fn apply_multi_table_updates<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    table_refs: &[sqlrustgo_parser::TableRef],
    per_table_updates: Vec<Vec<(Vec<Value>, Vec<Value>)>>,
    total_count: usize,
) -> SqlResult<ExecutorResult> {
    let mut storage = engine.storage.write().unwrap();
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

/// Multi-table DELETE executor body (`DELETE t1, t2 FROM t1, t2 WHERE ...`).
fn execute_delete_multi_table<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    delete: &DeleteStatement,
) -> SqlResult<ExecutorResult> {
    let source_refs: Vec<sqlrustgo_parser::TableRef> = match &delete.using {
        Some(s) => s.clone(),
        None => delete.tables.clone(),
    };
    let target_refs = &delete.tables;

    let resolved_where: Option<Expression> = match &delete.where_clause {
        Some(w) => {
            let scalar_eval = |subq: &sqlrustgo_parser::SelectStatement| -> Result<Value, String> {
                let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
                Ok(result
                    .rows
                    .first()
                    .and_then(|r| r.first().cloned())
                    .unwrap_or(Value::Null))
            };
            let list_eval =
                |subq: &sqlrustgo_parser::SelectStatement| -> Result<Vec<Value>, String> {
                    let result = engine.execute_select(subq).map_err(|e| e.to_string())?;
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
    let mut per_table_info: Vec<TableInfo> = Vec::with_capacity(source_refs.len());
    let mut per_table_prefix: Vec<String> = Vec::with_capacity(source_refs.len());
    {
        let storage = engine.storage.read().unwrap();
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
    let mut storage = engine.storage.write().unwrap();
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
