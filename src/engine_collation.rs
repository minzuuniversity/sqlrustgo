//! Collation-aware helpers for set-operation executors (V4077 / #4077).
//!
//! Free functions extracted from `execution_engine.rs` as part of the
//! C-ARCH-05 refactor. Used by `engine_setops::execute_union`,
//! `execute_intersect`, `execute_except`, and the trailing ORDER BY
//! logic shared by all three. Also used directly by tests in
//! `tests/anomaly/null_handling_test.rs` (V4077 / V4158 cases).
//!
//! The functions here operate on borrowed `Statement` / `SelectStatement`
//! trees and the storage engine's table metadata, and have no `self`
//! dependency on `ExecutionEngine`. They were top-level `fn`s in
//! `execution_engine.rs`; moving them into a named module makes the
//! dependency graph explicit and shrinks `execution_engine.rs` below
//! the 1600-line architectural limit.

use sqlrustgo_parser::parser::{OrderByExpression, SelectStatement};
use sqlrustgo_parser::Statement;
use sqlrustgo_storage::StorageEngine;

use sqlrustgo_types::Value;

/// Walk nested set-operation statements to find the left-most SELECT's
/// column display names (alias → name). Used by `execute_union` to
/// resolve trailing ORDER BY column references.
pub fn leftmost_column_names(stmt: &Statement) -> Vec<&str> {
    match stmt {
        Statement::Select(s) => s
            .columns
            .iter()
            .map(|c| c.alias.as_deref().unwrap_or(&c.name))
            .collect(),
        Statement::Union(u) => leftmost_column_names(&u.left),
        Statement::Intersect(i) => leftmost_column_names(&i.left),
        Statement::Except(e) => leftmost_column_names(&e.left),
        _ => Vec::new(),
    }
}

/// V4077 / Issue #4077: expand `*` / `table.*` to the actual physical
/// column names of the source table, so ORDER BY resolution against a
/// `SELECT *` left side can find real column indices. Returns owned
/// strings (rather than `&str`) because the physical columns come from
/// a fresh `get_table_info` call and live only as long as the storage
/// read guard.
pub fn expand_column_names(storage: &dyn StorageEngine, stmt: &Statement) -> Vec<String> {
    match stmt {
        Statement::Select(s) => {
            let raw = s
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect::<Vec<_>>();
            // Expand `*` (or `table.*`) once via the source table info.
            let mut out: Vec<String> = Vec::new();
            for n in raw {
                if n == "*" {
                    if !s.table.is_empty() {
                        if let Ok(info) = storage.get_table_info(&s.table) {
                            for col in &info.columns {
                                out.push(col.name.clone());
                            }
                            continue;
                        }
                    }
                    // V313-followup-6 / Issue #4159: `SELECT * FROM (VALUES ...) s(x)`
                    // has `table="s"` but `s` is not in catalog; recurse into
                    // `from_subquery` so the column-list form `s(x)` surfaces.
                    if let Some(sub) = &s.from_subquery {
                        let sub_names =
                            expand_column_names(storage, &Statement::Select(sub.as_ref().clone()));
                        out.extend(sub_names);
                    }
                } else {
                    out.push(n);
                }
            }
            out
        }
        Statement::Union(u) => expand_column_names(storage, &u.left),
        Statement::Intersect(i) => expand_column_names(storage, &i.left),
        Statement::Except(e) => expand_column_names(storage, &e.left),
        _ => Vec::new(),
    }
}

/// V4077 / Issue #4077: collation-aware multiset entries.
/// Returns a map keyed by the *normalized* row (per-column collation
/// applied) to `(count, first_original_row)`. The first row from the
/// input slice is preserved so set-op output retains the LEFT side's
/// original casing for NOCASE columns.
pub fn multiset_entries(
    rows: &[Vec<Value>],
    collations: &[Option<String>],
) -> std::collections::HashMap<Vec<Value>, (usize, Vec<Value>)> {
    let mut entries: std::collections::HashMap<Vec<Value>, (usize, Vec<Value>)> =
        std::collections::HashMap::new();
    for row in rows {
        let key = normalize_row_for_compare(row, collations);
        let entry = entries.entry(key).or_insert_with(|| (0usize, row.clone()));
        entry.0 += 1;
    }
    entries
}

/// V4077 / Issue #4077: walk a set-op's left operand to find the
/// leftmost SELECT statement (so we can look up its source table's
/// column collations).
pub fn leftmost_select(stmt: &Statement) -> Option<&SelectStatement> {
    match stmt {
        Statement::Select(s) => Some(s),
        Statement::Union(u) => leftmost_select(&u.left),
        Statement::Intersect(i) => leftmost_select(&i.left),
        Statement::Except(e) => leftmost_select(&e.left),
        _ => None,
    }
}

/// V4077 / Issue #4077: pull each column's COLLATE name from the
/// source table's TableInfo (if any). Returns a Vec aligned with
/// SELECT projection positions; position N corresponds to column N
/// of the SELECT's projection. If the leftmost SELECT references no
/// resolvable table, returns an empty Vec (caller treats empty as
/// "no collation context" → binary comparison, the original
/// behavior).
pub fn collect_column_collations(
    storage: &dyn StorageEngine,
    stmt: &Statement,
) -> Vec<Option<String>> {
    let Some(select) = leftmost_select(stmt) else {
        return Vec::new();
    };
    if select.table.is_empty() {
        return Vec::new();
    }
    let Ok(info) = storage.get_table_info(&select.table) else {
        return Vec::new();
    };
    // Column names from the SELECT projection (alias → name). For `*`
    // (or `table.*`), expand to every physical column of the source
    // table in declaration order. Otherwise the alias-or-name lookup
    // would fail for `*` and the collation context would silently
    // collapse to binary comparison — which is what produced the V4077
    // NOCASE bug where ABC != abc under binary.
    let mut names: Vec<String> = Vec::new();
    for c in &select.columns {
        let n = c.alias.clone().unwrap_or_else(|| c.name.clone());
        if n == "*" {
            for col in &info.columns {
                names.push(col.name.clone());
            }
        } else {
            names.push(n);
        }
    }
    names
        .into_iter()
        .map(|n| {
            info.columns
                .iter()
                .find(|c| c.name == n)
                .and_then(|c| c.collation.clone())
        })
        .collect()
}

/// V4077 / Issue #4077: produce a comparison key from a row by
/// applying each column's collation. NOCASE folds ASCII case;
/// unknown / BINARY leaves the value as-is.
pub fn normalize_row_for_compare(row: &[Value], collations: &[Option<String>]) -> Vec<Value> {
    if collations.is_empty() {
        return row.to_vec();
    }
    row.iter()
        .enumerate()
        .map(|(i, v)| {
            let coll = collations.get(i).and_then(|c| c.as_deref());
            normalize_value_for_collation(v, coll)
        })
        .collect()
}

pub fn normalize_value_for_collation(v: &Value, collation: Option<&str>) -> Value {
    match (v, collation) {
        (Value::Text(s), Some("NOCASE")) => Value::Text(s.to_uppercase()),
        (v, _) => v.clone(),
    }
}

/// Evaluate a single ORDER BY expression against a row, using column
/// names (from the left SELECT) or 1-based integer position.
pub fn order_by_expr_value(ob: &OrderByExpression, col_names: &[&str], row: &[Value]) -> Value {
    use sqlrustgo_parser::Expression;
    match &ob.expression {
        Expression::Identifier(name) => {
            if let Some(idx) = col_names.iter().position(|n| *n == name) {
                if idx < row.len() {
                    return row[idx].clone();
                }
            }
            Value::Null
        }
        Expression::Literal(lit) => {
            if let Ok(pos) = lit.parse::<usize>() {
                let idx = pos.saturating_sub(1);
                if idx < row.len() {
                    return row[idx].clone();
                }
            }
            Value::Null
        }
        _ => Value::Null,
    }
}
