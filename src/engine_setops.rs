//! Set-operation executors (UNION / INTERSECT / EXCEPT) extracted
//! from `execution_engine.rs`.
//!
//! Part of the C-ARCH-05 refactor (issue #3943 follow-up): reduce
//! `execution_engine.rs` line count below the 1600-line architectural
//! invariant. Each public function in this module takes
//! `&mut ExecutionEngine<S>` and mirrors the legacy associated
//! function. Thin `fn execute_*` wrappers in `execution_engine.rs`
//! delegate here.
//!
//! V4077 / Issue #4077 collation-aware multiset semantics for
//! INTERSECT/EXCEPT and NOCASE-aware DISTINCT for UNION.

use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{
    ExceptStatement, IntersectStatement, OrderByExpression, UnionStatement,
};
use sqlrustgo_parser::Statement;
use sqlrustgo_storage::StorageEngine;

use sqlrustgo_types::Value;

use crate::engine_collation::{
    collect_column_collations, expand_column_names, leftmost_column_names, multiset_entries,
    normalize_row_for_compare, order_by_expr_value,
};
use crate::{ExecutionEngine, SqlResult};

// ── Set-operation handlers (V310-06 PR2 / Issue #3723 C-2) ──────

pub fn execute_union<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    union_stmt: &UnionStatement,
) -> SqlResult<ExecutorResult> {
    let mut left_result = engine.execute_statement(&union_stmt.left)?;
    let right_result = engine.execute_statement(&union_stmt.right)?;

    left_result.rows.extend(right_result.rows);

    if !union_stmt.union_all {
        // V4077 / Issue #4077: collation-aware DISTINCT for UNION.
        // Two rows are duplicates if their per-column values compare
        // equal under each column's collation (NOCASE folds case,
        // BINARY is exact). We dedup by a normalized key while
        // preserving the *left* side's original casing in output.
        let storage = engine.storage_ref().read();
        let left_coll = collect_column_collations(&*storage, &union_stmt.left);
        let right_coll = collect_column_collations(&*storage, &union_stmt.right);
        drop(storage);
        // Walk rows keeping the FIRST occurrence (leftmost wins).
        let mut seen: std::collections::HashSet<Vec<Value>> = std::collections::HashSet::new();
        let mut out: Vec<Vec<Value>> = Vec::with_capacity(left_result.rows.len());
        for row in &left_result.rows {
            // Determine which collation set applies: pick left's if
            // available, otherwise right's. For UNION, both sides
            // contribute one collation context; rows from each side
            // are normalized under their own side's collation before
            // being inserted into the seen set.
            let row_coll = if out.is_empty() {
                &left_coll
            } else {
                &right_coll
            };
            let key = normalize_row_for_compare(row, row_coll);
            if seen.insert(key) {
                out.push(row.to_vec());
            }
        }
        left_result.rows = out;
        // SQL requires output order to be implementation-defined for
        // UNION DISTINCT; preserve insertion order which is the order
        // rows were first seen (left side wins ties).
    }

    // Trailing ORDER BY / LIMIT / OFFSET (C-2c).
    if !union_stmt.trailing_order_by.is_empty() {
        let col_names: Vec<&str> = leftmost_column_names(&union_stmt.left);
        let sort_keys: Vec<Vec<Value>> = left_result
            .rows
            .iter()
            .map(|row| {
                union_stmt
                    .trailing_order_by
                    .iter()
                    .map(|ob| order_by_expr_value(&ob, &col_names, row))
                    .collect()
            })
            .collect();
        let mut indices: Vec<usize> = (0..left_result.rows.len()).collect();
        indices.sort_by(|&a, &b| {
            for (i, ob) in union_stmt.trailing_order_by.iter().enumerate() {
                let ord = if i < sort_keys[a].len() && i < sort_keys[b].len() {
                    sort_keys[a][i].cmp(&sort_keys[b][i])
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
        left_result.rows = indices
            .into_iter()
            .map(|i| left_result.rows[i].clone())
            .collect();
    }
    if let Some(off) = union_stmt.trailing_offset {
        let off = off as usize;
        if off < left_result.rows.len() {
            left_result.rows.drain(..off);
        } else {
            left_result.rows.clear();
        }
    }
    if let Some(lim) = union_stmt.trailing_limit {
        left_result.rows.truncate(lim as usize);
    }

    left_result.affected_rows = left_result.rows.len();
    Ok(left_result)
}

pub fn execute_intersect<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    stmt: &IntersectStatement,
) -> SqlResult<ExecutorResult> {
    let mut left_result = engine.execute_statement(&stmt.left)?;
    let right_result = engine.execute_statement(&stmt.right)?;
    // SQL-92 multiset semantics for INTERSECT:
    //   * DISTINCT (default): deduplicate each side, then keep rows
    //     that appear on both sides (one copy each).
    //   * ALL: keep min(cntL(r), cntR(r)) copies of every row r.
    // V4077 / Issue #4077: row identity for the multiset is the
    // NORMALIZED row under each side's column collations, so a
    // NOCASE column on either side folds case before comparing.
    // The *output* keeps the LEFT side's original casing.
    let storage = engine.storage_ref().read();
    let left_coll = collect_column_collations(&*storage, &stmt.left);
    let right_coll = collect_column_collations(&*storage, &stmt.right);
    drop(storage);
    let left_entries = multiset_entries(&left_result.rows, &left_coll);
    let right_entries = multiset_entries(&right_result.rows, &right_coll);
    let mut out: Vec<Vec<Value>> = Vec::new();
    if stmt.intersect_all {
        for (key, (cnt_l, first_row)) in &left_entries {
            if let Some((cnt_r, _)) = right_entries.get(key) {
                let keep = (*cnt_l).min(*cnt_r);
                for _ in 0..keep {
                    out.push(first_row.clone());
                }
            }
        }
    } else {
        // DISTINCT: a row appears iff it appears on both sides; one copy.
        for (key, (_, first_row)) in &left_entries {
            if right_entries.contains_key(key) {
                out.push(first_row.clone());
            }
        }
    }
    left_result.rows = out;
    // V4077 / Issue #4077: trailing ORDER BY / LIMIT / OFFSET lifted
    // from the right SELECT (consistent with UNION's behavior).
    // INTERSECT DISTINCT now applies the ORDER BY so callers
    // observe deterministic output even when the HashMap iteration
    // order would otherwise shuffle the result.
    apply_trailing_order_limit_offset(
        engine,
        engine.session_null_order_first,
        &stmt.left,
        &stmt.trailing_order_by,
        stmt.trailing_offset.map(|v| v as u64),
        stmt.trailing_limit.map(|v| v as u64),
        &mut left_result.rows,
    );
    left_result.affected_rows = left_result.rows.len();
    Ok(left_result)
}

pub fn execute_except<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    stmt: &ExceptStatement,
) -> SqlResult<ExecutorResult> {
    let mut left_result = engine.execute_statement(&stmt.left)?;
    let right_result = engine.execute_statement(&stmt.right)?;
    // SQL-92 multiset semantics for EXCEPT:
    //   * DISTINCT (default): deduplicate each side, then keep rows
    //     from left that do NOT appear in right (one copy each).
    //   * ALL: keep max(0, cntL(r) - cntR(r)) copies of every row r.
    // V4077 / Issue #4077: same normalization for collation as
    // INTERSECT — NOCASE columns are case-folded for comparison.
    let storage = engine.storage_ref().read();
    let left_coll = collect_column_collations(&*storage, &stmt.left);
    let right_coll = collect_column_collations(&*storage, &stmt.right);
    drop(storage);
    let left_entries = multiset_entries(&left_result.rows, &left_coll);
    let right_entries = multiset_entries(&right_result.rows, &right_coll);
    let mut out: Vec<Vec<Value>> = Vec::new();
    if stmt.except_all {
        for (key, (cnt_l, first_row)) in &left_entries {
            let cnt_r = right_entries.get(key).map(|(c, _)| *c).unwrap_or(0);
            let keep = cnt_l.saturating_sub(cnt_r);
            for _ in 0..keep {
                out.push(first_row.clone());
            }
        }
    } else {
        // DISTINCT: a row is kept iff it appears in left and not in right.
        for (key, (_, first_row)) in &left_entries {
            if !right_entries.contains_key(key) {
                out.push(first_row.clone());
            }
        }
    }
    left_result.rows = out;
    // V4077 / Issue #4077: trailing ORDER BY / LIMIT / OFFSET lifted
    // from the right SELECT (consistent with UNION's behavior).
    // EXCEPT DISTINCT now applies the ORDER BY so callers
    // observe deterministic output (e.g. ORDER BY 1 → ascending).
    apply_trailing_order_limit_offset(
        engine,
        engine.session_null_order_first,
        &stmt.left,
        &stmt.trailing_order_by,
        stmt.trailing_offset.map(|v| v as u64),
        stmt.trailing_limit.map(|v| v as u64),
        &mut left_result.rows,
    );
    left_result.affected_rows = left_result.rows.len();
    Ok(left_result)
}

pub fn apply_trailing_order_limit_offset<S: StorageEngine + 'static>(
    engine: &ExecutionEngine<S>,
    session_null_order_first: Option<bool>,
    stmt_left: &Statement,
    order_by: &[OrderByExpression],
    offset: Option<u64>,
    limit: Option<u64>,
    rows: &mut Vec<Vec<Value>>,
) {
    if order_by.is_empty() && offset.is_none() && limit.is_none() {
        return;
    }
    if !order_by.is_empty() {
        // V4077 / Issue #4077: when the leftmost SELECT uses `*`
        // (or `table.*`), expand to the actual physical columns of
        // the source table so that `ORDER BY a` resolves to a
        // real column index instead of `Value::Null`. Without this
        // the order-by key collapses to NULL for every row and the
        // HashMap insertion order leaks into the output.
        let storage = engine.storage_ref().read();
        let col_names: Vec<String> = expand_column_names(&*storage, stmt_left);
        drop(storage);
        let col_names_ref: Vec<&str> = col_names.iter().map(|s| s.as_str()).collect();
        let sort_keys: Vec<Vec<Value>> = rows
            .iter()
            .map(|row| {
                order_by
                    .iter()
                    .map(|ob| order_by_expr_value(&ob, &col_names_ref, row))
                    .collect()
            })
            .collect();
        let mut indices: Vec<usize> = (0..rows.len()).collect();
        indices.sort_by(|&a, &b| {
            for (i, ob) in order_by.iter().enumerate() {
                // V313-followup-4 / Issue #4157: explicit ob.nulls_first
                // wins; otherwise fall back to session
                // `SET default_null_order`, otherwise default
                // (nulls_first).
                let nulls_first_eff: bool = ob
                    .nulls_first
                    .unwrap_or_else(|| session_null_order_first.unwrap_or(true));
                let ord = if i < sort_keys[a].len() && i < sort_keys[b].len() {
                    let va = &sort_keys[a][i];
                    let vb = &sort_keys[b][i];
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
        *rows = indices.into_iter().map(|i| rows[i].clone()).collect();
    }
    if let Some(off) = offset {
        let off = off as usize;
        if off < rows.len() {
            rows.drain(..off);
        } else {
            rows.clear();
        }
    }
    if let Some(lim) = limit {
        rows.truncate(lim as usize);
    }
}
