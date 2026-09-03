//! Expression utility functions extracted from execution_engine.rs.
//!
//! Pure helper functions for evaluating SQL expressions against rows.
//! These functions do not depend on the `ExecutionEngine` struct and can be
//! used independently by any module needing expression evaluation.

use sqlrustgo_parser::parser::WhenClause;
use sqlrustgo_parser::parser::WindowCall;
use sqlrustgo_parser::Expression;
use sqlrustgo_parser::SelectStatement;
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;
use std::collections::HashMap;

pub fn expression_to_string(expr: &sqlrustgo_parser::Expression) -> String {
    match expr {
        sqlrustgo_parser::Expression::Literal(s) => s.clone(),
        sqlrustgo_parser::Expression::Identifier(name) => name.clone(),
        sqlrustgo_parser::Expression::BinaryOp(left, op, right) => {
            format!(
                "({} {} {})",
                expression_to_string(left),
                op,
                expression_to_string(right)
            )
        }
        sqlrustgo_parser::Expression::IsNull(inner) => {
            format!("{} IS NULL", expression_to_string(inner))
        }
        sqlrustgo_parser::Expression::IsNotNull(inner) => {
            format!("{} IS NOT NULL", expression_to_string(inner))
        }
        sqlrustgo_parser::Expression::Aggregate(agg) => match agg.func {
            sqlrustgo_parser::AggregateFunction::Count => {
                if agg.args.is_empty() {
                    "COUNT(*)".to_string()
                } else {
                    format!(
                        "COUNT({})",
                        agg.args
                            .iter()
                            .map(expression_to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            sqlrustgo_parser::AggregateFunction::Sum => {
                format!(
                    "SUM({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::Avg => {
                format!(
                    "AVG({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::Min => {
                format!(
                    "MIN({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::Max => {
                format!(
                    "MAX({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::QuantileDisc => {
                format!(
                    "QUANTILE_DISC({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::QuantileCont => {
                format!(
                    "QUANTILE_CONT({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            sqlrustgo_parser::AggregateFunction::PercentileCont => {
                format!(
                    "PERCENTILE_CONT({})",
                    agg.args
                        .iter()
                        .map(expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            // V312-64b / Issue #4650: GROUP_CONCAT aggregate. The args contain
            // sentinel literals (__DISTINCT__/__NO_DISTINCT__/__ORDER_BY__/
            // __ASC__/__DESC__/__SEPARATOR__) that should NOT appear in the
            // canonical name — strip them and walk only real operand exprs.
            sqlrustgo_parser::AggregateFunction::GroupConcat => {
                let distinct = matches!(
                    agg.args.first(),
                    Some(sqlrustgo_parser::Expression::Literal(l)) if l == "__DISTINCT__"
                );
                let val_str = agg
                    .args
                    .get(1)
                    .map(expression_to_string)
                    .unwrap_or_default();
                let mut parts = Vec::new();
                if distinct {
                    parts.push("DISTINCT".to_string());
                }
                parts.push(val_str);
                // Scan args[2..] for ORDER BY / SEPARATOR clauses
                let mut i = 2;
                while i < agg.args.len() {
                    match &agg.args[i] {
                        sqlrustgo_parser::Expression::Literal(l) if l == "__ORDER_BY__" => {
                            let mut ob = String::from("ORDER BY ");
                            if let Some(e) = agg.args.get(i + 1) {
                                ob.push_str(&expression_to_string(e));
                                i += 2;
                            } else {
                                i += 1;
                                continue;
                            }
                            if let Some(sqlrustgo_parser::Expression::Literal(d)) = agg.args.get(i) {
                                if d == "__DESC__" {
                                    ob.push_str(" DESC");
                                    i += 1;
                                } else if d == "__ASC__" {
                                    ob.push_str(" ASC");
                                    i += 1;
                                }
                            }
                            parts.push(ob);
                        }
                        sqlrustgo_parser::Expression::Literal(l) if l == "__SEPARATOR__" => {
                            if let Some(e) = agg.args.get(i + 1) {
                                let sep = expression_to_string(e);
                                parts.push(format!("SEPARATOR {}", sep));
                                i += 2;
                            } else {
                                i += 1;
                            }
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                format!("GROUP_CONCAT({})", parts.join(" "))
            }
        },
        _ => "?".to_string(),
    }
}

/// Convert a parser Expression to a Value (simple literal evaluation)
pub fn expression_to_value(expr: &sqlrustgo_parser::Expression) -> Value {
    match expr {
        // P0-2 §3.4: delegated to `executor::expr::eval_literal_from_str`
        // (single source of truth for literal evaluation).
        sqlrustgo_parser::Expression::Literal(s) => {
            sqlrustgo_executor::expr::eval_literal_from_str(s)
        }
        // P0-2 §4.10: delegated to `executor::expr::eval_literal_from_str`-
        // style helper. For an Identifier (not a literal), the legacy
        // fallback was `Value::Text(name.clone())` (treat the identifier
        // as a string literal when not in the schema). Preserved here
        // because the same path is also reached via `evaluate_expression`'s
        // fallback arm when the column lookup fails — see the
        // `Expression::Identifier` arm in `evaluate_expression` for the
        // full single-source-of-truth delegation.
        // P0-2 §4.12: delegated to `executor::expr::eval_unary_op` /
        // `cast_val` for the corresponding arms. The Identifier
        // fallback path here is preserved for the non-row path
        // (used by `expression_to_value` callers like the EXTRACT
        // arm in the legacy `evaluate_expression`).
        sqlrustgo_parser::Expression::Identifier(name) => Value::Text(name.clone()),
        // ODKU UPDATE assignment: ignore LHS identifier, recurse on RHS.
        // The parser emits `BinaryOp(Identifier(col), "=", right)` for
        // the SET clause; we want only the right-hand side evaluated.
        sqlrustgo_parser::Expression::BinaryOp(_, op, right) if op == "=" => {
            expression_to_value(right)
        }
        _ => Value::Null,
    }
}

/// Convert a string argument to a Value (for CALL arguments)
/// SQL LIKE pattern matcher — DEPRECATED, delegates to
/// `sqlrustgo_executor::expr::sql_like_match` (P0-2 §4.5).
///
/// Kept as a `pub(crate)` shim during the transition; callers should
/// switch to importing the executor function directly. This shim will
/// be removed in a follow-up PR after `src/engine_utils.rs` is
/// migrated to use `executor::expr` directly (per OpenSpec Decision D3).
pub(crate) fn sql_like_match(text: &str, pattern: &str) -> bool {
    sqlrustgo_executor::expr::sql_like_match(text, pattern)
}

/// Recursive wildcard matcher. DEPRECATED, moved to
/// `sqlrustgo_executor::expr::like_match_recursive` (private).
#[allow(dead_code)] // stub retained for backward import compatibility (see P0-2 §4.5)
fn like_match_recursive(_text: &str, _pattern: &str) -> bool {
    // Body removed (P0-2 §4.5). Kept as a no-op stub so any in-tree
    // caller that imports it via `use crate::expr_utils::like_match_recursive;`
    // still compiles. The real implementation lives in
    // `crates/executor/src/expr/mod.rs`.
    unreachable!("like_match_recursive moved to sqlrustgo_executor::expr; this stub is unreachable")
}

pub fn expression_to_value_from_string(s: &str) -> Value {
    let s = s.trim();
    if s.eq_ignore_ascii_case("NULL") {
        Value::Null
    } else if let Ok(n) = s.parse::<i64>() {
        Value::Integer(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::Float(f)
    } else if s.starts_with('\'') && s.ends_with('\'') {
        Value::Text(s[1..s.len() - 1].to_string())
    } else {
        Value::Text(s.to_string())
    }
}

/// Evaluate an expression and return a Value
pub fn evaluate_expression(
    expr: &Expression,
    row: &[Value],
    table_info: &TableInfo,
) -> Result<Value, String> {
    evaluate_expression_with_subq(expr, row, table_info, &|_| Ok(Value::Null))
}

/// Like [`evaluate_expression_with_subq`] but additionally takes an
/// optional storage reference for evaluating sequence expressions
/// (`NEXT VALUE FOR seq`, `CURRVAL(seq)`).
///
/// The plain [`evaluate_expression`] path (no engine context) cannot
/// advance or read sequence state, so it returns `Value::Null` for these
/// arms. When the projection lives in a method that has `&self.storage`
/// Like [`evaluate_expression_with_subq`] but additionally takes an
/// optional mutable storage reference for evaluating sequence
/// expressions (`NEXT VALUE FOR seq`, `CURRVAL(seq)`).
///
/// The plain [`evaluate_expression`] path (no engine context) cannot
/// advance or read sequence state, so it returns `Value::Null` for these
/// arms. When the projection lives in a method that has `&self.storage`
/// (e.g. `ExecutionEngine::execute_select`), pass
/// `Some(&mut *engine.storage.write())` to enable real sequence
/// evaluation.
///
/// Pass `None` when no storage is in scope (e.g. testing the helper
/// in isolation, or evaluating expressions outside an engine context).
pub fn evaluate_expression_with_seq(
    expr: &Expression,
    row: &[Value],
    table_info: &TableInfo,
    seq_state: Option<&crate::sequence_state::SequenceState>,
    subq_eval: &dyn Fn(&SelectStatement) -> Result<Value, String>,
) -> Result<Value, String> {
    if let Some(state) = seq_state {
        match expr {
            Expression::SequenceNextVal(name) => {
                return state
                    .next_value(name)
                    .map(Value::Integer)
                    .map_err(|e| format!("NEXT VALUE FOR {}: {}", name, e));
            }
            Expression::SequenceCurrval(name) => {
                // CURRVAL semantics: return the value most recently
                // produced by NEXT_VALUE. State::currval returns Err
                // before the first NEXTVAL; we surface that as the
                // same error message the previous storage-backed path
                // returned.
                return state
                    .currval(name)
                    .map(Value::Integer)
                    .map_err(|e| format!("CURRVAL {}: {}", name, e));
            }
            _ => {}
        }
    }
    evaluate_expression_with_subq(expr, row, table_info, subq_eval)
}

/// Like `evaluate_expression` but can resolve `(SELECT ...)` scalar
/// subqueries via `subq_eval` (called with the inner select, expected
/// to return its scalar value or Null for an empty result).
pub fn evaluate_expression_with_subq(
    expr: &Expression,
    row: &[Value],
    table_info: &TableInfo,
    subq_eval: &dyn Fn(&SelectStatement) -> Result<Value, String>,
) -> Result<Value, String> {
    match expr {
        Expression::Literal(s) => {
            // P0-2 §4.15: delegated to `executor::expr::eval_literal_from_str`
            // (single source of truth for the Literal branch).
            Ok(sqlrustgo_executor::expr::eval_literal_from_str(s))
        }
        Expression::Identifier(name) => {
            // P0-2 §4.10: delegated to `executor::expr::eval_identifier`.
            // Looks up the column by name; if not found, falls back to
            // `Value::Text(name)` (the legacy behavior for unqualified
            // identifiers that happen to be string literals).
            sqlrustgo_executor::expr::eval_identifier(name, row, &table_info.columns)
        }
        Expression::UnaryOp(op, inner) => {
            // P0-2 §4.12: delegated to `executor::expr::eval_unary_op`.
            // The operator is applied to the *evaluated* inner value.
            // The arm was previously absent (UnaryOp fell through to
            // `_ => Ok(Value::Null)`), so this adds real new
            // functionality (TPC-H Q5/Q8 use NOT in HAVING).
            let val = evaluate_expression(inner, row, table_info)?;
            Ok(sqlrustgo_executor::expr::eval_unary_op(&val, op))
        }
        // P0-2 §4.13 (Cast): DEFERRED. The `sqlrustgo_parser::Expression`
        // enum does not currently have a `Cast` variant; the parser
        // expresses casts via `FunctionCall("CAST", ...)` instead.
        // Adding a dedicated `Cast` arm here would require a parser
        // change (new enum variant + parser changes) which is out of
        // scope for P0-2. The `executor::expr::cast_val` function
        // IS available (P0-2 §4.13 doc) for future use. Tracked in
        // the OpenSpec tasks.md §4.13.
        Expression::BinaryOp(left, op, right) => {
            // P0-2 §4.14: delegated to `executor::expr::eval_binary_op`
            // (single source of truth for the BinaryOp branch).
            let left_val = evaluate_expression(left, row, table_info).unwrap_or(Value::Null);
            let right_val = evaluate_expression(right, row, table_info).unwrap_or(Value::Null);
            Ok(sqlrustgo_executor::expr::eval_binary_op(
                &left_val, &right_val, op,
            ))
        }
        Expression::IsNull(inner) => {
            // P0-2 §4.2: delegated to `executor::expr::eval_is_null`
            // (single source of truth for the IsNull branch).
            let val = evaluate_expression(inner, row, table_info)?;
            Ok(sqlrustgo_executor::expr::eval_is_null(&val))
        }
        Expression::IsNotNull(inner) => {
            // P0-2 §4.3: delegated to `executor::expr::eval_is_not_null`.
            // **Pre-existing bug fixed by this PR**: `evaluate_expression`
            // previously had no explicit `Expression::IsNotNull` arm; the
            // call fell through to the wildcard `_ => Ok(Value::Null)`
            // arm, returning `Value::Null` for every `IS NOT NULL` query
            // (e.g. `WHERE col IS NOT NULL` would silently never match).
            // The new explicit arm + delegation to `executor::expr` (where
            // the UnifiedExpr::IsNotNull implementation has been correct
            // since v3.8.0) fixes this. Verified by
            // `test_isnull_delegation`'s "empty string (not null)" case.
            let val = evaluate_expression(inner, row, table_info)?;
            Ok(sqlrustgo_executor::expr::eval_is_not_null(&val))
        }
        // TPC-H Q9: `WHERE p_name LIKE '%green%'`. SQL LIKE substring
        // match: `%` matches any sequence (including empty), `_` matches
        // a single char. Issue #4677: the parser-supplied ESCAPE
        // character (if any) is now honored — `LIKE '100!%' ESCAPE '!'`
        // matches the literal string `100%`.
        Expression::Like(expr, pattern, escape) => {
            // P0-2 §4.5: delegated to `executor::expr::sql_like_match_esc`
            // (single source of truth for the LIKE pattern matcher).
            let val = evaluate_expression(expr, row, table_info)
                .map(|v| v.to_sql_string())
                .unwrap_or_default();
            let pat = evaluate_expression(pattern, row, table_info)
                .map(|v| v.to_sql_string())
                .unwrap_or_default();
            Ok(Value::Boolean(
                sqlrustgo_executor::expr::sql_like_match_esc(&val, &pat, *escape),
            ))
        }
        // Evaluate each WHEN's condition in order; the first one whose
        // value is Boolean(true) (or non-zero/non-null) wins, and we
        // return its THEN expression. If no WHEN matches and an ELSE
        // is present, return its value; otherwise Null. This matches
        // the executor's UnifiedExpr::CaseWhen semantics.
        Expression::CaseWhen(whens, else_val) => {
            // P0-2 §4.9: delegated to `executor::expr::eval_case_when`.
            // The evaluate_fn closure threads our local row/table_info
            // through so the algorithm (which lives in the executor)
            // doesn't need to know about TableInfo.
            sqlrustgo_executor::expr::eval_case_when(whens, else_val.as_deref(), |e| {
                evaluate_expression(e, row, table_info)
            })
        }
        // TPC-H Q7/Q8/Q9: EXTRACT(field FROM col). The parser encodes this
        // as FunctionCall("EXTRACT", [Literal(field), source_expr]). The
        // // generic `Expression::FunctionCall` arm below handles EXTRACT
        // // via `executor::expr::eval_fn` (which contains the same
        // // YEAR/MONTH/DAY slicing logic in a single, deduplicated
        // // implementation). P0-2 §4.11 removes the previous
        // // duplicate, special-cased arm that lived here.
        Expression::Aggregate(agg) => {
            // P0-2 §4.4: delegated to `executor::expr::eval_aggregate_lookup`.
            // The aggregate is *not* computed here; it is looked up from
            // a pre-aggregated column in the row by its canonical name
            // (e.g. "COUNT(*)", "SUM(l_quantity)"). See the doc comment
            // on `executor::expr::eval_aggregate_lookup` for why.
            let agg_name = expression_to_string(&Expression::Aggregate(agg.clone()));
            let column_names: Vec<String> =
                table_info.columns.iter().map(|c| c.name.clone()).collect();
            match sqlrustgo_executor::expr::eval_aggregate_lookup(&agg_name, row, &column_names) {
                Some(v) => Ok(v),
                None => Err(format!("Aggregate not found in schema: {}", agg_name)),
            }
        }
        // MySQL 5.7 function dispatch (Issue #2988 / MySQL-01).
        // The crate-level `sqlrustgo-executor::expr::eval_fn` is the
        // single source of truth for the function table. We delegate
        // here so that `engine_select.rs` (which uses this
        // `evaluate_expression`) gets the same behavior as the
        // `UnifiedExpr` path. Avoid duplicating the function table.
        Expression::FunctionCall(name, args) => {
            use sqlrustgo_executor::expr::eval_fn as dispatch_fn;
            let vals: Vec<Value> = args
                .iter()
                .map(|a| evaluate_expression(a, row, table_info).unwrap_or(Value::Null))
                .collect();
            Ok(dispatch_fn(name, &vals))
        }
        // TPC-H Q8/Q12/Q14: CASE WHEN cond THEN a ELSE b END.
        Expression::NotLike(left, pattern, escape) => {
            // P0-2 §4.6: delegated to `executor::expr::sql_like_match_esc`.
            // Issue #4677: NOT LIKE honors ESCAPE the same way LIKE does.
            let lv = evaluate_expression(left, row, table_info)?;
            let pv = evaluate_expression(pattern, row, table_info)?;
            Ok(Value::Boolean(
                !sqlrustgo_executor::expr::sql_like_match_esc(
                    &lv.to_sql_string(),
                    &pv.to_sql_string(),
                    *escape,
                ),
            ))
        }
        // TPC-H Q1: expr BETWEEN low AND high.
        Expression::Between(expr, low, high) => {
            // P0-2 §4.7: delegated to `executor::expr::eval_between`.
            let v = evaluate_expression(expr, row, table_info)?;
            let lo = evaluate_expression(low, row, table_info)?;
            let hi = evaluate_expression(high, row, table_info)?;
            Ok(sqlrustgo_executor::expr::eval_between(&v, &lo, &hi))
        }
        Expression::NotBetween(expr, low, high) => {
            // P0-2 §4.8: delegated to `executor::expr::eval_not_between`.
            let v = evaluate_expression(expr, row, table_info)?;
            let lo = evaluate_expression(low, row, table_info)?;
            let hi = evaluate_expression(high, row, table_info)?;
            Ok(sqlrustgo_executor::expr::eval_not_between(&v, &lo, &hi))
        }
        // TPC-H Q20: col IN (subquery) and col NOT IN (subquery).
        Expression::In(_, _)
        | Expression::NotIn(_, _)
        | Expression::InList(_, _)
        | Expression::NotInList(_, _)
        | Expression::Exists(_)
        | Expression::NotExists(_)
        | Expression::QuantifiedOp(_, _, _) => {
            // The executor handles these via the dedicated In/Exists path in
            // engine_select (Step 2 WHERE). evaluate_expression is only used
            // for column-projection and aggregate arguments, where In/Exists
            // appearing directly is unusual; return Null defensively.
            Ok(Value::Null)
        }
        Expression::Subquery(subq) => subq_eval(subq),
        Expression::SubqueryField(inner, _field) => {
            evaluate_expression_with_subq(inner, row, table_info, subq_eval)
        }
        // MySQL `@@version_comment` / `@@autocommit` etc. — resolved to a
        // scalar literal at evaluation time. We delegate to the same
        // resolver that the unified-expr path uses so the wire-protocol
        // result (column count, value type) is consistent regardless of
        // which evaluator the SELECT passes through.
        Expression::SystemVariable(name) => {
            Ok(sqlrustgo_executor::expr::resolve_system_variable(name))
        }
        _ => Ok(Value::Null),
    }
}

/// V312-58 / Issue #4517: evaluate a `WindowCall` (`AGG(x) OVER (PARTITION
/// BY ...) ORDER BY ...`) over the full projection row set, returning one
/// `Value` per input row in the same order.
///
/// The window spec follows standard SQL semantics:
/// * `partition_by` groups rows into independent windows. Empty partition
///   means a single global window.
/// * `order_by` (if non-empty) sorts rows within each partition;
///   `bool = false` means DESC.
/// * The aggregate / ordinal function is then evaluated per row over the
///   full partition (no frame-clause handling for now — issue scope is
///   `aggregate(expr) OVER (PARTITION BY cols)` plus
///   `ROW_NUMBER()/RANK()/DENSE_RANK() OVER (...)`).
///
/// Supported function names (case-insensitive):
/// * Aggregates: `SUM`, `AVG`, `COUNT`, `MIN`, `MAX`
/// * Ordinals:   `ROW_NUMBER`, `RANK`, `DENSE_RANK`
///
/// Returns `Ok(Vec<Value>)` whose length equals `rows.len()`. The caller is
/// responsible for inserting the value at the correct row position; we
/// preserve the input ordering so a positional lookup is a simple
/// `results[row_idx]`.
pub fn evaluate_window_call(
    call: &WindowCall,
    rows: &[Vec<Value>],
    table_info: &TableInfo,
) -> Result<Vec<Value>, String> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // 1. Compute the partition key for every row.
    let partition_keys: Vec<Vec<Value>> = if call.window_spec.partition_by.is_empty() {
        vec![Vec::new(); rows.len()]
    } else {
        let mut keys = Vec::with_capacity(rows.len());
        for row in rows {
            let mut key = Vec::with_capacity(call.window_spec.partition_by.len());
            for part_expr in &call.window_spec.partition_by {
                let v = evaluate_expression(part_expr, row, table_info)?;
                key.push(v);
            }
            keys.push(key);
        }
        keys
    };

    // 2. Bucket row indices by canonicalized partition key string.
    //    (Value doesn't implement Hash, but the canonical-string form is
    //    deterministic for use as a partition discriminator.)
    let canonical_keys: Vec<String> = partition_keys
        .iter()
        .map(|k| {
            k.iter()
                .map(|v| match v {
                    Value::Null => "__NULL__".to_string(),
                    Value::Integer(n) => format!("I:{}", n),
                    Value::Float(f) => format!("F:{}", f),
                    Value::Text(s) => format!("T:{}", s),
                    Value::Boolean(b) => format!("B:{}", b),
                    _ => format!("O:{}", v.to_sql_string()),
                })
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect();

    let mut partitions: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, key) in canonical_keys.iter().enumerate() {
        partitions.entry(key.clone()).or_default().push(idx);
    }

    // 3. Sort each partition by order_by (stable sort preserves input order
    //    for ties, matching standard SQL semantics).
    if !call.window_spec.order_by.is_empty() {
        for indices in partitions.values_mut() {
            indices.sort_by(|&a, &b| {
                let row_a = &rows[a];
                let row_b = &rows[b];
                let mut cmp = std::cmp::Ordering::Equal;
                for (sort_expr, asc) in &call.window_spec.order_by {
                    let val_a =
                        evaluate_expression(sort_expr, row_a, table_info).unwrap_or(Value::Null);
                    let val_b =
                        evaluate_expression(sort_expr, row_b, table_info).unwrap_or(Value::Null);
                    // Three-valued comparison: Null sorts after everything,
                    // matching standard SQL NULLS LAST default.
                    let step = match (val_a == Value::Null, val_b == Value::Null) {
                        (true, true) => std::cmp::Ordering::Equal,
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        (false, false) => compare_values(&val_a, &val_b).cmp(&0),
                    };
                    let step = if *asc { step } else { step.reverse() };
                    if step != std::cmp::Ordering::Equal {
                        cmp = step;
                        break;
                    }
                }
                cmp
            });
        }
    }

    // 4. Allocate output buffer (one Value per input row, in input order).
    let mut results = vec![Value::Null; rows.len()];

    let func_upper = call.func_name.to_uppercase();
    for indices in partitions.values() {
        for (local_idx, &row_idx) in indices.iter().enumerate() {
            let value = match func_upper.as_str() {
                "ROW_NUMBER" => Value::Integer((local_idx + 1) as i64),
                "RANK" => {
                    // RANK: peers share rank; rank of a row ranks = 1 +
                    // (number of rows in earlier positions that have a
                    // strictly different order_by key).
                    //
                    // Walk back through the partition (which is already
                    // sorted by order_by). The current row's rank is
                    // the 1-based position in the peer group: i.e. count
                    // the number of rows whose order_by keys differ from
                    // the current row's keys (those that appear strictly
                    // before `local_idx` and are in different peer groups).
                    let mut earlier_diff_groups = 0i64;
                    for &prev_idx in indices.iter().take(local_idx) {
                        let mut same = true;
                        for (sort_expr, _) in &call.window_spec.order_by {
                            let cur = evaluate_expression(sort_expr, &rows[row_idx], table_info)
                                .unwrap_or(Value::Null);
                            let prv = evaluate_expression(sort_expr, &rows[prev_idx], table_info)
                                .unwrap_or(Value::Null);
                            if compare_values(&cur, &prv) != 0 {
                                same = false;
                                break;
                            }
                        }
                        if !same {
                            earlier_diff_groups += 1;
                        }
                    }
                    // +1 because positions are 1-based and the row itself
                    // is not counted.
                    Value::Integer(earlier_diff_groups + 1)
                }
                "DENSE_RANK" => {
                    // DENSE_RANK: peers share rank; no gaps.
                    //
                    // Since the partition is sorted by order_by, count the
                    // distinct order_by groups among the rows at positions
                    // [0..local_idx]. A "group" transition is detected by
                    // comparing adjacent rows in sorted order.
                    let mut distinct_groups = 1i64;
                    for pair in indices.windows(2).take(local_idx) {
                        let prev_idx = pair[0];
                        let next_idx = pair[1];
                        let mut same = true;
                        for (sort_expr, _) in &call.window_spec.order_by {
                            let a = evaluate_expression(sort_expr, &rows[prev_idx], table_info)
                                .unwrap_or(Value::Null);
                            let b = evaluate_expression(sort_expr, &rows[next_idx], table_info)
                                .unwrap_or(Value::Null);
                            if compare_values(&a, &b) != 0 {
                                same = false;
                                break;
                            }
                        }
                        if !same {
                            distinct_groups += 1;
                        }
                    }
                    Value::Integer(distinct_groups)
                }
                "SUM" | "AVG" | "COUNT" | "MIN" | "MAX" => {
                    // Aggregate over the full partition (no frame clause yet).
                    if call.args.is_empty() {
                        // COUNT(*) equivalent — count rows in partition.
                        Value::Integer(indices.len() as i64)
                    } else if call.args.len() == 1 {
                        let arg = &call.args[0];
                        let is_star = matches!(arg, Expression::Literal(s) if s == "*");
                        if is_star && func_upper == "COUNT" {
                            Value::Integer(indices.len() as i64)
                        } else {
                            let collected: Vec<Value> = indices
                                .iter()
                                .map(|&i| {
                                    evaluate_expression(arg, &rows[i], table_info)
                                        .unwrap_or(Value::Null)
                                })
                                .collect();
                            match func_upper.as_str() {
                                "SUM" => numeric_agg_sum(&collected),
                                "AVG" => numeric_agg_avg(&collected),
                                "COUNT" => Value::Integer(
                                    collected
                                        .iter()
                                        .filter(|v| !matches!(v, Value::Null))
                                        .count() as i64,
                                ),
                                "MIN" => collected
                                    .iter()
                                    .filter(|v| !matches!(v, Value::Null))
                                    .min_by(|a, b| compare_values(a, b).cmp(&0))
                                    .cloned()
                                    .unwrap_or(Value::Null),
                                "MAX" => collected
                                    .iter()
                                    .filter(|v| !matches!(v, Value::Null))
                                    .max_by(|a, b| compare_values(a, b).cmp(&0))
                                    .cloned()
                                    .unwrap_or(Value::Null),
                                _ => unreachable!(),
                            }
                        }
                    } else {
                        // Multiple args — not meaningful for window aggregates
                        // at the issue scope; emit Null rather than panic.
                        Value::Null
                    }
                }
                _ => {
                    // Unsupported window function — return Null for the row.
                    Value::Null
                }
            };
            results[row_idx] = value;
        }
    }

    Ok(results)
}

fn numeric_agg_sum(vals: &[Value]) -> Value {
    let mut int_sum: Option<i64> = Some(0);
    let mut float_sum: Option<f64> = None;
    let mut any = false;
    for v in vals {
        match v {
            Value::Integer(n) => {
                any = true;
                if let Some(ref mut s) = int_sum {
                    *s += n;
                }
                if let Some(ref mut s) = float_sum {
                    *s += *n as f64;
                }
            }
            Value::Float(f) => {
                any = true;
                if float_sum.is_none() {
                    float_sum = Some(int_sum.unwrap_or(0) as f64);
                    int_sum = None;
                }
                if let Some(ref mut s) = float_sum {
                    *s += f;
                }
            }
            Value::Null => {}
            _ => {
                // Non-numeric: coerce via to_sql_string->parse.
                if let Ok(n) = v.to_sql_string().parse::<i64>() {
                    any = true;
                    if let Some(ref mut s) = int_sum {
                        *s += n;
                    }
                } else if let Ok(f) = v.to_sql_string().parse::<f64>() {
                    any = true;
                    if float_sum.is_none() {
                        float_sum = Some(int_sum.unwrap_or(0) as f64);
                        int_sum = None;
                    }
                    if let Some(ref mut s) = float_sum {
                        *s += f;
                    }
                }
            }
        }
    }
    if !any {
        Value::Null
    } else if let Some(f) = float_sum {
        Value::Float(f)
    } else {
        Value::Integer(int_sum.unwrap_or(0))
    }
}

fn numeric_agg_avg(vals: &[Value]) -> Value {
    let mut sum_int: i64 = 0;
    let mut sum_float: Option<f64> = None;
    let mut count: i64 = 0;
    for v in vals {
        match v {
            Value::Integer(n) => {
                count += 1;
                sum_int += n;
                if let Some(ref mut s) = sum_float {
                    *s += *n as f64;
                }
            }
            Value::Float(f) => {
                count += 1;
                if sum_float.is_none() {
                    sum_float = Some(sum_int as f64);
                }
                if let Some(ref mut s) = sum_float {
                    *s += f;
                }
            }
            Value::Null => {}
            _ => {
                if let Ok(n) = v.to_sql_string().parse::<i64>() {
                    count += 1;
                    sum_int += n;
                    if let Some(ref mut s) = sum_float {
                        *s += n as f64;
                    }
                } else if let Ok(f) = v.to_sql_string().parse::<f64>() {
                    count += 1;
                    if sum_float.is_none() {
                        sum_float = Some(sum_int as f64);
                    }
                    if let Some(ref mut s) = sum_float {
                        *s += f;
                    }
                }
            }
        }
    }
    if count == 0 {
        Value::Null
    } else if let Some(f) = sum_float {
        Value::Float(f / count as f64)
    } else {
        Value::Float(sum_int as f64 / count as f64)
    }
}

/// Evaluate a binary operation. Returns a Boolean for comparison/logical
/// operators, Integer or Float for arithmetic operators. Float inputs
/// promote; mixing Float and Integer produces Float. TPC-H Q7/Q8/Q9 use
/// `l_extendedprice * (1 - l_discount)` so the arithmetic arms must
/// actually work.
pub fn evaluate_binary_op(left: &Value, right: &Value, op: &str) -> Value {
    // SQL three-valued logic: NULL comparison returns Null (UNKNOWN).
    // Per SQL standard, only IS [NOT] NULL and IS [NOT] DISTINCT FROM
    // treat NULL as a known value. Plain `=`, `!=`, `<`, `>`, `<=`, `>=`
    // with any NULL operand yields Null (which the WHERE evaluator
    // treats as a non-match, matching the documented SQL semantics).
    let op_upper = op.to_uppercase();
    let any_null = matches!(left, Value::Null) || matches!(right, Value::Null);
    let _both_null = matches!(left, Value::Null) && matches!(right, Value::Null);
    match op_upper.as_str() {
        // Equality / inequality: with at least one NULL, the result is
        // UNKNOWN (Null in this engine). Only `IS NULL` / `IS NOT NULL`
        // can give a Boolean true/false on NULL — that path uses
        // Expression::IsNull and never reaches this arm.
        "=" | "==" | "IS" if any_null => Value::Null,
        "!=" | "<>" if any_null => Value::Null,
        ">" | ">=" | "<" | "<=" if any_null => Value::Null,
        // Boolean IS / IS NOT — same as =/!= but historically also
        // returns Null on NULL operand (we keep parity).
        "IS NOT" if any_null => Value::Null,
        "=" | "==" | "IS" => Value::Boolean(left == right),
        "!=" | "<>" => Value::Boolean(left != right),
        ">" => Value::Boolean(compare_values(left, right) > 0),
        ">=" => Value::Boolean(compare_values(left, right) >= 0),
        "<" => Value::Boolean(compare_values(left, right) < 0),
        "<=" => Value::Boolean(compare_values(left, right) <= 0),
        "AND" | "&&" => {
            if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                Value::Boolean(*l && *r)
            } else {
                Value::Boolean(false)
            }
        }
        "OR" | "||" => {
            if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                Value::Boolean(*l || *r)
            } else {
                Value::Boolean(false)
            }
        }
        // Arithmetic. Promote to Float if either side is Float so that
        // `100.0 * (1 - 0.10)` yields `90.0`, not `Integer(90)`.
        "+" => arithmetic_op(left, right, |a, b| a + b, |a, b| a + b),
        "-" => arithmetic_op(left, right, |a, b| a - b, |a, b| a - b),
        "*" => arithmetic_op(left, right, |a, b| a * b, |a, b| a * b),
        "/" => {
            if matches!(right, Value::Integer(0) | Value::Float(0.0)) {
                Value::Null
            } else {
                arithmetic_op(left, right, |a, b| a / b, |a, b| a / b)
            }
        }
        // V312-22b / Issue #4036: modulo operator. Without this arm the
        // parser-built `BinaryOp(_, "%", _)` falls through to `_ => Value::Null`
        // below, so `WHERE i % 2 <> 0` evaluates as NULL and matches no rows
        // (SQL three-valued logic: NULL predicate is UNKNOWN, treated as false).
        // This broke TPC-H Q4-style `i % 2 <> 0` filtering in
        // sqllogictest/insert__test_insert.test.
        "%" => {
            if matches!(right, Value::Integer(0) | Value::Float(0.0)) {
                Value::Null
            } else {
                arithmetic_op(left, right, |a, b| a % b, |a, b| a % b)
            }
        }
        // TPC-H Q9: `WHERE p_name LIKE '%green%'`. The parser emits
        // `Expression::BinaryOp(left, "LIKE", right)`, so this branch is
        // the one that actually fires during WHERE evaluation. Both
        // sides are coerced to text for substring matching.
        "LIKE" => Value::Boolean(sql_like_match(
            &left.to_sql_string(),
            &right.to_sql_string(),
        )),
        _ => Value::Null,
    }
}

/// Helper that picks the right arithmetic arm based on the value types.
/// Returns Null if either side is Null or non-numeric.
fn arithmetic_op<F, G>(left: &Value, right: &Value, int_op: F, float_op: G) -> Value
where
    F: Fn(i64, i64) -> i64,
    G: Fn(f64, f64) -> f64,
{
    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(int_op(*a, *b)),
        (Value::Float(a), Value::Float(b)) => Value::Float(float_op(*a, *b)),
        (Value::Integer(a), Value::Float(b)) => Value::Float(float_op(*a as f64, *b)),
        (Value::Float(a), Value::Integer(b)) => Value::Float(float_op(*a, *b as f64)),
        _ => Value::Null,
    }
}

/// Compare two values — DEPRECATED, delegates to
/// `sqlrustgo_executor::expr::compare_values` (P0-2 §4.7).
///
/// Kept as a 1-line shim during the transition. `src/engine_utils.rs`
/// still calls this shim; a follow-up PR (per OpenSpec Decision D3)
/// will migrate `engine_utils.rs` to import the executor function
/// directly, at which point this shim can be removed.
pub fn compare_values(left: &Value, right: &Value) -> i32 {
    sqlrustgo_executor::expr::compare_values(left, right)
}

/// Evaluate expression to string (for GROUP BY key)
pub fn evaluate_expr_to_string(expr: &Expression, row: &[Value], table_info: &TableInfo) -> String {
    let val = evaluate_expression(expr, row, table_info).unwrap_or(Value::Null);
    match val {
        Value::Null => "NULL".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(s) => s,
        Value::Boolean(b) => b.to_string(),
        _ => "?".to_string(),
    }
}

/// Find the index of a column in the table info
/// For JOIN queries with combined tables, handles qualified names like "t2.id"
/// by routing to the correct portion of the combined schema.
/// Combined table naming: left_table.col, right_table.col
///
/// For multi-join chains, `build_combined_schema` accumulates prefixes
/// (e.g. an `a.tag` column becomes `a_join_b.a.tag` after a second join),
/// so the lookup also matches the user reference against the trailing
/// segments of the accumulated column name. `a.tag` still resolves to the
/// `a_join_b.a.tag` column, and bare `tag` resolves by its final segment.
#[allow(dead_code)] // P0-2 §4.10: shim is currently unused (the
                    // `evaluate_expression` Identifier arm delegates
                    // directly to `executor::expr::eval_identifier`).
                    // Kept during the transition; a follow-up PR
                    // will either remove it or migrate the few
                    // remaining callers.
pub(crate) fn find_column_index(col_name: &str, table_info: &TableInfo) -> Option<usize> {
    // P0-2 §4.10: delegated to `executor::expr::find_column_index`
    // (single source of truth for column-name resolution).
    sqlrustgo_executor::expr::find_column_index(col_name, &table_info.columns)
}

/// Walk an expression and rewrite `(SELECT ...)` subqueries into
/// `Literal` / `InList` / `NotInList` so `evaluate_expression` and
/// `evaluate_where_clause` (which don't know about subqueries) can
/// consume the result. `scalar_eval` / `list_eval` are called once
/// per encountered subquery and must produce the resolved scalar /
/// list value.
pub fn resolve_subqueries_in_expr<F, G>(
    expr: &Expression,
    scalar_eval: &F,
    list_eval: &G,
) -> Result<Expression, String>
where
    F: Fn(&SelectStatement) -> Result<Value, String>,
    G: Fn(&SelectStatement) -> Result<Vec<Value>, String>,
{
    use Expression::*;
    Ok(match expr {
        Subquery(subq) => Literal(scalar_eval(subq)?.to_sql_string()),
        In(left, subq) => InList(
            Box::new(resolve_subqueries_in_expr(left, scalar_eval, list_eval)?),
            list_eval_to_expressions(subq, list_eval)?,
        ),
        NotIn(left, subq) => NotInList(
            Box::new(resolve_subqueries_in_expr(left, scalar_eval, list_eval)?),
            list_eval_to_expressions(subq, list_eval)?,
        ),
        BinaryOp(l, op, r) => BinaryOp(
            Box::new(resolve_subqueries_in_expr(l, scalar_eval, list_eval)?),
            op.clone(),
            Box::new(resolve_subqueries_in_expr(r, scalar_eval, list_eval)?),
        ),
        UnaryOp(op, inner) => UnaryOp(
            op.clone(),
            Box::new(resolve_subqueries_in_expr(inner, scalar_eval, list_eval)?),
        ),
        CaseWhen(whens, else_val) => {
            let mut new_whens = Vec::with_capacity(whens.len());
            for w in whens {
                let cond = resolve_subqueries_in_expr(&w.condition, scalar_eval, list_eval)?;
                let res = resolve_subqueries_in_expr(&w.result, scalar_eval, list_eval)?;
                new_whens.push(WhenClause {
                    condition: cond,
                    result: res,
                });
            }
            let new_else = else_val
                .as_ref()
                .map(|e| resolve_subqueries_in_expr(e, scalar_eval, list_eval).map(Box::new))
                .transpose()?;
            CaseWhen(new_whens, new_else)
        }
        FunctionCall(name, args) => {
            let mut new_args = Vec::with_capacity(args.len());
            for a in args {
                new_args.push(resolve_subqueries_in_expr(a, scalar_eval, list_eval)?);
            }
            FunctionCall(name.clone(), new_args)
        }
        _ => expr.clone(),
    })
}

fn list_eval_to_expressions<G>(
    subq: &SelectStatement,
    list_eval: &G,
) -> Result<Vec<Expression>, String>
where
    G: Fn(&SelectStatement) -> Result<Vec<Value>, String>,
{
    let values = list_eval(subq)?;
    Ok(values
        .into_iter()
        .map(|v| Expression::Literal(v.to_sql_string()))
        .collect())
}
