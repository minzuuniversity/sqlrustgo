//! Expression utility functions extracted from execution_engine.rs.
//!
//! Pure helper functions for evaluating SQL expressions against rows.
//! These functions do not depend on the `ExecutionEngine` struct and can be
//! used independently by any module needing expression evaluation.

use sqlrustgo_parser::parser::WhenClause;
use sqlrustgo_parser::Expression;
use sqlrustgo_parser::SelectStatement;
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;

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
    storage: Option<&mut dyn sqlrustgo_storage::StorageEngine>,
    subq_eval: &dyn Fn(&SelectStatement) -> Result<Value, String>,
) -> Result<Value, String> {
    if let Some(s) = storage {
        match expr {
            Expression::SequenceNextVal(name) => {
                return s
                    .next_sequence_value(name)
                    .map(Value::Integer)
                    .map_err(|e| format!("NEXT VALUE FOR {}: {}", name, e));
            }
            Expression::SequenceCurrval(name) => {
                // No public has_sequence_value in the trait; derive the
                // current value from get_sequence() (which returns
                // Some(SequenceInfo) for sequences that have been
                // created). CURRVAL semantics: return current_value
                // (the value most recently produced by NEXT_VALUE; the
                // engine has already advanced it on prior NEXT_VALUE_FOR).
                return match s.get_sequence(name) {
                    Some(info) => Ok(Value::Integer(info.current_value)),
                    None => Ok(Value::Null),
                };
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
        // a single char. No ESCAPE handling yet; that's a separate
        // follow-up.
        Expression::Like(expr, pattern, _escape) => {
            // P0-2 §4.5: delegated to `executor::expr::sql_like_match`
            // (single source of truth for the LIKE pattern matcher).
            let val = evaluate_expression(expr, row, table_info)
                .map(|v| v.to_sql_string())
                .unwrap_or_default();
            let pat = evaluate_expression(pattern, row, table_info)
                .map(|v| v.to_sql_string())
                .unwrap_or_default();
            Ok(Value::Boolean(sqlrustgo_executor::expr::sql_like_match(
                &val, &pat,
            )))
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
        Expression::NotLike(left, pattern, _escape) => {
            // P0-2 §4.6: delegated to `executor::expr::sql_like_match`.
            let lv = evaluate_expression(left, row, table_info)?;
            let pv = evaluate_expression(pattern, row, table_info)?;
            Ok(Value::Boolean(!sqlrustgo_executor::expr::sql_like_match(
                &lv.to_sql_string(),
                &pv.to_sql_string(),
            )))
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
