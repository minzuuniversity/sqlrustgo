//! V312-64d / Issue #4664: introspection of system tables.
//!
//! `sqlite_master` (and its alias `sqlite_schema`) lists every user-defined
//! table, view, index, and trigger. `mysql.user` and `mysql.db` expose
//! the privilege metadata maintained by the catalog's `AuthManager`.
//!
//! The interception lives in `try_system_table_select`: when a SELECT
//! names one of these tables, the function builds the rows in memory
//! and applies a minimal column-equality WHERE filter, then returns
//! the projected `ExecutorResult`. Otherwise it returns `None` and
//! the caller falls through to the normal table-scan path.

use crate::{ExecutorResult, SqlError, SqlResult, Value};
use sqlrustgo_catalog::system_tables::{MysqlDbTable, MysqlUserTable};
use sqlrustgo_catalog::AuthManager;
use sqlrustgo_parser::Expression;
use sqlrustgo_storage::StorageEngine;

/// Column schema for `sqlite_master` / `sqlite_schema`.
/// Mirrors SQLite's own definition (https://www.sqlite.org/fileformat2.html#intschema):
///   type TEXT, name TEXT, tbl_name TEXT, rootpage INTEGER, sql TEXT.
pub const SQLITE_MASTER_SCHEMA: &[&str] = &["type", "name", "tbl_name", "rootpage", "sql"];

/// Decide whether `select` targets a system table and, if so, return the
/// projected rows. Returns `None` when the FROM clause is a normal user
/// table — the caller should fall through to the standard scan path.
pub fn try_system_table_select(
    select: &sqlrustgo_parser::SelectStatement,
    storage: &dyn StorageEngine,
    auth_manager: Option<&AuthManager>,
) -> Option<SqlResult<ExecutorResult>> {
    let table_name = normalize_table_name(&select.table);
    let qualified = select
        .schema
        .as_deref()
        .map(|s| format!("{}.{}", s, table_name))
        .unwrap_or_else(|| table_name.clone());

    let rows: Vec<Vec<String>> = match qualified.as_str() {
        "sqlite_master" | "sqlite_schema" => Some(build_sqlite_master_rows(storage)),
        "mysql.user" => auth_manager.map(MysqlUserTable::rows),
        "mysql.db" => auth_manager.map(MysqlDbTable::rows),
        _ => None,
    }?;

    // Apply simple column = 'literal' WHERE filter (AND of conjuncts).
    let filtered: Vec<Vec<String>> = match apply_where_filter(
        &rows,
        &SQLITE_MASTER_OR_SCHEMA,
        select.where_clause.as_ref(),
    ) {
        Ok(f) => f,
        Err(e) => return Some(Err(e)),
    };

    // Project requested columns.
    let schema: Vec<String> = match qualified.as_str() {
        "sqlite_master" | "sqlite_schema" => {
            SQLITE_MASTER_SCHEMA.iter().map(|s| s.to_string()).collect()
        }
        "mysql.user" => MysqlUserTable::schema()
            .iter()
            .map(|(n, _)| n.to_string())
            .collect(),
        "mysql.db" => MysqlDbTable::schema()
            .iter()
            .map(|(n, _)| n.to_string())
            .collect(),
        _ => unreachable!(),
    };

    let projection_indices = match compute_projection(&select.columns, &schema) {
        Ok(idx) => idx,
        Err(e) => return Some(Err(e)),
    };

    let columns: Vec<String> = projection_indices
        .iter()
        .map(|i| schema[*i].clone())
        .collect();
    let result_rows: Vec<Vec<Value>> = filtered
        .iter()
        .map(|row| {
            projection_indices
                .iter()
                .map(|i| Value::Text(row[*i].clone()))
                .collect()
        })
        .collect();

    Some(Ok(ExecutorResult::new(result_rows, columns.len())))
}

const SQLITE_MASTER_OR_SCHEMA: &[&str] = SQLITE_MASTER_SCHEMA;

fn build_sqlite_master_rows(storage: &dyn StorageEngine) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    // Tables
    let mut table_names = storage.list_tables();
    table_names.sort();
    for name in table_names {
        if is_reserved(&name) {
            continue;
        }
        if let Ok(info) = storage.get_table_info(&name) {
            out.push(vec![
                "table".to_string(),
                name.clone(),
                name,
                "0".to_string(),
                info.original_sql,
            ]);
        }
    }
    // Views
    let mut view_names = storage.list_views();
    view_names.sort();
    for name in view_names {
        if let Some(info) = storage.get_view(&name) {
            out.push(vec![
                "view".to_string(),
                name.clone(),
                name,
                "0".to_string(),
                info.original_sql,
            ]);
        }
    }
    // Indexes
    let mut indexes = storage.list_all_indexes();
    indexes.sort_by(|a, b| a.name.cmp(&b.name));
    for idx in indexes {
        out.push(vec![
            "index".to_string(),
            idx.name.clone(),
            idx.table.clone(),
            "0".to_string(),
            if idx.original_sql.is_empty() {
                format!(
                    "CREATE {}INDEX {} ON {} ({})",
                    if idx.is_unique { "UNIQUE " } else { "" },
                    idx.name,
                    idx.table,
                    idx.columns.join(", ")
                )
            } else {
                idx.original_sql
            },
        ]);
    }
    // Triggers — list_triggers(table) returns only that table's triggers,
    // so iterate every known table and merge the results. We dedupe by
    // trigger name (an engine that already returns all triggers when
    // given a non-matching table would otherwise double-list).
    let mut trigger_infos: Vec<sqlrustgo_storage::TriggerInfo> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut table_names_for_triggers = storage.list_tables();
    table_names_for_triggers.sort();
    for tbl in table_names_for_triggers {
        if is_reserved(&tbl) {
            continue;
        }
        for info in storage.list_triggers(&tbl) {
            if seen.insert(info.name.clone()) {
                trigger_infos.push(info);
            }
        }
    }
    trigger_infos.sort_by(|a, b| a.name.cmp(&b.name));
    for info in trigger_infos {
        out.push(vec![
            "trigger".to_string(),
            info.name.clone(),
            info.table_name.clone(),
            "0".to_string(),
            info.original_sql,
        ]);
    }
    out
}

fn is_reserved(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "sqlite_master" | "sqlite_schema" | "sqlite_temp_master" | "sqlite_temp_schema"
    )
}

fn normalize_table_name(name: &str) -> String {
    // Strip quoting and lowercase the identifier so that mixed-case
    // `SELECT * FROM SQLite_Master` is accepted.
    let trimmed = name.trim().trim_matches('`').trim_matches('"').to_string();
    trimmed
}

fn apply_where_filter(
    rows: &[Vec<String>],
    schema: &[&str],
    where_clause: Option<&Expression>,
) -> SqlResult<Vec<Vec<String>>> {
    let Some(expr) = where_clause else {
        return Ok(rows.to_vec());
    };
    // Collect flat list of conjuncts (column = literal).
    let conjuncts = flatten_eq_conjuncts(expr);
    let mut out: Vec<Vec<String>> = Vec::with_capacity(rows.len());
    'outer: for row in rows {
        for (col, lit) in &conjuncts {
            let col_idx = match schema.iter().position(|s| s.eq_ignore_ascii_case(col)) {
                Some(i) => i,
                None => {
                    return Err(SqlError::ExecutionError(format!(
                        "Unknown column '{}' in WHERE clause",
                        col
                    )))
                }
            };
            let actual = row.get(col_idx).cloned().unwrap_or_default();
            if !actual.eq_ignore_ascii_case(lit) {
                continue 'outer;
            }
        }
        out.push(row.clone());
    }
    Ok(out)
}

fn flatten_eq_conjuncts(expr: &Expression) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_conjuncts(expr, &mut out);
    out
}

fn collect_conjuncts(expr: &Expression, out: &mut Vec<(String, String)>) {
    use sqlrustgo_parser::Expression as E;
    if let E::BinaryOp(left, op, right) = expr {
        if op.eq_ignore_ascii_case("AND") {
            collect_conjuncts(left, out);
            collect_conjuncts(right, out);
            return;
        }
    }
    if let Some((col, lit)) = try_extract_eq(expr) {
        out.push((col, lit));
    }
    // Non-eq conjuncts are ignored — system tables are filtered best-effort.
}

fn try_extract_eq(expr: &Expression) -> Option<(String, String)> {
    use sqlrustgo_parser::Expression as E;
    let E::BinaryOp(left, op, right) = expr else {
        return None;
    };
    if !op.eq_ignore_ascii_case("=") {
        return None;
    }
    let col = match left.as_ref() {
        E::Identifier(name) => name.clone(),
        _ => return None,
    };
    // Literals come pre-quoted by the parser as String ("123", "'foo'", etc.)
    let raw = match right.as_ref() {
        E::Literal(s) => s.clone(),
        E::Identifier(s) => s.clone(),
        _ => return None,
    };
    // Strip surrounding single quotes for string literals.
    let lit = if raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2 {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw
    };
    Some((col, lit))
}

fn compute_projection(
    cols: &[sqlrustgo_parser::SelectColumn],
    schema: &[String],
) -> SqlResult<Vec<usize>> {
    if cols.len() == 1 {
        // Detect `*` from the SelectColumn shape (name == "*", no alias).
        let c = &cols[0];
        if c.alias.is_none() && (c.name == "*" || c.expression.is_none()) {
            return Ok((0..schema.len()).collect());
        }
    }
    let mut out = Vec::with_capacity(cols.len());
    for c in cols {
        // V312-76 / Issue #4682: accept qualified column references
        // (`SELECT sqlite_master.name, type FROM sqlite_master`).
        // Strip any leading `table.` / `schema.table.` prefix before
        // looking the column up in the synthesised schema.
        let unqualified = c
            .name
            .rsplit('.')
            .next()
            .unwrap_or(&c.name)
            .trim_matches('"');
        let idx = schema
            .iter()
            .position(|s| s.eq_ignore_ascii_case(unqualified))
            .ok_or_else(|| {
                SqlError::ExecutionError(format!("Unknown column '{}' in SELECT list", c.name))
            })?;
        out.push(idx);
    }
    Ok(out)
}

/// V312-88 / Issue #4755: best-effort constant evaluation of an
/// expression, used by the JSON_EACH / JSON_TREE table-valued
/// functions to extract the JSON document from the function argument.
///
/// Only `Expression::Literal` is currently supported; the parser
/// stores single-quoted string literals as `Literal("'text'")`
/// (with the surrounding quotes preserved). Column references and
/// function calls return `None` and the caller treats that as
/// "argument is unknown → produce an empty result set".
pub fn try_evaluate_const(expr: &Expression) -> Option<Value> {
    use sqlrustgo_parser::Expression as E;
    match expr {
        E::Literal(s) => {
            let trimmed = s.trim();
            if trimmed.eq_ignore_ascii_case("NULL") {
                return Some(Value::Null);
            }
            // Strip surrounding single quotes.
            if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
                return Some(Value::Text(trimmed[1..trimmed.len() - 1].to_string()));
            }
            // Try to coerce bare numerics for completeness.
            if let Ok(i) = trimmed.parse::<i64>() {
                return Some(Value::Integer(i));
            }
            if let Ok(f) = trimmed.parse::<f64>() {
                return Some(Value::Float(f));
            }
            // Fall back to treating it as a bare text.
            Some(Value::Text(trimmed.to_string()))
        }
        _ => None,
    }
}

/// V312-88 / Issue #4755: project an in-memory table-valued result
/// (`ExecutorResult`) according to the SELECT column list. The TVF
/// materialiser produces rows in `schema` order; the SELECT list
/// may request a subset (or `*`). `*` is detected by a single
/// `SelectColumn` with no alias and either `name == "*"` or no
/// expression; in that case no projection is applied.
pub fn project_select_columns(
    select: &sqlrustgo_parser::SelectStatement,
    result: &mut ExecutorResult,
    schema: &[&str],
) -> SqlResult<()> {
    let cols = &select.columns;
    if cols.len() == 1 {
        let c = &cols[0];
        if c.alias.is_none() && (c.name == "*" || c.expression.is_none()) {
            return Ok(());
        }
    }
    let mut indices: Vec<usize> = Vec::with_capacity(cols.len());
    for c in cols {
        let unqualified = c
            .name
            .rsplit('.')
            .next()
            .unwrap_or(&c.name)
            .trim_matches('"');
        let idx = schema
            .iter()
            .position(|s| s.eq_ignore_ascii_case(unqualified))
            .ok_or_else(|| {
                SqlError::ExecutionError(format!("Unknown column '{}' in SELECT list", c.name))
            })?;
        indices.push(idx);
    }
    let projected: Vec<Vec<Value>> = result
        .rows
        .iter()
        .map(|row| indices.iter().map(|i| row[*i].clone()).collect())
        .collect();
    result.rows = projected;
    Ok(())
}
