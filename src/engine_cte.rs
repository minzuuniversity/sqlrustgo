//! CTE (WITH-clause) executors extracted from `execution_engine.rs`.
//!
//! Free-function implementations of `execute_with_select` and
//! `execute_with_dml`. The thin `pub fn execute_with_*` wrappers in
//! `execution_engine.rs` invoke these.
//!
//! Part of the AD-001 / PR-900 file split (issue #3661).

use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_storage::StorageEngine;

use crate::{ExecutionEngine, SqlError, SqlResult};

/// CTE materialisation helper: execute each CTE's subquery, create a
/// temporary table per CTE, and return the list of created table names so
/// the caller can clean them up. Returns an empty Vec if `with_clause`
/// is None.
pub fn materialize_cte_tables<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    with_clause: Option<&sqlrustgo_parser::parser::WithClause>,
) -> SqlResult<Vec<String>> {
    use sqlrustgo_parser::Statement;
    use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};

    let Some(with_clause) = with_clause else {
        return Ok(Vec::new());
    };
    if with_clause.recursive {
        return Err(SqlError::ExecutionError(
            "Recursive CTE not yet supported".to_string(),
        ));
    }
    for cte in &with_clause.ctes {
        let cte_rows = match cte.subquery.as_ref() {
            Statement::Select(s) => engine.execute_select(s)?.rows,
            _ => {
                return Err(SqlError::ExecutionError(
                    "CTE subquery must be SELECT".to_string(),
                ));
            }
        };
        // Resolve the column names for this CTE in priority order:
        //   1. Explicit `name(col1, col2, ...)` form
        //   2. The subquery's SELECT-column aliases (e.g. `SELECT 'foo' AS a`)
        //   3. The subquery's SELECT-column names (raw expression-derived)
        //   4. Fallback `col_<i>`
        //
        // V313-13 / Issue #4041: previously step 2/3 was missing, so a CTE
        // such as `WITH t AS (SELECT 'foo' AS a)` got columns named
        // `col_0` instead of `a`. Downstream references like
        // `t.a` then failed (or, worse, `t.foobar` silently fell
        // through to `Value::Text("t.foobar")` and produced wrong
        // output). With this fix the CTE column schema matches the
        // subquery's projection, which is what users (and the
        // binder__alias_error_10057 fixture) expect.
        let column_count = if !cte.columns.is_empty() {
            cte.columns.len()
        } else if !cte_rows.is_empty() {
            cte_rows[0].len()
        } else {
            0
        };
        let subquery_column_names: Vec<String> = match cte.subquery.as_ref() {
            Statement::Select(s) => s
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect(),
            _ => Vec::new(),
        };
        let columns: Vec<ColumnDefinition> = (0..column_count)
            .map(|i| {
                let name = if !cte.columns.is_empty() {
                    cte.columns[i].clone()
                } else if i < subquery_column_names.len() && !subquery_column_names[i].is_empty() {
                    subquery_column_names[i].clone()
                } else {
                    format!("col_{}", i)
                };
                ColumnDefinition {
                    name,
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                }
            })
            .collect();
        let table_info = TableInfo {
            name: cte.name.clone(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        };
        let mut storage = engine.storage.write();
        storage
            .create_table(&table_info)
            .map_err(|e| SqlError::ExecutionError(format!("Create CTE table: {}", e)))?;
        if !cte_rows.is_empty() {
            storage
                .insert(&cte.name, cte_rows)
                .map_err(|e| SqlError::ExecutionError(format!("Insert CTE rows: {}", e)))?;
        }
    }
    Ok(with_clause.ctes.iter().map(|c| c.name.clone()).collect())
}

/// Drop a list of temporary CTE tables. Best-effort: a single failed drop
/// does not abort the loop.
pub fn cleanup_cte_tables<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    names: &[String],
) {
    if names.is_empty() {
        return;
    }
    let mut storage = engine.storage.write();
    for name in names {
        let _ = storage.drop_table(name);
    }
}

/// CTE + SELECT body.
pub fn execute_with_select<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    with: &sqlrustgo_parser::parser::WithSelect,
) -> SqlResult<ExecutorResult> {
    let materialized_tables = materialize_cte_tables(engine, with.with_clause.as_ref())?;
    let result = engine.execute_select(&with.select);
    cleanup_cte_tables(engine, &materialized_tables);
    result
}

/// CTE + DML body.
pub fn execute_with_dml<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    with: &sqlrustgo_parser::parser::WithDmlStatement,
) -> SqlResult<ExecutorResult> {
    use sqlrustgo_parser::Statement;

    let materialized_tables = materialize_cte_tables(engine, Some(&with.with_clause))?;
    let result = match with.body.as_ref() {
        Statement::Insert(insert) => crate::engine_dml::execute_insert(engine, insert),
        Statement::Update(update) => crate::engine_dml::execute_update(engine, update),
        Statement::Delete(delete) => crate::engine_dml::execute_delete(engine, delete),
        _ => Err(SqlError::ExecutionError(
            "Unsupported WithDml body type".to_string(),
        )),
    };
    cleanup_cte_tables(engine, &materialized_tables);
    result
}
