//! Expression utility functions extracted from execution_engine.rs.
//!
//! Pure helper functions for evaluating SQL expressions against rows.
//! These functions do not depend on the `ExecutionEngine` struct and can be
//! used independently by any module needing expression evaluation.

use sqlrustgo_parser::Expression;
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
        },
        _ => "?".to_string(),
    }
}

/// Convert a parser Expression to a Value (simple literal evaluation)
pub fn expression_to_value(expr: &sqlrustgo_parser::Expression) -> Value {
    match expr {
        sqlrustgo_parser::Expression::Literal(s) => {
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
        sqlrustgo_parser::Expression::Identifier(name) => Value::Text(name.clone()),
        _ => Value::Null,
    }
}

/// Convert a string argument to a Value (for CALL arguments)
/// SQL LIKE pattern matcher. `%` matches any sequence (including empty),
/// `_` matches a single character; all other characters are literal.
/// Case-insensitive to match MySQL's default LIKE semantics. The
/// pattern's leading/trailing quotes (set by the literal parser) are
/// stripped before matching.
pub(crate) fn sql_like_match(text: &str, pattern: &str) -> bool {
    // Strip the surrounding single quotes that the literal parser
    // attaches to string values. `pattern` is usually passed in
    // already without quotes, but be defensive.
    let pat = pattern
        .trim()
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(pattern.trim());
    let txt = text.to_lowercase();
    let pat = pat.to_lowercase();
    like_match_recursive(&txt, &pat)
}

/// Recursive wildcard matcher. Walks the pattern character by character;
/// on `%` it tries matching the rest of the pattern against every
/// suffix of the remaining text. Pure recursive implementation; safe
/// for the small TPC-H patterns (`%green%`, etc.) but could be
/// O(len(text) * len(pat)) in the worst case. A DFA-based matcher
/// would scale better; the recursive version is fine for now.
fn like_match_recursive(text: &str, pattern: &str) -> bool {
    let mut t_idx = 0;
    let mut p_idx = 0;
    let t_bytes = text.as_bytes();
    let p_bytes = pattern.as_bytes();
    let mut star: Option<(usize, usize)> = None; // (text position, pattern position after %)

    while t_idx < t_bytes.len() {
        if p_idx < p_bytes.len() {
            match p_bytes[p_idx] {
                b'%' => {
                    // Record the position to backtrack to, then advance.
                    star = Some((t_idx, p_idx + 1));
                    p_idx += 1;
                    continue;
                }
                b'_' => {
                    t_idx += 1;
                    p_idx += 1;
                    continue;
                }
                c if c == t_bytes[t_idx] => {
                    t_idx += 1;
                    p_idx += 1;
                    continue;
                }
                _ => {
                    // Mismatch — if we have a prior `%`, backtrack: advance
                    // t_idx by one and restart matching from just after the
                    // saved position. (The saved `ts` is fixed, so we use
                    // t_idx + 1, not ts + 1, to actually make progress.)
                    if let Some((_, ps)) = star {
                        p_idx = ps;
                        t_idx += 1;
                        continue;
                    }
                    return false;
                }
            }
        } else {
            // Pattern exhausted but text has more. If we have a prior
            // `%`, backtrack and advance one more text position.
            if let Some((_, ps)) = star {
                p_idx = ps;
                t_idx += 1;
                continue;
            }
            return false;
        }
    }

    // Text exhausted; remaining pattern must be only `%`s.
    while p_idx < p_bytes.len() && p_bytes[p_idx] == b'%' {
        p_idx += 1;
    }
    p_idx == p_bytes.len()
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
    match expr {
        Expression::Literal(_) => Ok(expression_to_value(expr)),
        Expression::Identifier(name) => {
            if let Some(col_idx) = find_column_index(name, table_info) {
                Ok(row.get(col_idx).cloned().unwrap_or(Value::Null))
            } else {
                Ok(expression_to_value(expr))
            }
        }
        Expression::BinaryOp(left, op, right) => {
            let left_val = evaluate_expression(left, row, table_info).unwrap_or(Value::Null);
            let right_val = evaluate_expression(right, row, table_info).unwrap_or(Value::Null);
            Ok(evaluate_binary_op(&left_val, &right_val, op))
        }
        Expression::IsNull(inner) => {
            let val = evaluate_expression(inner, row, table_info)?;
            Ok(Value::Boolean(matches!(val, Value::Null)))
        }
        // TPC-H Q9: `WHERE p_name LIKE '%green%'`. Implement SQL LIKE
        // substring match: `%` matches any sequence (including empty),
        // `_` matches a single char. No ESCAPE handling yet; that's
        // a separate follow-up.
        Expression::Like(expr, pattern, _escape) => {
            // The parser folds `LIKE` into `Expression::Like(left, pat, _)`,
            // so this arm fires during WHERE evaluation. Implements the
            // same wildcard match that the BinaryOp("LIKE") arm goes
            // through for consistency.
            let val = evaluate_expression(expr, row, table_info)
                .map(|v| v.to_sql_string())
                .unwrap_or_default();
            let pat = evaluate_expression(pattern, row, table_info)
                .map(|v| v.to_sql_string())
                .unwrap_or_default();
            Ok(Value::Boolean(sql_like_match(&val, &pat)))
        }
        // Evaluate each WHEN's condition in order; the first one whose
        // value is Boolean(true) (or non-zero/non-null) wins, and we
        // return its THEN expression. If no WHEN matches and an ELSE
        // is present, return its value; otherwise Null. This matches
        // the executor's UnifiedExpr::CaseWhen semantics.
        Expression::CaseWhen(whens, else_val) => {
            for w in whens {
                let cond_val = evaluate_expression(&w.condition, row, table_info)?;
                if matches!(cond_val, Value::Boolean(true)) {
                    return evaluate_expression(&w.result, row, table_info);
                }
                // SQL CASE treats non-Boolean non-null values as truthy
                // when used as conditions; mirror that.
                if !matches!(cond_val, Value::Null | Value::Boolean(false)) {
                    return evaluate_expression(&w.result, row, table_info);
                }
            }
            match else_val {
                Some(e) => evaluate_expression(e, row, table_info),
                None => Ok(Value::Null),
            }
        }
        // TPC-H Q7/Q8/Q9: EXTRACT(field FROM col). The parser encodes this
        // as FunctionCall("EXTRACT", [Literal(field), source_expr]). We
        // dispatch on the field name and slice the source (which we expect
        // to be a Text date in YYYY-MM-DD form). Returns Null on shape
        // mismatch so a downstream operator can decide.
        Expression::FunctionCall(name, args) if name.to_uppercase() == "EXTRACT" => {
            let field = args
                .first()
                .map(|e| expression_to_value(e).to_sql_string().to_uppercase())
                .unwrap_or_default();
            let source = match args.get(1) {
                Some(e) => evaluate_expression(e, row, table_info)?.to_sql_string(),
                None => return Ok(Value::Null),
            };
            Ok(match field.as_str() {
                "YEAR" if source.len() >= 4 => Value::Text(source[..4].to_string()),
                "MONTH" if source.len() >= 7 => Value::Text(source[5..7].to_string()),
                "DAY" if source.len() >= 10 => Value::Text(source[8..10].to_string()),
                _ => Value::Null,
            })
        }
        Expression::Aggregate(agg) => {
            let agg_name = expression_to_string(&Expression::Aggregate(agg.clone()));
            if let Some(col_idx) = find_column_index(&agg_name, table_info) {
                Ok(row.get(col_idx).cloned().unwrap_or(Value::Null))
            } else {
                Err(format!("Aggregate not found in schema: {}", agg_name))
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
        Expression::NotLike(left, pattern, _escape) => {
            let lv = evaluate_expression(left, row, table_info)?;
            let pv = evaluate_expression(pattern, row, table_info)?;
            Ok(Value::Boolean(!sql_like_match(
                &lv.to_sql_string(),
                &pv.to_sql_string(),
            )))
        }
        // TPC-H Q1: expr BETWEEN low AND high.
        Expression::Between(expr, low, high) => {
            let v = evaluate_expression(expr, row, table_info)?;
            let lo = evaluate_expression(low, row, table_info)?;
            let hi = evaluate_expression(high, row, table_info)?;
            Ok(Value::Boolean(
                compare_values(&v, &lo) >= 0 && compare_values(&v, &hi) <= 0,
            ))
        }
        Expression::NotBetween(expr, low, high) => {
            let v = evaluate_expression(expr, row, table_info)?;
            let lo = evaluate_expression(low, row, table_info)?;
            let hi = evaluate_expression(high, row, table_info)?;
            Ok(Value::Boolean(
                !(compare_values(&v, &lo) >= 0 && compare_values(&v, &hi) <= 0),
            ))
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

/// Compare two values and return -1, 0, or 1
pub fn compare_values(left: &Value, right: &Value) -> i32 {
    match (left, right) {
        (Value::Integer(l), Value::Integer(r)) => l.cmp(r) as i32,
        (Value::Float(l), Value::Float(r)) => {
            if l < r {
                -1
            } else if l > r {
                1
            } else {
                0
            }
        }
        (Value::Text(l), Value::Text(r)) => l.cmp(r) as i32,
        (Value::Null, Value::Null) => 0,
        (Value::Null, _) => -1,
        (_, Value::Null) => 1,
        _ => 0,
    }
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
pub(crate) fn find_column_index(col_name: &str, table_info: &TableInfo) -> Option<usize> {
    // Fast path: exact match.
    if let Some(idx) = table_info
        .columns
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(col_name))
    {
        return Some(idx);
    }

    if let Some((_qualifier, col)) = col_name.split_once('.') {
        // Qualified: prefer the unqualified column-name match (works for the
        // first-JOIN case where columns are named `t.col`).
        if let Some(idx) = table_info
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col))
        {
            return Some(idx);
        }
        // Multi-join: the accumulated column may be `a_join_b.t.col`; match
        // when the user's `qualifier.col` is the trailing two segments.
        let user_segments: Vec<&str> = col_name.split('.').collect();
        for (i, c) in table_info.columns.iter().enumerate() {
            let col_segments: Vec<&str> = c.name.split('.').collect();
            if col_segments.len() >= user_segments.len()
                && col_segments[col_segments.len() - user_segments.len()..] == user_segments[..]
            {
                return Some(i);
            }
        }
        None
    } else {
        // Unqualified: try a trailing-segment match so bare `tag` still
        // resolves against the accumulated `a_join_b.a.tag`.
        for (i, c) in table_info.columns.iter().enumerate() {
            if let Some((_, tail)) = c.name.rsplit_once('.') {
                if tail.eq_ignore_ascii_case(col_name) {
                    return Some(i);
                }
            }
        }
        None
    }
}
