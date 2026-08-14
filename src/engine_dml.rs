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
    TriggerBodyAuthCheck, TriggerEvent as ExecTriggerEvent, TriggerExecutor,
    TriggerTiming as ExecTriggerTiming,
};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{DeleteStatement, InsertStatement, UpdateStatement};
use sqlrustgo_parser::Expression;
use sqlrustgo_storage::{StorageEngine, TableInfo};

use sqlrustgo_types::Value;

use crate::engine_helpers::{
    apply_odku, apply_set_clauses, build_insert_records, ir_validate_update_filter,
    map_select_result_to_records, materialise_default_tokens, record_matches_unique_key,
    run_before_update_triggers,
};
use crate::engine_utils::{
    build_multi_table_combined_schema, cartesian_product, evaluate_where_clause, find_column_index,
    validate_foreign_keys, validate_not_null,
};
use crate::expr_utils::{evaluate_expression, resolve_subqueries_in_expr};
use crate::{ExecutionEngine, SqlError, SqlResult};
/// INSERT executor body. ARCH-3 VtuGuard call lives in the `pub fn
/// execute_insert` wrapper in `execution_engine.rs` (gate requirement).
pub fn execute_insert<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    insert: &InsertStatement,
) -> SqlResult<ExecutorResult> {
    // V311-01 F-23: ClusteredTable main-path DML routing. The SELECT path
    // already reads via ClusteredTable (engine_select.rs::scan_with_ahi);
    // here we route INSERT to ClusteredTable.insert() so the rows actually
    // land in the clustered B+ tree, not the Heap. Without this, SELECT
    // after INSERT on a CLUSTERED table returns 0 rows.
    if engine.clustered_tables.read().contains_key(&insert.table) {
        return execute_insert_clustered(engine, insert);
    }
    let (_tm_tx_id, started_implicit) =
        engine.begin_implicit_dml_tx("execute_insert", &insert.table)?;
    let table_name = insert.table.clone();

    // Get table info first (need it for triggers and FK validation)
    let table_info = {
        let storage = engine.storage.read();
        storage.get_table_info(&table_name)?.clone()
    };

    let all_records: Vec<Vec<Value>> = if let Some(ref select) = insert.select {
        let select_result = engine.execute_select(select)?;
        map_select_result_to_records(select_result, &insert.columns, &table_info)?
    } else {
        // V312-17 #3970: validate row consistency and column count
        if insert.values.len() >= 2 {
            let first_len = insert.values[0].len();
            for row in &insert.values {
                if row.len() != first_len {
                    return Err(SqlError::ExecutionError(
                        "Parser Error: VALUES lists must all be the same length".to_string(),
                    ));
                }
            }
        }
        let expected_cols = if !insert.columns.is_empty() {
            insert.columns.len()
        } else {
            table_info.columns.len()
        };
        for row in insert.values.iter() {
            if row.len() != expected_cols {
                return Err(SqlError::ExecutionError(format!(
                    "Binder Error: table {} has {} columns but {} values were supplied",
                    insert.table,
                    expected_cols,
                    row.len()
                )));
            }
        }
        materialise_default_tokens(
            build_insert_records(&insert.values),
            &insert.columns,
            &table_info.columns,
        )
    };

    // For REPLACE INTO: if insert.values has a unique/key conflict, delete old row first
    if insert.is_replace {
        {
            let mut storage = engine.storage.write();
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
    let mut trigger_executor = TriggerExecutor::new(engine.storage.clone());
    // V312-55F / Issue #4243: mirror the engine's current session user
    // and wire the catalog-backed privilege check hook so any DML inside
    // the trigger body is gated against the same identity as top-level
    // statements.
    trigger_executor.set_current_user(engine.current_user().clone());
    trigger_executor.set_auth_check(Some(build_trigger_auth_check(engine)));
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
        let mut storage = engine.storage.write();
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
                    // V311-23: INSERT IGNORE skips duplicates instead of erroring
                    if insert.is_ignore {
                        odku_handled_indices.insert(new_idx); // Mark as "handled" to skip
                        continue;
                    }
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
                                    "CHECK constraint '{}' violated: {:?}",
                                    constraint.name.as_deref().unwrap_or("unnamed"),
                                    constraint.expression
                                )
                                .into());
                            }
                        }
                    }
                    // Validate NOT NULL constraints
                    validate_not_null(&table_info, record, &insert.columns)?;
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
                                "CHECK constraint '{}' violated: {:?}",
                                constraint.name.as_deref().unwrap_or("unnamed"),
                                constraint.expression
                            )
                            .into());
                        }
                    }
                }
                // Validate NOT NULL constraints
                validate_not_null(&table_info, record, &insert.columns)?;
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
    // V311-01 F-23: ClusteredTable main-path DML routing. SELECT/INSERT
    // already use ClusteredTable; UPDATE must too or it would write to
    // the Heap and leave the clustered B+ tree stale.
    if engine
        .clustered_tables
        .read()
        .contains_key(&update.tables[0].name)
    {
        return execute_update_clustered(engine, update);
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
            let storage = engine.storage.read();
            storage.get_table_info(&table_name)?.clone()
        };
        let sample_row: Vec<sqlrustgo_types::Value> = {
            let storage = engine.storage.read();
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

        // Validate NOT NULL constraints for the update values.
        // Build a synthetic full-width row to validate. Pass empty
        // `set_col_names` so validate_not_null indexes by table column
        // position (records are full-width rows, not SET-only slices).
        // V313-12 / Issue #4040.
        let mut synthetic_row = sample_row.clone();
        for (col_idx, new_val) in &updates {
            synthetic_row[*col_idx] = new_val.clone();
        }
        validate_not_null(&table_info, &synthetic_row, &[])?;

        // V312-18 #3971: validate CHECK constraints for the synthetic updated row
        if !table_info.check_constraints.is_empty() {
            let col_names: Vec<String> =
                table_info.columns.iter().map(|c| c.name.clone()).collect();
            for constraint in &table_info.check_constraints {
                let valid = sqlrustgo_storage::evaluate_check_constraint(
                    constraint,
                    &col_names,
                    &synthetic_row,
                )?;
                if !valid {
                    return Err(format!(
                        "CHECK constraint '{}' violated: {:?}",
                        constraint.name.as_deref().unwrap_or("unnamed"),
                        constraint.expression
                    )
                    .into());
                }
            }
        }

        let mut storage = engine.storage.write();
        // V312-18 / Issue #3971: route the no-WHERE UPDATE path through
        // delete+insert so the WAL layer (which only hooks delete/insert)
        // correctly records each row update for crash recovery. The prior
        // implementation called `storage.update(&table_name, &[], ...)`
        // which bypassed WAL logging entirely — recovered values were
        // stale after a crash (ISSUE-2741 / RECOVERY-009..011 L2 runtime).
        let pk_idx = table_info
            .columns
            .iter()
            .position(|c| c.primary_key)
            .unwrap_or(0);
        let all_rows_no_where = storage.scan(&table_name)?;
        let mut count = 0usize;
        for prior_row in all_rows_no_where {
            let mut new_row = prior_row.clone();
            for (col_idx, new_val) in &updates {
                new_row[*col_idx] = new_val.clone();
            }
            let pk_val = prior_row
                .get(pk_idx)
                .cloned()
                .unwrap_or(sqlrustgo_types::Value::Null);
            storage.delete(&table_name, std::slice::from_ref(&pk_val))?;
            storage.insert(&table_name, vec![new_row])?;
            count += 1;
        }
        drop(storage);
        engine.commit_implicit_dml_tx(started_implicit)?;
        return Ok(ExecutorResult::new(vec![], count));
    }

    // Get table info and scan rows
    let table_info = {
        let storage = engine.storage.read();
        storage.get_table_info(&table_name)?.clone()
    };

    let all_rows = {
        let storage = engine.storage.read();
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
    let mut trigger_executor = TriggerExecutor::new(engine.storage.clone());
    // V312-55F / Issue #4243: mirror the engine's current session user
    // and wire the catalog-backed privilege check hook so any DML inside
    // the trigger body is gated against the same identity as top-level
    // statements.
    trigger_executor.set_current_user(engine.current_user().clone());
    trigger_executor.set_auth_check(Some(build_trigger_auth_check(engine)));
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
        let mut storage = engine.storage.write();
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
                            "CHECK constraint '{}' violated: {:?}",
                            constraint.name.as_deref().unwrap_or("unnamed"),
                            constraint.expression
                        )
                        .into());
                    }
                }
            }
        }

        // Validate NOT NULL constraints for each updated row.
        // Use empty `set_col_names` so validate_not_null indexes into `record`
        // by table column position (records are full-width rows here, not
        // SET-only slices). V313-12 / Issue #4040.
        for record in &trigger_modified_rows {
            validate_not_null(&table_info, record, &[])?;
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
    // V311-01 F-23: ClusteredTable main-path DML routing. See execute_update.
    if engine
        .clustered_tables
        .read()
        .contains_key(&delete.tables[0].name)
    {
        return execute_delete_clustered(engine, delete);
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
        let mut storage = engine.storage.write();
        let count = storage.delete(&table_name, &[])?;
        drop(storage);
        // INT-1: Autocommit — commit the implicit TX so WAL/MVCC see this.
        engine.commit_implicit_dml_tx(started_implicit)?;
        return Ok(ExecutorResult::new(vec![], count));
    }

    // Scan all rows from the table
    let all_rows = {
        let storage = engine.storage.read();
        storage.scan(&table_name)?
    };

    // Get table info to find column indices
    let table_info = {
        let storage = engine.storage.read();
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
    let mut trigger_executor = TriggerExecutor::new(engine.storage.clone());
    // V312-55F / Issue #4243: mirror the engine's current session user
    // and wire the catalog-backed privilege check hook so any DML inside
    // the trigger body is gated against the same identity as top-level
    // statements.
    trigger_executor.set_current_user(engine.current_user().clone());
    trigger_executor.set_auth_check(Some(build_trigger_auth_check(engine)));
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
        let storage = engine.storage.read();
        let all_rows = storage.scan(&table_name)?;
        all_rows
            .into_iter()
            .filter(|row| !evaluate_where_clause(where_clause, row, &table_info))
            .collect()
    };

    {
        let mut storage = engine.storage.write();
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
        let storage = engine.storage.read();
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
    let mut storage = engine.storage.write();
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
        let storage = engine.storage.read();
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
    let mut storage = engine.storage.write();
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

// =============================================================================
// V311-01 F-23: ClusteredTable DML implementations
// =============================================================================
// These three functions implement INSERT/UPDATE/DELETE for tables registered
// in `engine.clustered_tables` (i.e. tables created with
// `CREATE TABLE ... ENGINE=InnoDB CLUSTERED`). They are dispatched from the
// public `execute_insert`/`execute_update`/`execute_delete` entry points
// above when the target table is clustered. The default Heap DML path
// (and all of its trigger/FK/CHECK/ODUK/REPLACE logic) is unchanged.
//
// Known limitations in this initial main-path integration (V311-01 v2):
//   - REPLACE INTO / ON DUPLICATE KEY UPDATE are not supported on clustered
//     tables. ClusteredTable.insert() returns Err on PK collision, which
//     surfaces as a `Duplicate entry` error. ON DUPLICATE KEY UPDATE would
//     require a per-row CAS inside the clustered B+ tree; deferred to
//     v3.12+ alongside disk-backed ClusteredTable pages.
//   - FK / CHECK constraints on clustered tables are not validated here.
//     ClusteredTable v1 enforces PK uniqueness only.
//   - BEFORE / AFTER triggers are not invoked. Same reason as Heap path
//     uses the storage-backed TriggerExecutor (which can't see clustered
//     rows). Deferred to a future revision.
//   - Multi-table DML (`UPDATE t1, t2 ...`, `DELETE t1 FROM t2 ...`) is not
//     supported on clustered tables — the public entry points route these
//     to `execute_update_multi_table`/`execute_delete_multi_table` first.

/// V311-01 F-23: INSERT into a ClusteredTable.
///
/// Validates the row (PK column present, PK uniqueness — ClusteredTable
/// itself enforces uniqueness and returns Err on collision), then inserts.
/// Returns the number of rows inserted.
fn execute_insert_clustered<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    insert: &InsertStatement,
) -> SqlResult<ExecutorResult> {
    if insert.is_replace {
        return Err(SqlError::ExecutionError(
            "REPLACE INTO is not supported on CLUSTERED tables (V311-01 v2)".to_string(),
        ));
    }
    if insert.on_duplicate_key_update.is_some() {
        return Err(SqlError::ExecutionError(
            "ON DUPLICATE KEY UPDATE is not supported on CLUSTERED tables (V311-01 v2)".to_string(),
        ));
    }

    let table_name = insert.table.clone();
    let (_tm_tx_id, started_implicit) =
        engine.begin_implicit_dml_tx("execute_insert_clustered", &table_name)?;

    // Look up the ClusteredTable once (early return if the engine state
    // changed between the dispatch check and now).
    let ct_arc = {
        let map = engine.clustered_tables.read();
        map.get(&table_name)
            .ok_or_else(|| {
                SqlError::ExecutionError(format!(
                    "ClusteredTable '{}' disappeared mid-INSERT",
                    table_name
                ))
            })?
            .clone()
    };

    // Get the table info (still stored in storage for catalog).
    let table_info = {
        let storage = engine.storage.read();
        storage.get_table_info(&table_name)?.clone()
    };

    // Resolve the records to insert.
    let all_records: Vec<Vec<Value>> = if let Some(ref select) = insert.select {
        let select_result = engine.execute_select(select)?;
        map_select_result_to_records(select_result, &insert.columns, &table_info)?
    } else {
        build_insert_records(&insert.values)
    };

    // Apply CHAR(N) padding (same as Heap path).
    let processed_records: Vec<Vec<Value>> = all_records
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

    // Validate NOT NULL constraints before inserting each row.
    for record in &processed_records {
        validate_not_null(&table_info, record, &insert.columns)?;
    }

    // Insert into ClusteredTable (PK uniqueness enforced internally).
    let mut count = 0usize;
    {
        let mut ct = ct_arc.write();
        for record in &processed_records {
            ct.insert(record.clone())?;
            count += 1;
        }
    }

    engine.commit_implicit_dml_tx(started_implicit)?;
    Ok(ExecutorResult::new(vec![], count))
}

/// V311-01 F-23: UPDATE on a ClusteredTable.
///
/// Reads all rows via ClusteredTable.full_scan(), filters by WHERE, applies
/// SET clauses per row, then writes each updated row back via
/// `update_pk` (PK stays the same).
fn execute_update_clustered<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    update: &UpdateStatement,
) -> SqlResult<ExecutorResult> {
    let table_name = update.tables[0].name.clone();
    let (_tm_tx_id, started_implicit) =
        engine.begin_implicit_dml_tx("execute_update_clustered", &table_name)?;

    let ct_arc = {
        let map = engine.clustered_tables.read();
        map.get(&table_name)
            .ok_or_else(|| {
                SqlError::ExecutionError(format!(
                    "ClusteredTable '{}' disappeared mid-UPDATE",
                    table_name
                ))
            })?
            .clone()
    };

    let table_info = {
        let storage = engine.storage.read();
        storage.get_table_info(&table_name)?.clone()
    };

    // Read all rows from the clustered B+ tree.
    let all_rows: Vec<Vec<Value>> = {
        let ct = ct_arc.read();
        ct.full_scan()
    };

    // Filter rows matching WHERE clause.
    let where_clause = update.where_clause.as_ref();
    let rows_to_update: Vec<Vec<Value>> = all_rows
        .iter()
        .filter(|row| match where_clause {
            Some(w) => evaluate_where_clause(w, row, &table_info),
            None => true, // no WHERE → update all
        })
        .cloned()
        .collect();

    if rows_to_update.is_empty() {
        engine.commit_implicit_dml_tx(started_implicit)?;
        return Ok(ExecutorResult::new(vec![], 0));
    }

    // Build (col_idx, new_expr) for SET clauses.
    let set_pairs: Vec<(usize, &Expression)> = update
        .set_clauses
        .iter()
        .filter_map(|(col, expr)| find_column_index(col, &table_info).map(|i| (i, expr)))
        .collect();

    // Apply SET to each row, then write back via update_pk.
    let count = rows_to_update.len();
    let pk_idx = table_info
        .columns
        .iter()
        .position(|c| c.primary_key)
        .unwrap_or(0);

    {
        let mut ct = ct_arc.write();
        for row in &rows_to_update {
            let mut new_row = row.clone();
            for (col_idx, expr) in &set_pairs {
                let new_val = evaluate_expression(expr, row, &table_info).unwrap_or(Value::Null);
                if *col_idx < new_row.len() {
                    new_row[*col_idx] = new_val;
                }
            }
            let pk_val = row.get(pk_idx).cloned().unwrap_or(Value::Null);
            ct.update_pk(&pk_val, new_row)?;
        }
    }

    engine.commit_implicit_dml_tx(started_implicit)?;
    Ok(ExecutorResult::new(vec![], count))
}

/// V311-01 F-23: DELETE on a ClusteredTable.
///
/// Reads all rows, filters by WHERE, deletes matching rows by PK.
fn execute_delete_clustered<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    delete: &DeleteStatement,
) -> SqlResult<ExecutorResult> {
    let table_name = delete.tables[0].name.clone();
    let (_tm_tx_id, started_implicit) =
        engine.begin_implicit_dml_tx("execute_delete_clustered", &table_name)?;

    let ct_arc = {
        let map = engine.clustered_tables.read();
        map.get(&table_name)
            .ok_or_else(|| {
                SqlError::ExecutionError(format!(
                    "ClusteredTable '{}' disappeared mid-DELETE",
                    table_name
                ))
            })?
            .clone()
    };

    let table_info = {
        let storage = engine.storage.read();
        storage.get_table_info(&table_name)?.clone()
    };

    let all_rows: Vec<Vec<Value>> = {
        let ct = ct_arc.read();
        ct.full_scan()
    };

    let where_clause = delete.where_clause.as_ref();
    let pk_idx = table_info
        .columns
        .iter()
        .position(|c| c.primary_key)
        .unwrap_or(0);

    // Collect PKs to delete (filter first, then write).
    let pks_to_delete: Vec<Value> = all_rows
        .iter()
        .filter(|row| match where_clause {
            Some(w) => evaluate_where_clause(w, row, &table_info),
            None => true,
        })
        .map(|row| row.get(pk_idx).cloned().unwrap_or(Value::Null))
        .collect();

    let count = pks_to_delete.len();
    {
        let mut ct = ct_arc.write();
        for pk in &pks_to_delete {
            ct.delete_pk(pk);
        }
    }

    engine.commit_implicit_dml_tx(started_implicit)?;
    Ok(ExecutorResult::new(vec![], count))
}

// ── V312-55F / Issue #4243: trigger-body privilege hook ───────────────
//
// TriggerExecutor::check_body_privilege (defined in crates/executor/src/trigger.rs)
// calls a `TriggerBodyAuthCheck` callback before any DML inside a trigger
// body mutates storage. The engine owns the catalog (and therefore the
// authoritative AuthManager), so we build a thin adapter that forwards
// the trigger's intent ("user U wants privilege P on table T") back into
// `ExecutionEngine::check_privilege`. This keeps the trigger crate free
// of catalog imports while still enforcing fail-closed privilege checks
// against the same identity as top-level statements.
//
// Behavior:
//   * root@localhost → always OK (MySQL convention; short-circuits before
//     touching the catalog lock).
//   * No catalog configured → ExecutionError with stable 1105/HY000 surface.
//   * Non-root, no grant → ExecutionError "Permission denied (trigger
//     body DML): ..." before any storage mutation occurs.

/// Build a `TriggerBodyAuthCheck` adapter that delegates to
/// `ExecutionEngine::check_privilege` against the engine's current user
/// identity and (if configured) catalog.
fn build_trigger_auth_check<S: StorageEngine + 'static>(
    engine: &ExecutionEngine<S>,
) -> std::sync::Arc<dyn TriggerBodyAuthCheck> {
    use sqlrustgo_catalog::auth::Privilege as CatalogPrivilege;
    use sqlrustgo_catalog::auth::UserIdentity as CatalogIdentity;
    use sqlrustgo_catalog::ObjectRef as CatalogObjectRef;

    struct EngineAuthCheck {
        catalog: Option<std::sync::Arc<parking_lot::RwLock<sqlrustgo_catalog::Catalog>>>,
        identity: CatalogIdentity,
    }

    impl TriggerBodyAuthCheck for EngineAuthCheck {
        fn check(
            &self,
            user: &CatalogIdentity,
            privilege: CatalogPrivilege,
            table_name: &str,
        ) -> SqlResult<()> {
            // Honor root@localhost bypass first — same rule as top-level
            // `ExecutionEngine::check_privilege`.
            if user.username == "root" {
                return Ok(());
            }
            // Non-root must also match the engine's currently-bound
            // identity. The trigger executor stores its own `current_user`
            // mirror, but we re-validate here so a mismatched mirror
            // (e.g. test racing) can't escalate.
            if user.username != self.identity.username || user.host != self.identity.host {
                return Err(SqlError::ExecutionError(format!(
                    "trigger body DML identity mismatch: hook={}@{} engine={}@{}",
                    user.username, user.host, self.identity.username, self.identity.host
                )));
            }
            let catalog_arc = match self.catalog.as_ref() {
                Some(c) => c.clone(),
                None => {
                    return Err(SqlError::ExecutionError(
                        "trigger body DML requires a catalog to enforce privileges".to_string(),
                    ))
                }
            };
            let catalog = catalog_arc.read();
            catalog
                .auth_manager()
                .check_privilege(
                    user,
                    &CatalogObjectRef::table(table_name),
                    privilege,
                )
                .map_err(|e| {
                    SqlError::ExecutionError(format!(
                        "Permission denied (trigger body DML): {} on {} for {}@{} ({})",
                        privilege, table_name, user.username, user.host, e.message
                    ))
                })
        }
    }

    std::sync::Arc::new(EngineAuthCheck {
        catalog: engine.catalog.clone(),
        identity: engine.current_user().clone(),
    })
}
