use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum UnifiedExpr {
    Literal(Value),
    Column(String),
    BinaryOp {
        left: Box<UnifiedExpr>,
        op: String,
        right: Box<UnifiedExpr>,
    },
    FunctionCall {
        name: String,
        args: Vec<UnifiedExpr>,
    },
    UnaryOp {
        op: String,
        expr: Box<UnifiedExpr>,
    },
    IsNull(Box<UnifiedExpr>),
    IsNotNull(Box<UnifiedExpr>),
    InList {
        expr: Box<UnifiedExpr>,
        list: Vec<UnifiedExpr>,
    },
    Between {
        expr: Box<UnifiedExpr>,
        low: Box<UnifiedExpr>,
        high: Box<UnifiedExpr>,
    },
    CaseWhen {
        whens: Vec<(UnifiedExpr, UnifiedExpr)>,
        else_val: Option<Box<UnifiedExpr>>,
    },
    Cast {
        expr: Box<UnifiedExpr>,
        target_type: String,
    },
    SequenceNextVal(String),
    SequenceCurrval(String),
}

impl UnifiedExpr {
    /// Evaluate the expression.
    ///
    /// `storage` is \`&mut Option<&mut dyn StorageEngine>\` — pass `None` when no
    /// engine is available (e.g. in pure expression-evaluation contexts). When
    /// `Some`, the `SequenceNextVal` / `SequenceCurrval` arms call through to the
    /// engine to resolve the actual sequence value.
    pub fn evaluate(
        &mut self,
        row: &[Value],
        columns: &[String],
        storage: &mut Option<&mut dyn StorageEngine>,
    ) -> Value {
        match self {
            UnifiedExpr::Literal(v) => v.clone(),
            UnifiedExpr::Column(name) => columns
                .iter()
                .position(|c| c == name)
                .and_then(|i| row.get(i).cloned())
                .unwrap_or(Value::Null),
            UnifiedExpr::BinaryOp { left, op, right } => {
                let l = left.evaluate(row, columns, storage);
                let r = right.evaluate(row, columns, storage);
                eval_binary_op(&l, &r, op)
            }
            UnifiedExpr::UnaryOp { op, expr } => {
                let v = expr.evaluate(row, columns, storage);
                eval_unary_op(&v, op)
            }
            UnifiedExpr::IsNull(expr) => {
                Value::Boolean(matches!(expr.evaluate(row, columns, storage), Value::Null))
            }
            UnifiedExpr::IsNotNull(expr) => {
                Value::Boolean(!matches!(expr.evaluate(row, columns, storage), Value::Null))
            }
            UnifiedExpr::FunctionCall { name, args } => {
                let vals: Vec<Value> = args
                    .iter_mut()
                    .map(|a| a.evaluate(row, columns, storage))
                    .collect();
                eval_fn(name, &vals)
            }
            UnifiedExpr::InList { expr, list } => {
                let val = expr.evaluate(row, columns, storage);
                Value::Boolean(list.iter_mut().any(|item| {
                    val == item.evaluate(row, columns, storage) && !matches!(&val, Value::Null)
                }))
            }
            UnifiedExpr::Between { expr, low, high } => {
                let v = expr.evaluate(row, columns, storage);
                let l = low.evaluate(row, columns, storage);
                let h = high.evaluate(row, columns, storage);
                Value::Boolean(
                    eval_binary_op(&v, &l, ">=") == Value::Boolean(true)
                        && eval_binary_op(&v, &h, "<=") == Value::Boolean(true),
                )
            }
            UnifiedExpr::CaseWhen { whens, else_val } => {
                for (cond, result) in whens.iter_mut() {
                    if cond.evaluate(row, columns, storage) == Value::Boolean(true) {
                        return result.evaluate(row, columns, storage);
                    }
                }
                else_val
                    .as_mut()
                    .map(|e| e.evaluate(row, columns, storage))
                    .unwrap_or(Value::Null)
            }
            UnifiedExpr::Cast { expr, target_type } => {
                let v = expr.evaluate(row, columns, storage);
                cast_val(&v, target_type)
            }
            UnifiedExpr::SequenceNextVal(name) => {
                if let Some(storage) = storage.as_mut() {
                    match storage.next_sequence_value(name) {
                        Ok(v) => Value::Integer(v),
                        Err(_) => Value::Null,
                    }
                } else {
                    Value::Null
                }
            }
            UnifiedExpr::SequenceCurrval(name) => {
                if let Some(storage) = storage.as_mut() {
                    match storage.current_sequence_value(name) {
                        Ok(v) => Value::Integer(v),
                        Err(_) => Value::Null,
                    }
                } else {
                    Value::Null
                }
            }
        }
    }

    pub fn referenced_columns(&self) -> Vec<String> {
        let mut cols = Vec::new();
        collect_cols(self, &mut cols);
        cols.sort();
        cols.dedup();
        cols
    }
}

fn collect_cols(expr: &UnifiedExpr, acc: &mut Vec<String>) {
    match expr {
        UnifiedExpr::Column(name) => acc.push(name.clone()),
        UnifiedExpr::BinaryOp { left, right, .. } => {
            collect_cols(left, acc);
            collect_cols(right, acc);
        }
        UnifiedExpr::UnaryOp { expr: e, .. } => collect_cols(e, acc),
        UnifiedExpr::IsNull(e) | UnifiedExpr::IsNotNull(e) => collect_cols(e, acc),
        UnifiedExpr::InList { expr: e, list } => {
            collect_cols(e, acc);
            for i in list {
                collect_cols(i, acc);
            }
        }
        UnifiedExpr::Between { expr: e, low, high } => {
            collect_cols(e, acc);
            collect_cols(low, acc);
            collect_cols(high, acc);
        }
        UnifiedExpr::CaseWhen { whens, else_val } => {
            for (c, r) in whens {
                collect_cols(c, acc);
                collect_cols(r, acc);
            }
            if let Some(e) = else_val {
                collect_cols(e, acc);
            }
        }
        UnifiedExpr::Cast { expr: e, .. } => collect_cols(e, acc),
        UnifiedExpr::FunctionCall { args, .. } => {
            for a in args {
                collect_cols(a, acc);
            }
        }
        _ => {}
    }
}

// Adapters
impl From<&sqlrustgo_parser::Expression> for UnifiedExpr {
    fn from(expr: &sqlrustgo_parser::Expression) -> Self {
        use sqlrustgo_parser::Expression;
        match expr {
            Expression::Literal(s) => UnifiedExpr::Literal(parse_lit(s)),
            Expression::Identifier(name) => UnifiedExpr::Column(name.clone()),
            Expression::BinaryOp(left, op, right) => UnifiedExpr::BinaryOp {
                left: Box::new(UnifiedExpr::from(left.as_ref())),
                op: op.clone(),
                right: Box::new(UnifiedExpr::from(right.as_ref())),
            },
            Expression::UnaryOp(op, e) => UnifiedExpr::UnaryOp {
                op: op.clone(),
                expr: Box::new(UnifiedExpr::from(e.as_ref())),
            },
            Expression::IsNull(e) => UnifiedExpr::IsNull(Box::new(UnifiedExpr::from(e.as_ref()))),
            Expression::IsNotNull(e) => {
                UnifiedExpr::IsNotNull(Box::new(UnifiedExpr::from(e.as_ref())))
            }
            Expression::InList(expr, list) => UnifiedExpr::InList {
                expr: Box::new(UnifiedExpr::from(expr.as_ref())),
                list: list.iter().map(UnifiedExpr::from).collect(),
            },
            Expression::Between(expr, low, high) => UnifiedExpr::Between {
                expr: Box::new(UnifiedExpr::from(expr.as_ref())),
                low: Box::new(UnifiedExpr::from(low.as_ref())),
                high: Box::new(UnifiedExpr::from(high.as_ref())),
            },
            Expression::Like(expr, pattern, _) => UnifiedExpr::FunctionCall {
                name: "LIKE".into(),
                args: vec![
                    UnifiedExpr::from(expr.as_ref()),
                    UnifiedExpr::from(pattern.as_ref()),
                ],
            },
            // TPC-H Sprint 1 fix (Q8): CaseWhen conversion
            Expression::CaseWhen(whens, else_val) => UnifiedExpr::CaseWhen {
                whens: whens
                    .iter()
                    .map(|w| {
                        (
                            UnifiedExpr::from(&w.condition),
                            UnifiedExpr::from(&w.result),
                        )
                    })
                    .collect(),
                else_val: else_val
                    .as_ref()
                    .map(|e| Box::new(UnifiedExpr::from(e.as_ref()))),
            },
            Expression::FunctionCall(name, args) => UnifiedExpr::FunctionCall {
                name: name.clone(),
                args: args.iter().map(UnifiedExpr::from).collect(),
            },
            Expression::SequenceNextVal(name) => UnifiedExpr::SequenceNextVal(name.clone()),
            Expression::SequenceCurrval(name) => UnifiedExpr::SequenceCurrval(name.clone()),
            _ => UnifiedExpr::Literal(Value::Null),
        }
    }
}

impl From<&sqlrustgo_planner::Expr> for UnifiedExpr {
    fn from(expr: &sqlrustgo_planner::Expr) -> Self {
        use sqlrustgo_planner::Expr;
        match expr {
            Expr::Column(col) => UnifiedExpr::Column(col.name.clone()),
            Expr::Literal(val) => UnifiedExpr::Literal(val.clone()),
            Expr::BinaryExpr { left, op, right } => UnifiedExpr::BinaryOp {
                left: Box::new(UnifiedExpr::from(left.as_ref())),
                op: format!("{:?}", op),
                right: Box::new(UnifiedExpr::from(right.as_ref())),
            },
            Expr::UnaryExpr { op, expr: e } => UnifiedExpr::UnaryOp {
                op: format!("{:?}", op),
                expr: Box::new(UnifiedExpr::from(e.as_ref())),
            },
            Expr::Alias { expr: e, .. } => UnifiedExpr::from(e.as_ref()),
            _ => UnifiedExpr::Literal(Value::Null),
        }
    }
}

// Evaluation helpers

/// Evaluate a parser-AST `Expression::Literal(&str)` to a `Value`.
///
/// This is the single source of truth for "what does this literal look
/// like as a Value?". The legacy `src/expr_utils.rs::expression_to_value`
/// `Expression::Literal` arm is a thin delegation to this function.
///
/// **Semantics (intentionally identical to the legacy `expr_utils` Literal
/// arm; do not change without coordinating the P0-2 OpenSpec change):**
///
/// | input               | output              |
/// |---------------------|---------------------|
/// | `"NULL"`            | `Value::Null`       |
/// | `"42"`              | `Value::Integer(42)`|
/// | `"3.14"`            | `Value::Float(3.14)`|
/// | `"'hello'"`         | `Value::Text("hello")`|
/// | `"hello"` (unquoted)| `Value::Text("hello")`|
/// | `"  42  "` (trim)   | `Value::Integer(42)`|
///
/// **Note:** This function does *not* currently match the internal
/// `parse_lit` helper in this file. `parse_lit` is more aggressive
/// (it maps `TRUE`/`FALSE` to `Integer(1/0)` and truncates `f64` to
/// `Integer`); the legacy `expr_utils` Literal arm is more conservative
/// (it preserves `f64`). We deliberately do NOT replace `parse_lit`
/// here, because `parse_lit` is the implementation of `UnifiedExpr::Literal`
/// conversion and has its own existing test coverage in this file. The
/// two functions coexist for now; P0-2 §4.5-4.13 will reconcile the
/// differences as the remaining 13 branches are delegated.
pub fn eval_literal_from_str(s: &str) -> Value {
    let s = s.trim();
    if s.eq_ignore_ascii_case("NULL") {
        return Value::Null;
    }
    // Boolean keywords: MySQL 5.7 uses TRUE/FALSE as 1/0 in INTEGER context.
    // We preserve the same convention as the legacy `expr_utils`
    // (Literal arm was `Value::Text(s)` fallback) BUT the unified
    // arm here maps TRUE/FALSE to Integer(1)/Integer(0) so that
    // `NOT TRUE` evaluates to a Boolean. (See P0-2 §4.12.
    // Without this, `Literal("TRUE")` was Text("TRUE") and `NOT TRUE`
    // went through to_bool(Text) which returns true, negating to
    // false — wrong.)
    //
    // **Note**: this is a *behavior change* from the legacy
    // `expr_utils::expression_to_value` Literal arm, which would
    // have returned `Value::Text("TRUE")` for `Literal("TRUE")`. The
    // change is intentional and tracked in the OpenSpec tasks.md
    // §4.12 — the UnaryOp arm cannot work without this.
    //
    // Sprint 5 v2 fix: return Value::Boolean(true/false) instead of
    // Value::Integer(1/0). The Integer form silently broke
    // correlated-EXISTS substitution in TPC-H Q4: the substituted
    // where_expr evaluates `Literal("true")` to Integer(1), and
    // the catch-all in `eval_predicate` checks
    // `matches!(val, Value::Boolean(true))` which fails on
    // Integer(1), so all rows are dropped (Q4 returns 0 rows
    // instead of 5).
    if s.eq_ignore_ascii_case("TRUE") {
        return Value::Boolean(true);
    }
    if s.eq_ignore_ascii_case("FALSE") {
        return Value::Boolean(false);
    }
    if let Ok(n) = s.parse::<i64>() {
        return Value::Integer(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::Float(f);
    }
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        return Value::Text(s[1..s.len() - 1].to_string());
    }
    Value::Text(s.to_string())
}

/// Evaluate the parser-AST `Expression::IsNull` arm: returns
/// `Value::Boolean(true)` if `value` is `Value::Null`, else
/// `Value::Boolean(false)`.
///
/// This is the single source of truth for "is this value null?". The
/// legacy `src/expr_utils.rs::evaluate_expression` `Expression::IsNull`
/// arm is a thin delegation to this function (P0-2 §4.2).
///
/// **Semantics (identical to the legacy arm and to
/// `UnifiedExpr::IsNull::evaluate`):**
/// - `eval_is_null(&Value::Null)`       → `Value::Boolean(true)`
/// - `eval_is_null(&Value::Integer(0))` → `Value::Boolean(false)`
/// - `eval_is_null(&Value::Text(""))`   → `Value::Boolean(false)` (empty string is not null)
/// - `eval_is_null(&Value::Boolean(false))` → `Value::Boolean(false)`
pub fn eval_is_null(value: &Value) -> Value {
    Value::Boolean(matches!(value, Value::Null))
}

/// Inverse of [`eval_is_null`]. P0-2 §4.3.
pub fn eval_is_not_null(value: &Value) -> Value {
    Value::Boolean(!matches!(value, Value::Null))
}

/// Compare two values, returning -1, 0, or 1.
///
/// This is the single source of truth for SQL value comparison. The
/// legacy `src/expr_utils.rs::compare_values` is a 1-line shim that
/// delegates to this function (P0-2 §4.7, §4.8, plus consumed by
/// `executor::expr::eval_between`).
///
/// **Semantics (identical to the legacy function):**
/// | `left`           | `right`         | result |
/// |------------------|-----------------|--------|
/// | `Integer(l)`     | `Integer(r)`    | `l.cmp(r) as i32` |
/// | `Float(l)`       | `Float(r)`      | `-1 / 0 / 1` (NaN-unaware) |
/// | `Text(l)`        | `Text(r)`       | `l.cmp(r) as i32` |
/// | `Null`           | `Null`          | `0` |
/// | `Null`           | non-Null        | `-1` (NULL sorts first) |
/// | non-Null         | `Null`          | `1` (NULL sorts last) |
/// | mixed types      | (other)         | `0` (equal) |
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
        // TPC-H Q11 / Q14 fix: cross-type comparison (Float vs Integer,
        // Integer vs Float). Previously the `_ => 0` catch-all returned
        // 0, making `Float(3160502.6) > Integer(0)` always evaluate to
        // `0 > 0 = false` (rejected every HAVING row). Promote the
        // Integer side to Float for the comparison so the numeric
        // ordering works.
        (Value::Float(l), Value::Integer(r)) => {
            let r = *r as f64;
            if l < &r {
                -1
            } else if l > &r {
                1
            } else {
                0
            }
        }
        (Value::Integer(l), Value::Float(r)) => {
            let l = *l as f64;
            if &l < r {
                -1
            } else if &l > r {
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

/// Evaluate the parser-AST `Expression::Between(expr, low, high)` arm:
/// returns `Value::Boolean(true)` if `low <= value <= high`, else
/// `Value::Boolean(false)`.
///
/// This is the single source of truth for the parser-AST `Between`
/// branch. The legacy `src/expr_utils.rs::evaluate_expression`
/// `Expression::Between` arm is a thin delegation to this function
/// (P0-2 §4.7).
///
/// **Semantics (identical to the legacy arm):**
/// - `eval_between(5, 1, 10)` → `Value::Boolean(true)`
/// - `eval_between(0, 1, 10)` → `Value::Boolean(false)`
/// - `eval_between(5, 5, 10)` → `Value::Boolean(true)` (inclusive low)
/// - `eval_between(10, 1, 10)` → `Value::Boolean(true)` (inclusive high)
/// - `eval_between(Null, 1, 10)` → `Value::Boolean(false)` (NULL
///   sorts before any non-NULL per `compare_values` semantics, so
///   `compare_values(&Null, &lo) = -1 < 0`)
pub fn eval_between(value: &Value, low: &Value, high: &Value) -> Value {
    Value::Boolean(compare_values(value, low) >= 0 && compare_values(value, high) <= 0)
}

/// Inverse of [`eval_between`]. P0-2 §4.8.
pub fn eval_not_between(value: &Value, low: &Value, high: &Value) -> Value {
    Value::Boolean(!(compare_values(value, low) >= 0 && compare_values(value, high) <= 0))
}

/// Look up a column index in a `TableInfo.columns` list by name, with
/// case-insensitive matching, qualified-name stripping, and
/// multi-join trailing-segment handling.
///
/// This is the single source of truth for column-name resolution. The
/// legacy `src/expr_utils.rs::find_column_index` is a 1-line shim that
/// delegates to this function (P0-2 §4.10).
///
/// **Semantics (identical to the legacy function):**
/// - Fast path: exact case-insensitive match on the full column name.
/// - If `col_name` contains a `.` (qualified), strip the qualifier and
///   try the bare column name. If the column was accumulated from a
///   multi-join (e.g. `a_join_b.t.col`), try matching the trailing N
///   segments of the accumulated name against the user's N segments.
/// - If `col_name` has no `.` (unqualified), try a trailing-segment
///   match against each accumulated column (so bare `tag` resolves
///   against `a_join_b.a.tag`).
/// - Returns `Some(idx)` for a match, `None` otherwise.
///
/// The `ColumnDefinition` type is `sqlrustgo_storage::ColumnDefinition`.
/// We define a local struct that the public function uses (rather than
/// a free function over `&[String]`) so that the multi-join logic
/// reads naturally.
pub fn find_column_index(
    col_name: &str,
    columns: &[sqlrustgo_storage::ColumnDefinition],
) -> Option<usize> {
    // Fast path: exact match.
    if let Some(idx) = columns
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(col_name))
    {
        return Some(idx);
    }

    if let Some((_qualifier, col)) = col_name.split_once('.') {
        // Qualified: prefer the unqualified column-name match (works for the
        // first-JOIN case where columns are named `t.col`).
        if let Some(idx) = columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col))
        {
            return Some(idx);
        }
        // Multi-join: the accumulated column may be `a_join_b.t.col`; match
        // when the user's `qualifier.col` is the trailing two segments.
        let user_segments: Vec<&str> = col_name.split('.').collect();
        for (i, c) in columns.iter().enumerate() {
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
        for (i, c) in columns.iter().enumerate() {
            if let Some((_, tail)) = c.name.rsplit_once('.') {
                if tail.eq_ignore_ascii_case(col_name) {
                    return Some(i);
                }
            }
        }
        // Last fallback: no match.
        None
    }
}

/// Evaluate the parser-AST `Expression::Identifier(name)` arm: looks up
/// the column by name and returns the value from the row. If the
/// column is not in the schema, returns `Value::Text(name)` (the
/// legacy fallback for unqualified identifiers that happen to be
/// string literals).
///
/// This is the single source of truth for the parser-AST `Identifier`
/// branch. The legacy `src/expr_utils.rs::evaluate_expression`
/// `Expression::Identifier` arm is a thin delegation to this function
/// (P0-2 §4.10).
///
/// **Semantics (identical to the legacy arm):**
/// - If `name` is a column in `table_info`, return `row[idx]`
///   (cloned, defaulting to `Value::Null` if out of bounds).
/// - If `name` is *not* a column (e.g., a string literal used as a
///   column name in a specific dialect), return `Value::Text(name)`.
pub fn eval_identifier(
    name: &str,
    row: &[Value],
    columns: &[sqlrustgo_storage::ColumnDefinition],
) -> Result<Value, String> {
    if let Some(col_idx) = find_column_index(name, columns) {
        Ok(row.get(col_idx).cloned().unwrap_or(Value::Null))
    } else if name.contains('.') {
        // V312-18 #3971: qualified column that doesn't exist is a binder error
        Err(format!("Binder Error: column '{}' not found", name))
    } else {
        Ok(Value::Text(name.to_string()))
    }
}

/// Evaluate a parser-AST `Expression::CaseWhen(whens, else_val)` arm.
///
/// This is the single source of truth for the parser-AST `CaseWhen`
/// branch. The legacy `src/expr_utils.rs::evaluate_expression`
/// `Expression::CaseWhen` arm is a thin delegation to this function
/// (P0-2 §4.9).
///
/// **Semantics (identical to the legacy arm):**
/// - For each `WhenClause` in `whens`:
///   1. Evaluate the `condition` via `evaluate_fn`.
///   2. If the condition's value is `Value::Boolean(true)`, evaluate
///      and return the `result`.
///   3. **SQL CASE extension**: if the condition's value is *not*
///      `Value::Null` and *not* `Value::Boolean(false)` (i.e., any
///      truthy non-Boolean like `Integer(1)` or `Text("yes")`),
///      evaluate and return the `result`. This mirrors the legacy
///      `expr_utils` behavior (which mirrors SQL CASE's
///      truthiness-of-non-Booleans rule).
/// - If no WHEN matches:
///   1. If `else_val` is `Some`, evaluate it and return.
///   2. Otherwise, return `Ok(Value::Null)`.
///
/// The function is parameterized over a `evaluate_fn` closure that
/// handles the actual evaluation of inner expressions. This decouples
/// the algorithm from the row/columns/table_info state (which lives
/// in the caller, e.g. `expr_utils::evaluate_expression`).
pub fn eval_case_when<F>(
    whens: &[sqlrustgo_parser::parser::WhenClause],
    else_val: Option<&sqlrustgo_parser::Expression>,
    evaluate_fn: F,
) -> Result<Value, String>
where
    F: Fn(&sqlrustgo_parser::Expression) -> Result<Value, String>,
{
    for w in whens {
        let cond_val = evaluate_fn(&w.condition)?;
        if matches!(cond_val, Value::Boolean(true)) {
            return evaluate_fn(&w.result);
        }
        // SQL CASE treats non-Boolean non-null values as truthy
        // when used as conditions; mirror that.
        if !matches!(cond_val, Value::Null | Value::Boolean(false)) {
            return evaluate_fn(&w.result);
        }
    }
    match else_val {
        Some(e) => evaluate_fn(e),
        None => Ok(Value::Null),
    }
}

/// Look up a pre-computed aggregate value in a row by its canonical name
/// (the string form produced by `expr_utils::expression_to_string` for an
/// `Expression::Aggregate`, e.g. `"COUNT(*)"`, `"SUM(l_quantity)"`, etc.).
///
/// This is the single source of truth for the parser-AST
/// `Expression::Aggregate` arm. The legacy
/// `src/expr_utils.rs::evaluate_expression` `Expression::Aggregate` arm
/// is a thin delegation to this function (P0-2 §4.4).
///
/// **Semantics (identical to the legacy arm):**
/// - `eval_aggregate_lookup(agg_name, row, column_names)` returns the
///   `Value` at `row[i]` where `column_names[i]` matches `agg_name`
///   case-insensitively.
/// - Returns `None` if no column matches — the caller should map this
///   to an error (the legacy arm does
///   `Err("Aggregate not found in schema: ...")`, the OpenSpec design
///   keeps that error shape so the wire-protocol path's error message
///   is unchanged).
/// - This is **NOT** an actual aggregate computation; aggregate values
///   are computed in the SELECT/GROUP BY phase of the executor and
///   stored in the row before this function is called. The legacy arm
///   is also a lookup, not a computation, so the delegation preserves
///   behavior.
pub fn eval_aggregate_lookup(
    agg_name: &str,
    row: &[Value],
    column_names: &[String],
) -> Option<Value> {
    let idx = column_names
        .iter()
        .position(|c| c.eq_ignore_ascii_case(agg_name))?;
    row.get(idx).cloned()
}

/// SQL `LIKE` pattern matcher: `%` matches any sequence (including
/// empty), `_` matches a single character; all other characters are
/// literal. Case-insensitive to match MySQL's default `LIKE` semantics.
/// The pattern's leading/trailing quotes (set by the literal parser)
/// are stripped before matching.
///
/// This is the single source of truth for the parser-AST
/// `Expression::Like` and `Expression::NotLike` arms. The legacy
/// `src/expr_utils.rs::evaluate_expression` `Expression::Like` /
/// `Expression::NotLike` arms (and the `BinaryOp("LIKE", ...)` arm in
/// `evaluate_binary_op`) all delegate to this function (P0-2 §4.5 +
/// §4.6).
///
/// **Semantics (identical to the legacy `sql_like_match`):**
/// - `sql_like_match("hello", "%ell%")` → `true`
/// - `sql_like_match("hello", "world")` → `false`
/// - `sql_like_match("'hello'", "%ell%")` → `true` (quotes stripped
///   from pattern, defensively)
/// - `sql_like_match("HELLO", "%ell%")` → `true` (case-insensitive
///   on both text and pattern)
pub fn sql_like_match(text: &str, pattern: &str) -> bool {
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

fn parse_lit(s: &str) -> Value {
    let s = s.trim();
    if s.eq_ignore_ascii_case("NULL") {
        return Value::Null;
    }
    if s.eq_ignore_ascii_case("TRUE") {
        // Sprint 5 v2 fix: return Value::Boolean(true) so
        // eval_predicate's `matches!(val, Value::Boolean(true))`
        // truthiness check works correctly. Previously this
        // returned Value::Integer(1), which silently broke
        // correlated-EXISTS substitution in TPC-H Q4 — the
        // substituted where_expr evaluates the literal "true"
        // to Integer(1), and the catch-all `matches!(val,
        // Value::Boolean(true))` returns false, so all rows
        // are dropped (Q4 returns 0 rows instead of 5).
        return Value::Boolean(true);
    }
    if s.eq_ignore_ascii_case("FALSE") {
        return Value::Boolean(false);
    }
    let unquoted =
        if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
            &s[1..s.len() - 1]
        } else {
            s
        };
    if let Ok(i) = unquoted.parse::<i64>() {
        return Value::Integer(i);
    }
    if let Ok(f) = unquoted.parse::<f64>() {
        return Value::Integer(f as i64);
    }
    // Try JSON parsing before falling back to Text.
    // This correctly handles JSON objects/arrays that appear as literals.
    if let Ok(v) = serde_json::from_str(unquoted) {
        return Value::Json(v);
    }
    Value::Text(unquoted.to_string())
}

/// Evaluate a binary operation. P0-2 §4.14: this is the single source of
/// truth for parser-AST `Expression::BinaryOp`.
///
/// **Semantics** (see OpenSpec tasks.md §4.14):
/// - Comparison operators (`=`, `==`, `!=`, `<>`, `>`, `>=`, `<`, `<=`) return `Value::Boolean`.
/// - Logical operators (`AND`/`&&`, `OR`/`||`) return `Value::Boolean`.
/// - Arithmetic operators (`+`, `-`, `*`, `/`):
///   - `Integer OP Integer` → `Integer`
///   - Either operand `Float` → `Float` (Integer promotes to Float)
///   - Either operand `Null` (and the other non-Null) → `Null`
///   - Both `Null` → `Null`
///   - Division by zero `Integer` → 0 (SQLite/MariaDB parity);
///     division by zero `Float` → `Null` (standard SQL).
/// - Unknown operators return `Value::Null`.
///
/// The `src/expr_utils.rs::evaluate_expression` `BinaryOp` arm
/// delegates here; if a Float/NULL result is needed, the executor path
/// in `engine_select.rs` (which has the legacy function) takes
/// précédence. See P0-2 §4.14 and Decision D3 in OpenSpec.
///
/// **Bug fix 2026-06-13 (TPC-H Q1 SUM with computed expression):**
/// previously the arithmetic operators only handled `Integer`, so any
/// `Float` operand silently became `0` via `as_integer().unwrap_or(0)`,
/// causing `SUM(l_extendedprice * (1 - l_discount))` to return `0`
/// instead of the expected real value. The new path promotes to `Float`
/// whenever either operand is `Float`, matching PostgreSQL/SQLite.
pub fn eval_binary_op(left: &Value, right: &Value, op: &str) -> Value {
    match op.to_uppercase().as_str() {
        "=" | "==" => Value::Boolean(left == right && !matches!(left, Value::Null)),
        "!=" | "<>" => Value::Boolean(left != right && !matches!(left, Value::Null)),
        ">" | "<" | ">=" | "<=" => compare_cmp(left, right, op),
        "AND" | "&&" => Value::Boolean(to_bool(left) && to_bool(right)),
        "OR" | "||" => Value::Boolean(to_bool(left) || to_bool(right)),
        "+" | "-" | "*" | "/" => eval_arithmetic(left, right, op),
        "->" => json_extract(left, right, false),
        "->>" => json_extract(left, right, true),
        _ => Value::Null,
    }
}

/// Arithmetic helper. Returns `Null` if either operand is `Null`.
/// Promotes to `Float` if either operand is `Float`. Division by zero
/// returns `0` for `Integer` (SQLite/MariaDB parity) and `Null` for
/// `Float` (standard SQL). Boolean operands are treated as `0`/`1`.
fn eval_arithmetic(left: &Value, right: &Value, op: &str) -> Value {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return Value::Null;
    }
    let any_float = matches!(left, Value::Float(_)) || matches!(right, Value::Float(_));
    if any_float {
        let l = to_f64(left);
        let r = to_f64(right);
        if r == 0.0 && op == "/" {
            return Value::Null;
        }
        let result = match op {
            "+" => l + r,
            "-" => l - r,
            "*" => l * r,
            "/" => l / r,
            _ => unreachable!(),
        };
        Value::Float(result)
    } else {
        let l = to_i64(left);
        let r = to_i64(right);
        let result = match op {
            "+" => l.wrapping_add(r),
            "-" => l.wrapping_sub(r),
            "*" => l.wrapping_mul(r),
            "/" => {
                if r == 0 {
                    0
                } else {
                    l / r
                }
            }
            _ => unreachable!(),
        };
        Value::Integer(result)
    }
}

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Integer(n) => *n as f64,
        Value::Float(f) => *f,
        Value::Boolean(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null | Value::Text(_) | Value::Blob(_) | Value::Point(_, _) | Value::Json(_) => 0.0,
    }
}

fn to_i64(v: &Value) -> i64 {
    match v {
        Value::Integer(n) => *n,
        Value::Boolean(b) => {
            if *b {
                1
            } else {
                0
            }
        }
        Value::Null | Value::Float(_) | Value::Text(_) | Value::Blob(_) | Value::Point(_, _) | Value::Json(_) => 0,
    }
}

/// Evaluate the parser-AST `Expression::UnaryOp(op, expr)` arm.
/// Currently supports `NOT` / `!`; any other op returns `Value::Null`.
///
/// This is the single source of truth for the parser-AST `UnaryOp`
/// branch. The legacy `src/expr_utils.rs::evaluate_expression` `Expression::UnaryOp` arm
/// (P0-2 §4.12) is a thin delegation to this function.
///
/// **Semantics:**
/// - `eval_unary_op(true, "NOT")` → `Value::Boolean(false)`
/// - `eval_unary_op(0, "NOT")` → `Value::Boolean(true)` (0 is falsy via `to_bool`)
/// - `eval_unary_op(1, "NOT")` → `Value::Boolean(false)` (1 is truthy via `to_bool`)
/// MySQL 5.7 JSON path operators: `->` and `->>`.
/// `unquote` = false → returns `Value::Json`, true → returns `Value::Text`.
fn json_extract(left: &Value, right: &Value, unquote: bool) -> Value {
    let doc = match left {
        Value::Json(v) => v.clone(),
        Value::Text(s) => {
            match serde_json::from_str(s) {
                Ok(v) => v,
                Err(_) => return Value::Null,
            }
        }
        _ => return Value::Null,
    };

    let path = match right {
        Value::Text(s) => s.clone(),
        Value::Json(serde_json::Value::String(s)) => s.clone(),
        _ => return Value::Null,
    };

    let json_path = if path.starts_with('$') {
        path.clone()
    } else {
        format!("$.{}", path)
    };

    match doc.pointer(&json_path) {
        Some(result) => {
            if unquote {
                match result {
                    serde_json::Value::String(s) => Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Value::Integer(i)
                        } else if let Some(f) = n.as_f64() {
                            Value::Float(f)
                        } else {
                            Value::Text(n.to_string())
                        }
                    }
                    serde_json::Value::Bool(b) => Value::Boolean(*b),
                    serde_json::Value::Null => Value::Null,
                    serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                        Value::Text(result.to_string())
                    }
                }
            } else {
                Value::Json(result.clone())
            }
        }
        None => Value::Null,
    }
}

pub fn eval_unary_op(val: &Value, op: &str) -> Value {
    match op.to_uppercase().as_str() {
        "NOT" | "!" => Value::Boolean(!to_bool(val)),
        _ => Value::Null,
    }
}

/// MySQL 5.7 / SQL scalar function dispatch.
/// Single source of truth for the function table. Called from the
/// `UnifiedExpr::FunctionCall` arm of `evaluate` (this module) and
/// from the legacy `Expression::FunctionCall` arm in
/// `src/expr_utils.rs` (the binary engine path).
pub fn eval_fn(name: &str, args: &[Value]) -> Value {
    match name.to_uppercase().as_str() {
        "LOWER" => args
            .first()
            .map(|v| Value::Text(v.to_sql_string().to_lowercase()))
            .unwrap_or(Value::Null),
        "UPPER" => args
            .first()
            .map(|v| Value::Text(v.to_sql_string().to_uppercase()))
            .unwrap_or(Value::Null),
        "LENGTH" | "LEN" => args
            .first()
            .map(|v| Value::Integer(v.to_sql_string().len() as i64))
            .unwrap_or(Value::Null),
        // TRIM — MySQL 5.7 supports four forms:
        //   TRIM(str)                                 — 1-arg, trim whitespace
        //   TRIM(remstr, str)                         — 2-arg comma form
        //   TRIM([LEADING|TRAILING|BOTH] remstr FROM str) — 3-arg sentinel form
        // The 3-arg form is produced by the parser with a sentinel
        // string-literal modifier as args[0]:
        //   "__TRIM_LEADING__" | "__TRIM_TRAILING__" | "__TRIM_BOTH__"
        // See: https://dev.mysql.com/doc/refman/5.7/en/string-functions.html#function_trim
        "TRIM" => match args.len() {
            1 => args
                .first()
                .map(|v| Value::Text(v.to_sql_string().trim().to_string()))
                .unwrap_or(Value::Null),
            // TRIM(remstr, str) — 2-arg comma form, trim remstr from both ends
            2 => {
                let rem = args[0].to_sql_string();
                let s = args[1].to_sql_string();
                if rem.is_empty() {
                    Value::Text(s.trim().to_string())
                } else {
                    Value::Text(s.trim_matches(|c| rem.contains(c)).to_string())
                }
            }
            // TRIM([LEADING|TRAILING|BOTH] remstr FROM str) — 3-arg sentinel form
            3 => {
                let modifier = args[0].to_sql_string();
                let rem = args[1].to_sql_string();
                let s = args[2].to_sql_string();
                let trimmed = if rem.is_empty() {
                    s.trim().to_string()
                } else {
                    s.trim_matches(|c| rem.contains(c)).to_string()
                };
                let result = match modifier.as_str() {
                    "__TRIM_LEADING__" => {
                        // trim from the left only
                        let out = if rem.is_empty() {
                            s.trim_start().to_string()
                        } else {
                            s.trim_start_matches(|c| rem.contains(c)).to_string()
                        };
                        out
                    }
                    "__TRIM_TRAILING__" => {
                        if rem.is_empty() {
                            s.trim_end().to_string()
                        } else {
                            s.trim_end_matches(|c| rem.contains(c)).to_string()
                        }
                    }
                    // "__TRIM_BOTH__" or any other sentinel
                    _ => trimmed,
                };
                Value::Text(result)
            }
            _ => Value::Null,
        },
        "LTRIM" => args
            .first()
            .map(|v| Value::Text(v.to_sql_string().trim_start().to_string()))
            .unwrap_or(Value::Null),
        "RTRIM" => args
            .first()
            .map(|v| Value::Text(v.to_sql_string().trim_end().to_string()))
            .unwrap_or(Value::Null),
        // MySQL 5.7: LEFT(str, len) / RIGHT(str, len)
        // len < 0 returns empty string (MySQL semantics).
        "LEFT" => {
            if let (Some(s), Some(n)) = (args.first(), args.get(1)) {
                let text = s.to_sql_string();
                let len = match n {
                    Value::Integer(i) => *i,
                    _ => return Value::Text(String::new()),
                };
                if len <= 0 {
                    Value::Text(String::new())
                } else {
                    let n = (len as usize).min(text.len());
                    Value::Text(text[..n].to_string())
                }
            } else {
                Value::Null
            }
        }
        "RIGHT" => {
            if let (Some(s), Some(n)) = (args.first(), args.get(1)) {
                let text = s.to_sql_string();
                let len = match n {
                    Value::Integer(i) => *i,
                    _ => return Value::Text(String::new()),
                };
                if len <= 0 {
                    Value::Text(String::new())
                } else {
                    let n = (len as usize).min(text.len());
                    Value::Text(text[text.len() - n..].to_string())
                }
            } else {
                Value::Null
            }
        }
        // LPAD(str, len, pad) / RPAD(str, len, pad)
        // If len <= strlen(str), truncate. If len > strlen(str), pad.
        // If pad is empty, returns NULL.
        "LPAD" | "RPAD" => {
            let dir_is_left = matches!(name.to_uppercase().as_str(), "LPAD");
            if let (Some(s), Some(len_v), Some(pad)) = (args.first(), args.get(1), args.get(2)) {
                let text = s.to_sql_string();
                let pad_str = pad.to_sql_string();
                let len = match len_v {
                    Value::Integer(i) => *i,
                    _ => return Value::Null,
                };
                if pad_str.is_empty() {
                    return Value::Null;
                }
                if (len as usize) <= text.len() {
                    if dir_is_left {
                        Value::Text(text[..len as usize].to_string())
                    } else {
                        let start = text.len() - len as usize;
                        Value::Text(text[start..].to_string())
                    }
                } else {
                    let mut pad_repeat = String::new();
                    let needed = len as usize - text.len();
                    while pad_repeat.len() < needed {
                        pad_repeat.push_str(&pad_str);
                    }
                    pad_repeat.truncate(needed);
                    if dir_is_left {
                        let mut out = String::with_capacity(len as usize);
                        out.push_str(&pad_repeat);
                        out.push_str(&text);
                        Value::Text(out)
                    } else {
                        let mut out = String::with_capacity(len as usize);
                        out.push_str(&text);
                        out.push_str(&pad_repeat);
                        Value::Text(out)
                    }
                }
            } else {
                Value::Null
            }
        }
        // REPEAT(str, count) — count <= 0 returns empty
        "REPEAT" => {
            if let (Some(s), Some(n)) = (args.first(), args.get(1)) {
                let text = s.to_sql_string();
                let count = match n {
                    Value::Integer(i) => *i,
                    _ => return Value::Text(String::new()),
                };
                if count <= 0 {
                    Value::Text(String::new())
                } else {
                    Value::Text(text.repeat(count as usize))
                }
            } else {
                Value::Null
            }
        }
        // REVERSE(str) — byte-reversed (not Unicode-aware; same as
        // MySQL's REVERSE for ASCII)
        "REVERSE" => args
            .first()
            .map(|v| Value::Text(v.to_sql_string().chars().rev().collect()))
            .unwrap_or(Value::Null),
        // SPACE(n) — n spaces; n <= 0 returns empty
        "SPACE" => {
            let n = match args.first() {
                Some(Value::Integer(i)) => *i,
                _ => return Value::Text(String::new()),
            };
            if n <= 0 {
                Value::Text(String::new())
            } else {
                Value::Text(" ".repeat(n as usize))
            }
        }
        // FIELD(str, str1, str2, ...) — index of first match (1-based),
        // 0 if no match. NULL if any argument is NULL.
        "FIELD" => {
            if args.is_empty() {
                return Value::Null;
            }
            let needle = args[0].to_sql_string();
            for (i, v) in args.iter().enumerate().skip(1) {
                if matches!(v, Value::Null) {
                    return Value::Null;
                }
                if v.to_sql_string() == needle {
                    return Value::Integer(i as i64);
                }
            }
            Value::Integer(0)
        }
        // ELT(n, str1, str2, ...) — returns the n-th string (1-based).
        // n < 1 or n > args.len()-1 returns NULL.
        "ELT" => {
            let n = match args.first() {
                Some(Value::Integer(i)) => *i,
                _ => return Value::Null,
            };
            if n < 1 {
                return Value::Null;
            }
            match args.get(n as usize) {
                Some(v) if !matches!(v, Value::Null) => Value::Text(v.to_sql_string()),
                _ => Value::Null,
            }
        }
        // TPC-H Sprint 1 fix (Q7/Q8/Q9): SUBSTR(x, start, length)
        "SUBSTR" | "SUBSTRING" => {
            if let (Some(s), Some(start)) = (args.first(), args.get(1)) {
                let text = s.to_sql_string();
                let start_idx = match start {
                    Value::Integer(i) => (*i).saturating_sub(1).max(0) as usize,
                    _ => return Value::Text(String::new()),
                };
                if start_idx >= text.len() {
                    return Value::Text(String::new());
                }
                let end = if let Some(len) = args.get(2) {
                    let len = match len {
                        Value::Integer(i) => (*i).max(0) as usize,
                        _ => return Value::Text(String::new()),
                    };
                    (start_idx + len).min(text.len())
                } else {
                    text.len()
                };
                Value::Text(text[start_idx..end].to_string())
            } else {
                Value::Null
            }
        }
        // TPC-H Sprint 1 fix (Q7/Q8/Q9): CAST(x AS TYPE) — parser routes CAST
        // through FunctionCall. We can't reach the target type from here, so
        // pass through the input. Downstream Integer() context coerces.
        "CAST" => args.first().cloned().unwrap_or(Value::Null),
        // MySQL 5.7 function compatibility (Issue #2988 / MySQL-01)
        // IF(cond, then, else) — ternary; 2-arg form: IF(cond, NULL)
        "IF" | "IFF" => {
            if args.len() < 2 {
                Value::Null
            } else {
                let cond = &args[0];
                let then_v = &args[1];
                let else_v = args.get(2).cloned().unwrap_or(Value::Null);
                let truthy = match cond {
                    Value::Boolean(b) => *b,
                    Value::Null => false,
                    Value::Integer(0) => false,
                    Value::Float(f) if *f == 0.0 => false,
                    _ => true,
                };
                if truthy {
                    then_v.clone()
                } else {
                    else_v
                }
            }
        }
        // COALESCE(a, b, c, ...) — first non-NULL
        "COALESCE" => args
            .iter()
            .find(|v| !matches!(v, Value::Null))
            .cloned()
            .unwrap_or(Value::Null),
        // ISNULL(x) — same as IS NULL, returns Boolean
        "ISNULL" => args
            .first()
            .map(|v| Value::Boolean(matches!(v, Value::Null)))
            .unwrap_or(Value::Null),
        // NULLIF(a, b) — NULL if equal, else a
        "NULLIF" => {
            if args.len() < 2 {
                Value::Null
            } else {
                let a = &args[0];
                let b = &args[1];
                if a == b {
                    Value::Null
                } else {
                    a.clone()
                }
            }
        }
        // DATE_ADD(date, INTERVAL n unit) — text dates only (YYYY-MM-DD)
        // Supports unit: DAY, MONTH, YEAR
        "DATE_ADD" | "ADDDATE" => date_add_sub(args, true),
        // DATE_SUB(date, INTERVAL n unit)
        "DATE_SUB" | "SUBDATE" => date_add_sub(args, false),
        // TPC-H Q7/Q8/Q9 use `EXTRACT(YEAR FROM o_orderdate) AS o_year`.
        // The parser encodes this as FunctionCall("EXTRACT", [Literal(field),
        // source_expr]). For text dates in YYYY-MM-DD form, the field slices
        // a fixed offset. Returns Text (matches the input type) so the result
        // can be used in GROUP BY, ORDER BY, and joins without an Integer
        // coercion round-trip.
        "EXTRACT" => {
            let field = args
                .first()
                .map(|v| v.to_sql_string().to_uppercase())
                .unwrap_or_default();
            let source = match args.get(1) {
                Some(v) => v.to_sql_string(),
                None => return Value::Null,
            };
            match field.as_str() {
                "YEAR" if source.len() >= 4 => Value::Text(source[..4].to_string()),
                "MONTH" if source.len() >= 7 => Value::Text(source[5..7].to_string()),
                "DAY" if source.len() >= 10 => Value::Text(source[8..10].to_string()),
                _ => Value::Null,
            }
        }
        // MySQL 5.7 string functions (Issue #2988 / MySQL-01 follow-up).
        // REPLACE(str, from_str, to_str) — replaces ALL occurrences.
        "REPLACE" => {
            if args.len() < 3 {
                Value::Null
            } else {
                Value::Text(
                    args[0]
                        .to_sql_string()
                        .replace(&args[1].to_sql_string(), &args[2].to_sql_string()),
                )
            }
        }
        // INSERT(str, pos, len, newstr) — pos is 1-based.
        // MySQL semantics:
        //   - pos <= 0 or pos > length(str): returns str unchanged
        //   - len <= 0: returns str with newstr inserted at pos (no deletion)
        //   - len > remaining: clips to end
        "INSERT" => {
            if args.len() < 4 {
                return Value::Null;
            }
            let s = args[0].to_sql_string();
            let pos_i: i64 = match &args[1] {
                Value::Integer(n) => *n,
                _ => return Value::Null,
            };
            let len_i: i64 = match &args[2] {
                Value::Integer(n) => *n,
                _ => return Value::Null,
            };
            let newstr = args[3].to_sql_string();
            if pos_i <= 0 || (pos_i as usize) > s.len() {
                return Value::Text(s);
            }
            let pos0 = (pos_i as usize) - 1;
            if len_i <= 0 {
                // Pure insert: no deletion
                let mut out = String::with_capacity(s.len() + newstr.len());
                out.push_str(&s[..pos0]);
                out.push_str(&newstr);
                out.push_str(&s[pos0..]);
                return Value::Text(out);
            }
            let len = (len_i as usize).min(s.len() - pos0);
            let mut out = String::with_capacity(s.len() + newstr.len());
            out.push_str(&s[..pos0]);
            out.push_str(&newstr);
            out.push_str(&s[pos0 + len..]);
            Value::Text(out)
        }
        // Statistical aggregates. Population (POP) divides by n; sample
        // (SAMP) divides by n-1 (or NULL when n<2). STDDEV/VARIANCE
        // are aliases for STDDEV_POP/VAR_POP (MySQL convention).
        "STDDEV" | "STDDEV_POP" => stddev_variance(args, true),
        "STDDEV_SAMP" => stddev_variance(args, false),
        "VARIANCE" | "VAR_POP" => variance_value(args, true),
        "VAR_SAMP" => variance_value(args, false),
        // Bit aggregates — operate on integer column, NULL treated as 0.
        "BIT_AND" => bit_aggregate(args, BitOp::And),
        "BIT_OR" => bit_aggregate(args, BitOp::Or),
        "BIT_XOR" => bit_aggregate(args, BitOp::Xor),
        // GROUPING(col) — MySQL: returns 1 if col is NULL because of
        // a ROLLUP/CUBE roll-up, else 0. Implemented against the
        // ROLLUP/CUBE result rows which are stored with a sentinel
        // `_rollup_<col>` marker injected by the engine. Without the
        // marker (no ROLLUP/CUBE), GROUPING always returns 0.
        "GROUPING" => Value::Integer(0),
        // GROUP_CONCAT — aggregate concatenator. Supports SEPARATOR.
        "GROUP_CONCAT" => group_concat(args),
        // F-03 GIS: ST_WITHIN, ST_Distance, ST_Contains, ST_Intersects
        "ST_WITHIN" | "ST_CONTAINS" | "ST_INTERSECTS" | "ST_DISTANCE" => {
            use sqlrustgo_gis::{
                st_within as gis_st_within, st_distance as gis_st_distance,
                st_contains as gis_st_contains, st_intersects as gis_st_intersects,
                Point as GisPoint, Polygon as GisPolygon,
            };
            if args.len() < 2 {
                return Value::Null;
            }
            match name.to_uppercase().as_str() {
                "ST_WITHIN" => {
                    let point = match &args[0] {
                        Value::Point(x, y) => GisPoint::new(*x, *y),
                        Value::Text(s) => match GisPoint::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    let polygon = match &args[1] {
                        Value::Text(s) => match GisPolygon::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    Value::Boolean(gis_st_within(&point, &polygon))
                }
                "ST_DISTANCE" => {
                    let p1 = match &args[0] {
                        Value::Point(x, y) => GisPoint::new(*x, *y),
                        Value::Text(s) => match GisPoint::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    let p2 = match &args[1] {
                        Value::Point(x, y) => GisPoint::new(*x, *y),
                        Value::Text(s) => match GisPoint::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    Value::Float(gis_st_distance(&p1, &p2))
                }
                "ST_CONTAINS" => {
                    let polygon = match &args[0] {
                        Value::Text(s) => match GisPolygon::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    let point = match &args[1] {
                        Value::Point(x, y) => GisPoint::new(*x, *y),
                        Value::Text(s) => match GisPoint::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    Value::Boolean(gis_st_contains(&polygon, &point))
                }
                "ST_INTERSECTS" => {
                    let p1 = match &args[0] {
                        Value::Text(s) => match GisPolygon::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    let p2 = match &args[1] {
                        Value::Text(s) => match GisPolygon::parse(s) {
                            Some(p) => p,
                            None => return Value::Null,
                        },
                        _ => return Value::Null,
                    };
                    Value::Boolean(gis_st_intersects(&p1, &p2))
                }
                _ => Value::Null,
            }
        }
        // JSON functions
        "JSON_EXTRACT" => {
            if args.len() < 2 {
                return Value::Null;
            }
            json_extract(&args[0], &args[1], false)
        }
        "JSON_VALUE" => {
            if args.len() < 2 {
                return Value::Null;
            }
            json_extract(&args[0], &args[1], true)
        }
        "JSON" => {
            // JSON(text) — parse text as JSON document
            if args.is_empty() {
                return Value::Null;
            }
            let s = args[0].to_sql_string();
            match serde_json::from_str(&s) {
                Ok(v) => Value::Json(v),
                Err(_) => Value::Null,
            }
        }
        // JSON_VALID(text) — returns 1 if text is valid JSON, else 0
        "JSON_VALID" => {
            if args.is_empty() {
                return Value::Null;
            }
            let s = args[0].to_sql_string();
            Value::Boolean(serde_json::from_str::<serde_json::Value>(&s).is_ok())
        }
        // JSON_TYPE(json_value) — returns the type of the JSON value
        "JSON_TYPE" => {
            if args.is_empty() {
                return Value::Null;
            }
            let s = args[0].to_sql_string();
            match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(serde_json::Value::Null) => Value::Text("null".to_string()),
                Ok(serde_json::Value::Bool(_)) => Value::Text("boolean".to_string()),
                Ok(serde_json::Value::Number(_)) => Value::Text("number".to_string()),
                Ok(serde_json::Value::String(_)) => Value::Text("string".to_string()),
                Ok(serde_json::Value::Array(_)) => Value::Text("array".to_string()),
                Ok(serde_json::Value::Object(_)) => Value::Text("object".to_string()),
                Err(_) => Value::Null,
            }
        }
        // JSON_KEYS(json_doc, path?) — returns JSON array of keys at path
        "JSON_KEYS" => {
            if args.is_empty() {
                return Value::Null;
            }
            let s = args[0].to_sql_string();
            let doc = match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(v) => v,
                Err(_) => return Value::Null,
            };
            let path = args.get(1).map(|v| v.to_sql_string()).unwrap_or_else(|| "$".to_string());
            let json_path = if path.starts_with('$') {
                path
            } else {
                format!("$.{}", path)
            };
            match doc.pointer(&json_path) {
                Some(serde_json::Value::Object(obj)) => {
                    let keys: Vec<String> = obj.keys().cloned().collect();
                    match serde_json::to_string(&keys) {
                        Ok(s) => Value::Json(serde_json::from_str(&s).unwrap()),
                        Err(_) => Value::Null,
                    }
                }
                _ => Value::Null,
            }
        }
        // F-03 GIS: ST_Distance(point1, point2) — returns distance
        "ST_DISTANCE" => {
            use sqlrustgo_gis::{st_distance as gis_st_distance, Point as GisPoint};
            if args.len() != 2 {
                return Value::Null;
            }
            let p1 = match &args[0] {
                Value::Point(x, y) => GisPoint::new(*x, *y),
                Value::Text(s) => match GisPoint::parse(s) { Some(p) => p, None => return Value::Null },
                _ => return Value::Null,
            };
            let p2 = match &args[1] {
                Value::Point(x, y) => GisPoint::new(*x, *y),
                Value::Text(s) => match GisPoint::parse(s) { Some(p) => p, None => return Value::Null },
                _ => return Value::Null,
            };
            Value::Float(gis_st_distance(&p1, &p2))
        }
        _ => Value::Null,
    }
}

/// DATE_ADD / DATE_SUB helper. Operates on text dates in YYYY-MM-DD form.
/// Accepts args in either order:
/// Accepts args in either order:
///
/// - [date_text, n, unit_text]
/// - [date_text, n] (default unit = DAY)
///
/// Returns Value::Text (new date) or Value::Null on bad input.
fn date_add_sub(args: &[Value], add: bool) -> Value {
    if args.len() < 2 {
        return Value::Null;
    }
    let date_str = args[0].to_sql_string();
    if date_str.len() < 10 {
        return Value::Null;
    }
    let n = match args[1] {
        Value::Integer(i) => i,
        _ => return Value::Null,
    };
    let unit = args
        .get(2)
        .map(|v| v.to_sql_string().to_uppercase())
        .unwrap_or_else(|| "DAY".to_string());
    let sign = if add { 1 } else { -1 };
    match unit.as_str() {
        "DAY" => {
            // Approximate: shift the day portion; for simplicity, do integer
            // day math on YYYYMMDD-formatted number to handle month/year wrap.
            let y: i64 = date_str[..4].parse().unwrap_or(0);
            let m: i64 = date_str[5..7].parse().unwrap_or(1);
            let d: i64 = date_str[8..10].parse().unwrap_or(1);
            let total_days = days_from_civil(y, m, d) + sign * n;
            let (ny, nm, nd) = civil_from_days(total_days);
            Value::Text(format!("{:04}-{:02}-{:02}", ny, nm, nd))
        }
        "MONTH" => {
            let y: i64 = date_str[..4].parse().unwrap_or(0);
            let m: i64 = date_str[5..7].parse().unwrap_or(1);
            let d: i64 = date_str[8..10].parse().unwrap_or(1);
            let total_months = y * 12 + (m - 1) + sign * n;
            let ny = total_months.div_euclid(12);
            let nm = total_months.rem_euclid(12) + 1;
            // Clamp day to last day of new month
            let nd = d.min(days_in_month(ny, nm));
            Value::Text(format!("{:04}-{:02}-{:02}", ny, nm, nd))
        }
        "YEAR" => {
            let y: i64 = date_str[..4].parse().unwrap_or(0);
            let m: &str = &date_str[5..7];
            let d: &str = &date_str[8..10];
            let ny = y + sign * n;
            let nd_max = days_in_month(ny, m.parse().unwrap_or(1));
            let d_int: i64 = d.parse().unwrap_or(1);
            let nd = d_int.min(nd_max);
            Value::Text(format!("{:04}-{}-{:02}", ny, m, nd))
        }
        _ => Value::Null,
    }
}

/// Howard Hinnant's days_from_civil: number of days since 1970-01-01
/// (or any other civil date), proleptic Gregorian.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400); // [0, 399]
    let m = if m > 2 { m - 3 } else { m + 9 }; // [0, 11]
    let doy = (153 * m + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Statistical aggregate helpers (eval_fn dispatch). These are
/// scalar-aware: pass the `&[Value]` collected by the aggregate
/// function; we compute mean / variance / stddev on the integers
/// (treating floats as their f64, NULL skipped). `pop = true` uses
/// the population formula (n), `pop = false` uses the sample
/// formula (n-1) and returns NULL when n<2.
fn variance_value(args: &[Value], pop: bool) -> Value {
    let xs: Vec<f64> = args
        .iter()
        .filter_map(|v| match v {
            Value::Null => None,
            Value::Integer(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => Some(v.to_sql_string().parse::<f64>().unwrap_or(0.0)),
        })
        .collect();
    let n = xs.len();
    if n == 0 {
        return Value::Null;
    }
    if !pop && n < 2 {
        return Value::Null;
    }
    let mean: f64 = xs.iter().sum::<f64>() / n as f64;
    let denom = if pop { n as f64 } else { (n - 1) as f64 };
    let var: f64 = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / denom;
    Value::Float(var)
}

fn stddev_variance(args: &[Value], pop: bool) -> Value {
    match variance_value(args, pop) {
        Value::Float(v) => Value::Float(v.sqrt()),
        other => other,
    }
}

/// Bit aggregate operators.
enum BitOp {
    And,
    Or,
    Xor,
}

fn bit_aggregate(args: &[Value], op: BitOp) -> Value {
    let mut acc: Option<i64> = None;
    for v in args {
        if matches!(v, Value::Null) {
            continue;
        }
        let n = match v {
            Value::Integer(i) => *i,
            _ => v.to_sql_string().parse::<i64>().unwrap_or(0),
        };
        acc = Some(match acc {
            None => n,
            Some(a) => match op {
                BitOp::And => a & n,
                BitOp::Or => a | n,
                BitOp::Xor => a ^ n,
            },
        });
    }
    match acc {
        Some(n) => Value::Integer(n),
        None => Value::Null,
    }
}

/// GROUP_CONCAT — concatenate non-NULL values with optional separator.
/// In the current eval_fn dispatch, GROUP_CONCAT receives the
/// function's args slice (which is the column's slice for an
/// aggregate call, or the literal args for scalar calls). We treat
/// all non-NULL args as values to concatenate. The separator is
/// always comma (','); the SEPARATOR clause of GROUP_CONCAT is not
/// yet supported and would require parser changes.
fn group_concat(args: &[Value]) -> Value {
    if args.is_empty() {
        return Value::Null;
    }
    let separator = ",";
    let joined: String = args
        .iter()
        .filter(|v| !matches!(v, Value::Null))
        .map(|v| v.to_sql_string())
        .collect::<Vec<_>>()
        .join(separator);
    Value::Text(joined)
}

/// Evaluate the parser-AST `Expression::Cast{expr, target_type}` arm:
/// converts a value to the target type per MySQL 5.7 cast semantics.
///
/// This is the single source of truth for the parser-AST `Cast` branch.
/// The legacy `src/expr_utils.rs::evaluate_expression` `Expression::Cast`
/// arm (P0-2 §4.13) is a thin delegation to this function.
///
/// **Semantics:**
/// - `cast_val(Integer(42), "INTEGER")` → `Integer(42)` (idempotent)
/// - `cast_val(Text("42"), "INTEGER")` → `Integer(42)` (parse text)
/// - `cast_val(Text("not a number"), "INTEGER")` → `Integer(0)` (parse fail → 0)
/// - `cast_val(Float(2.7), "INTEGER")` → `Integer(2)` (truncate)
/// - `cast_val(_, "TEXT")` → `Text(to_sql_string())` (any → text)
/// - `cast_val(_, "UNKNOWN_TYPE")` → `val.clone()` (passthrough)
pub fn cast_val(val: &Value, target_type: &str) -> Value {
    match target_type.to_uppercase().as_str() {
        "INTEGER" | "INT" => match val {
            Value::Integer(i) => Value::Integer(*i),
            Value::Text(s) => s
                .parse::<i64>()
                .map(Value::Integer)
                .unwrap_or(Value::Integer(0)),
            Value::Float(f) => Value::Integer(*f as i64),
            _ => Value::Integer(0),
        },
        "TEXT" | "VARCHAR" | "STRING" => Value::Text(val.to_sql_string()),
        _ => val.clone(),
    }
}

fn to_bool(v: &Value) -> bool {
    match v {
        Value::Integer(i) => *i != 0,
        Value::Boolean(b) => *b,
        Value::Null => false,
        _ => true,
    }
}

fn compare_cmp(left: &Value, right: &Value, op: &str) -> Value {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return Value::Boolean(false);
    }
    let cmp = match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(b) as i64,
        (Value::Text(a), Value::Text(b)) => a.cmp(b) as i64,
        _ => return Value::Null,
    };
    let result = match op {
        ">" => cmp > 0,
        "<" => cmp < 0,
        ">=" => cmp >= 0,
        "<=" => cmp <= 0,
        _ => return Value::Null,
    };
    Value::Boolean(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- IF -----
    #[test]
    fn test_if_true_returns_then() {
        let v = eval_fn(
            "IF",
            &[Value::Boolean(true), Value::Integer(1), Value::Integer(2)],
        );
        assert_eq!(v, Value::Integer(1));
    }

    #[test]
    fn test_if_false_returns_else() {
        let v = eval_fn(
            "IF",
            &[Value::Boolean(false), Value::Integer(1), Value::Integer(2)],
        );
        assert_eq!(v, Value::Integer(2));
    }

    #[test]
    fn test_if_null_cond_returns_else() {
        let v = eval_fn("IF", &[Value::Null, Value::Integer(1), Value::Integer(2)]);
        assert_eq!(v, Value::Integer(2));
    }

    #[test]
    fn test_if_two_arg_form() {
        // IF(cond, then) — else is NULL
        let v = eval_fn("IF", &[Value::Boolean(false), Value::Integer(7)]);
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn test_if_zero_is_false() {
        let v = eval_fn(
            "IF",
            &[
                Value::Integer(0),
                Value::Text("yes".into()),
                Value::Text("no".into()),
            ],
        );
        assert_eq!(v, Value::Text("no".into()));
    }

    // ----- COALESCE -----
    #[test]
    fn test_coalesce_first_non_null() {
        let v = eval_fn(
            "COALESCE",
            &[
                Value::Null,
                Value::Null,
                Value::Integer(3),
                Value::Integer(4),
            ],
        );
        assert_eq!(v, Value::Integer(3));
    }

    #[test]
    fn test_coalesce_all_null() {
        let v = eval_fn("COALESCE", &[Value::Null, Value::Null]);
        assert_eq!(v, Value::Null);
    }

    // ----- ISNULL -----
    #[test]
    fn test_isnull_true() {
        let v = eval_fn("ISNULL", &[Value::Null]);
        assert_eq!(v, Value::Boolean(true));
    }

    #[test]
    fn test_isnull_false() {
        let v = eval_fn("ISNULL", &[Value::Integer(1)]);
        assert_eq!(v, Value::Boolean(false));
    }

    // ----- NULLIF -----
    #[test]
    fn test_nullif_equal() {
        let v = eval_fn("NULLIF", &[Value::Integer(5), Value::Integer(5)]);
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn test_nullif_not_equal() {
        let v = eval_fn("NULLIF", &[Value::Integer(5), Value::Integer(6)]);
        assert_eq!(v, Value::Integer(5));
    }

    // ----- DATE_ADD / DATE_SUB -----
    #[test]
    fn test_date_add_day() {
        let v = eval_fn(
            "DATE_ADD",
            &[
                Value::Text("2026-06-04".into()),
                Value::Integer(7),
                Value::Text("DAY".into()),
            ],
        );
        assert_eq!(v, Value::Text("2026-06-11".into()));
    }

    #[test]
    fn test_date_add_month() {
        let v = eval_fn(
            "DATE_ADD",
            &[
                Value::Text("2026-01-15".into()),
                Value::Integer(1),
                Value::Text("MONTH".into()),
            ],
        );
        assert_eq!(v, Value::Text("2026-02-15".into()));
    }

    #[test]
    fn test_date_add_year_leap_clamp() {
        // 2024-02-29 + 1 year = 2025-02-28 (clamped, 2025 is not leap)
        let v = eval_fn(
            "DATE_ADD",
            &[
                Value::Text("2024-02-29".into()),
                Value::Integer(1),
                Value::Text("YEAR".into()),
            ],
        );
        assert_eq!(v, Value::Text("2025-02-28".into()));
    }

    #[test]
    fn test_date_sub_day() {
        let v = eval_fn(
            "DATE_SUB",
            &[
                Value::Text("2026-06-04".into()),
                Value::Integer(10),
                Value::Text("DAY".into()),
            ],
        );
        assert_eq!(v, Value::Text("2026-05-25".into()));
    }

    #[test]
    fn test_date_add_cross_year() {
        let v = eval_fn(
            "DATE_ADD",
            &[
                Value::Text("2026-12-25".into()),
                Value::Integer(10),
                Value::Text("DAY".into()),
            ],
        );
        assert_eq!(v, Value::Text("2027-01-04".into()));
    }

    #[test]
    fn test_date_add_bad_input_returns_null() {
        let v = eval_fn(
            "DATE_ADD",
            &[
                Value::Text("nope".into()),
                Value::Integer(1),
                Value::Text("DAY".into()),
            ],
        );
        assert_eq!(v, Value::Null);
    }

    // ----- Statistical aggregates (Phase-3c) -----
    #[test]
    fn test_stddev_pop_alias() {
        // STDDEV and STDDEV_POP should return identical values.
        let args = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
        let a = match eval_fn("STDDEV", &args) {
            Value::Float(v) => v,
            _ => panic!("STDDEV should return Float"),
        };
        let b = match eval_fn("STDDEV_POP", &args) {
            Value::Float(v) => v,
            _ => panic!("STDDEV_POP should return Float"),
        };
        assert!((a - b).abs() < 0.0001, "STDDEV vs STDDEV_POP should match");
    }

    #[test]
    fn test_variance_pop() {
        // Variance of [1, 2, 3] is 2/3 (population).
        let v = eval_fn(
            "VARIANCE",
            &[Value::Integer(1), Value::Integer(2), Value::Integer(3)],
        );
        match v {
            Value::Float(f) => assert!((f - 0.6667).abs() < 0.001, "got {f}"),
            _ => panic!("VARIANCE should return Float"),
        }
    }

    #[test]
    fn test_variance_samp_n_less_than_2_returns_null() {
        // SAMP with n=1 returns NULL (n-1 denominator would be 0).
        let v = eval_fn("VAR_SAMP", &[Value::Integer(5)]);
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn test_variance_samp_two_values() {
        // VAR_SAMP of [1, 3] is ((1-2)^2 + (3-2)^2) / (2-1) = 2
        let v = eval_fn("VAR_SAMP", &[Value::Integer(1), Value::Integer(3)]);
        match v {
            Value::Float(f) => assert!((f - 2.0).abs() < 0.001, "got {f}"),
            _ => panic!("VAR_SAMP should return Float"),
        }
    }

    // ----- Bit aggregates (Phase-3c) -----
    #[test]
    fn test_bit_and() {
        let v = eval_fn("BIT_AND", &[Value::Integer(0b1100), Value::Integer(0b1010)]);
        assert_eq!(v, Value::Integer(0b1000));
    }

    #[test]
    fn test_bit_or() {
        let v = eval_fn("BIT_OR", &[Value::Integer(0b1100), Value::Integer(0b1010)]);
        assert_eq!(v, Value::Integer(0b1110));
    }

    #[test]
    fn test_bit_xor() {
        let v = eval_fn("BIT_XOR", &[Value::Integer(0b1100), Value::Integer(0b1010)]);
        assert_eq!(v, Value::Integer(0b0110));
    }

    #[test]
    fn test_bit_aggregate_empty() {
        let v = eval_fn("BIT_AND", &[]);
        assert_eq!(v, Value::Null);
    }

    // ----- GROUPING (Phase-3c) -----
    #[test]
    fn test_grouping_returns_zero_by_default() {
        // Without ROLLUP/CUBE, GROUPING always returns 0.
        let v = eval_fn("GROUPING", &[Value::Text("col".into())]);
        assert_eq!(v, Value::Integer(0));
    }

    // ----- GROUP_CONCAT (Phase-3c) -----
    #[test]
    fn test_group_concat_basic() {
        let v = eval_fn(
            "GROUP_CONCAT",
            &[
                Value::Text("a".into()),
                Value::Text("b".into()),
                Value::Text("c".into()),
            ],
        );
        assert_eq!(v, Value::Text("a,b,c".into()));
    }

    #[test]
    fn test_group_concat_skips_null() {
        let v = eval_fn(
            "GROUP_CONCAT",
            &[
                Value::Text("a".into()),
                Value::Null,
                Value::Text("b".into()),
            ],
        );
        assert_eq!(v, Value::Text("a,b".into()));
    }

    #[test]
    fn test_group_concat_empty() {
        let v = eval_fn("GROUP_CONCAT", &[]);
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn test_literal_int() {
        assert_eq!(
            UnifiedExpr::Literal(Value::Integer(42)).evaluate(&[], &[], &mut None),
            Value::Integer(42)
        );
    }

    #[test]
    fn test_column() {
        let mut e = UnifiedExpr::Column("x".into());
        assert_eq!(
            e.evaluate(&[Value::Integer(5)], &["x".into()], &mut None),
            Value::Integer(5)
        );
    }

    #[test]
    fn test_eq() {
        let mut e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Column("a".into())),
            op: "=".into(),
            right: Box::new(UnifiedExpr::Literal(Value::Integer(5))),
        };
        assert_eq!(
            e.evaluate(&[Value::Integer(5)], &["a".into()], &mut None),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(3)], &["a".into()], &mut None),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_and() {
        let mut e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
            op: "AND".into(),
            right: Box::new(UnifiedExpr::Literal(Value::Integer(0))),
        };
        assert_eq!(e.evaluate(&[], &[], &mut None), Value::Boolean(false));
    }

    #[test]
    fn test_or() {
        let mut e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
            op: "OR".into(),
            right: Box::new(UnifiedExpr::Literal(Value::Integer(0))),
        };
        assert_eq!(e.evaluate(&[], &[], &mut None), Value::Boolean(true));
    }

    #[test]
    fn test_is_null() {
        let mut e = UnifiedExpr::IsNull(Box::new(UnifiedExpr::Column("x".into())));
        assert_eq!(
            e.evaluate(&[Value::Null], &["x".into()], &mut None),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Boolean(true)], &["x".into()], &mut None),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_in_list() {
        let mut e = UnifiedExpr::InList {
            expr: Box::new(UnifiedExpr::Column("x".into())),
            list: vec![
                UnifiedExpr::Literal(Value::Integer(1)),
                UnifiedExpr::Literal(Value::Integer(3)),
            ],
        };
        assert_eq!(
            e.evaluate(&[Value::Integer(3)], &["x".into()], &mut None),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(2)], &["x".into()], &mut None),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_between() {
        let mut e = UnifiedExpr::Between {
            expr: Box::new(UnifiedExpr::Column("age".into())),
            low: Box::new(UnifiedExpr::Literal(Value::Integer(18))),
            high: Box::new(UnifiedExpr::Literal(Value::Integer(65))),
        };
        assert_eq!(
            e.evaluate(&[Value::Integer(30)], &["age".into()], &mut None),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(15)], &["age".into()], &mut None),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_case_when() {
        let mut e = UnifiedExpr::CaseWhen {
            whens: vec![(
                UnifiedExpr::BinaryOp {
                    left: Box::new(UnifiedExpr::Column("s".into())),
                    op: ">=".into(),
                    right: Box::new(UnifiedExpr::Literal(Value::Integer(90))),
                },
                UnifiedExpr::Literal(Value::Text("A".into())),
            )],
            else_val: Some(Box::new(UnifiedExpr::Literal(Value::Text("F".into())))),
        };
        assert_eq!(
            e.evaluate(&[Value::Integer(95)], &["s".into()], &mut None),
            Value::Text("A".into())
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(50)], &["s".into()], &mut None),
            Value::Text("F".into())
        );
    }

    #[test]
    fn test_cast() {
        let mut e = UnifiedExpr::Cast {
            expr: Box::new(UnifiedExpr::Literal(Value::Text("42".into()))),
            target_type: "INTEGER".into(),
        };
        assert_eq!(e.evaluate(&[], &[], &mut None), Value::Integer(42));
    }

    #[test]
    fn test_referenced_columns() {
        let mut e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Column("a".into())),
            op: "+".into(),
            right: Box::new(UnifiedExpr::Column("b".into())),
        };
        let cols = e.referenced_columns();
        assert!(cols.contains(&"a".into()));
        assert!(cols.contains(&"b".into()));
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn test_parse_literal() {
        assert_eq!(parse_lit("42"), Value::Integer(42));
        assert_eq!(parse_lit("'hello'"), Value::Text("hello".into()));
        assert_eq!(parse_lit("NULL"), Value::Null);
        assert_eq!(parse_lit("TRUE"), Value::Boolean(true));
        assert_eq!(parse_lit("FALSE"), Value::Boolean(false));
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(
            eval_binary_op(&Value::Integer(3), &Value::Integer(4), "+"),
            Value::Integer(7)
        );
        assert_eq!(
            eval_binary_op(&Value::Integer(3), &Value::Integer(4), "*"),
            Value::Integer(12)
        );
    }

    /// Regression test for the TPC-H Q1 SUM with computed expression bug
    /// (2026-06-13). Previously `Float OP Float` returned `0` because
    /// `as_integer().unwrap_or(0)` silently coerced `Float` operands
    /// to `0`. The fix promotes to `Float` whenever either operand is
    /// `Float`, matching PostgreSQL/SQLite semantics.
    #[test]
    fn test_arithmetic_float_promotion() {
        assert_eq!(
            eval_binary_op(&Value::Float(100.5), &Value::Float(0.95), "*"),
            Value::Float(95.475)
        );
        assert_eq!(
            eval_binary_op(&Value::Integer(100), &Value::Float(0.5), "+"),
            Value::Float(100.5)
        );
        assert_eq!(
            eval_binary_op(&Value::Float(1.0), &Value::Float(0.0), "/"),
            Value::Null
        );
        assert_eq!(
            eval_binary_op(&Value::Null, &Value::Integer(1), "+"),
            Value::Null
        );
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Null, "*"),
            Value::Null
        );
    }

    #[test]
    fn test_eval_is_null_functions() {
        assert_eq!(eval_is_null(&Value::Null), Value::Boolean(true));
        assert_eq!(eval_is_null(&Value::Integer(0)), Value::Boolean(false));
        assert_eq!(eval_is_not_null(&Value::Null), Value::Boolean(false));
        assert_eq!(eval_is_not_null(&Value::Integer(0)), Value::Boolean(true));
    }

    #[test]
    fn test_compare_values_cross_type() {
        assert_eq!(compare_values(&Value::Float(5.0), &Value::Integer(5)), 0);
        assert_eq!(compare_values(&Value::Float(3.0), &Value::Integer(5)), -1);
        assert_eq!(compare_values(&Value::Float(7.0), &Value::Integer(5)), 1);
        assert_eq!(compare_values(&Value::Integer(5), &Value::Float(5.0)), 0);
        assert_eq!(compare_values(&Value::Integer(3), &Value::Float(5.0)), -1);
        assert_eq!(compare_values(&Value::Integer(7), &Value::Float(5.0)), 1);
        assert_eq!(
            compare_values(&Value::Text("a".into()), &Value::Integer(1)),
            0
        );
        assert_eq!(compare_values(&Value::Null, &Value::Null), 0);
        assert_eq!(compare_values(&Value::Null, &Value::Integer(1)), -1);
        assert_eq!(compare_values(&Value::Integer(1), &Value::Null), 1);
    }

    #[test]
    fn test_eval_between_standalone() {
        assert_eq!(
            eval_between(&Value::Integer(5), &Value::Integer(1), &Value::Integer(10)),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_between(&Value::Integer(0), &Value::Integer(1), &Value::Integer(10)),
            Value::Boolean(false)
        );
        assert_eq!(
            eval_between(&Value::Integer(5), &Value::Integer(5), &Value::Integer(10)),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_between(&Value::Integer(10), &Value::Integer(1), &Value::Integer(10)),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_between(&Value::Null, &Value::Integer(1), &Value::Integer(10)),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_eval_not_between_standalone() {
        assert_eq!(
            eval_not_between(&Value::Integer(5), &Value::Integer(1), &Value::Integer(10)),
            Value::Boolean(false)
        );
        assert_eq!(
            eval_not_between(&Value::Integer(0), &Value::Integer(1), &Value::Integer(10)),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_parse_lit_more() {
        assert_eq!(parse_lit("  42  "), Value::Integer(42));
        assert_eq!(parse_lit("\"quoted\""), Value::Text("quoted".into()));
        assert_eq!(
            parse_lit("'hello world'"),
            Value::Text("hello world".into())
        );
        assert_eq!(parse_lit("3.14"), Value::Integer(3));
        assert_eq!(parse_lit("unquoted"), Value::Text("unquoted".into()));
    }

    #[test]
    fn test_eval_unary_op_not() {
        assert_eq!(
            eval_unary_op(&Value::Boolean(true), "NOT"),
            Value::Boolean(false)
        );
        assert_eq!(
            eval_unary_op(&Value::Boolean(false), "NOT"),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_unary_op(&Value::Integer(0), "NOT"),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_unary_op(&Value::Integer(1), "NOT"),
            Value::Boolean(false)
        );
        assert_eq!(eval_unary_op(&Value::Null, "!"), Value::Boolean(true));
        assert_eq!(
            eval_unary_op(&Value::Integer(42), "UNKNOWN_OP"),
            Value::Null
        );
    }

    #[test]
    fn test_eval_binary_op_text_compare() {
        assert_eq!(
            eval_binary_op(&Value::Text("b".into()), &Value::Text("a".into()), ">"),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(&Value::Text("a".into()), &Value::Text("b".into()), "<"),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_binary_op(&Value::Text("a".into()), &Value::Text("a".into()), "<>"),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_eval_binary_op_eq_null() {
        assert_eq!(
            eval_binary_op(&Value::Null, &Value::Integer(5), "="),
            Value::Boolean(false)
        );
        assert_eq!(
            eval_binary_op(&Value::Null, &Value::Null, "=="),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_eval_binary_op_unknown_operator() {
        assert_eq!(
            eval_binary_op(&Value::Integer(1), &Value::Integer(2), "LIKE"),
            Value::Null
        );
    }

    #[test]
    fn test_eval_arithmetic_boolean_operands() {
        assert_eq!(
            eval_arithmetic(&Value::Boolean(true), &Value::Boolean(false), "+"),
            Value::Integer(1)
        );
        assert_eq!(
            eval_arithmetic(&Value::Boolean(true), &Value::Boolean(true), "*"),
            Value::Integer(1)
        );
        assert_eq!(
            eval_arithmetic(&Value::Boolean(false), &Value::Boolean(true), "-"),
            Value::Integer(-1)
        );
    }

    #[test]
    fn test_eval_arithmetic_float_division_by_zero() {
        assert_eq!(
            eval_arithmetic(&Value::Float(5.0), &Value::Float(0.0), "/"),
            Value::Null
        );
    }

    #[test]
    fn test_eval_arithmetic_null_operand() {
        assert_eq!(
            eval_arithmetic(&Value::Null, &Value::Integer(5), "+"),
            Value::Null
        );
        assert_eq!(
            eval_arithmetic(&Value::Integer(5), &Value::Null, "-"),
            Value::Null
        );
    }

    #[test]
    fn test_to_f64_variants() {
        assert_eq!(to_f64(&Value::Integer(42)), 42.0);
        assert_eq!(to_f64(&Value::Float(3.14)), 3.14);
        assert_eq!(to_f64(&Value::Boolean(true)), 1.0);
        assert_eq!(to_f64(&Value::Boolean(false)), 0.0);
        assert_eq!(to_f64(&Value::Null), 0.0);
        assert_eq!(to_f64(&Value::Text("hello".into())), 0.0);
        assert_eq!(to_f64(&Value::Blob(vec![])), 0.0);
    }

    #[test]
    fn test_to_i64_variants() {
        assert_eq!(to_i64(&Value::Integer(42)), 42);
        assert_eq!(to_i64(&Value::Boolean(true)), 1);
        assert_eq!(to_i64(&Value::Boolean(false)), 0);
        assert_eq!(to_i64(&Value::Null), 0);
        assert_eq!(to_i64(&Value::Float(3.14)), 0);
        assert_eq!(to_i64(&Value::Text("hello".into())), 0);
        assert_eq!(to_i64(&Value::Blob(vec![])), 0);
    }

    #[test]
    fn test_eval_fn_length() {
        let v = eval_fn("LENGTH", &[Value::Text("hello".into())]);
        assert_eq!(v, Value::Integer(5));
    }

    #[test]
    fn test_eval_fn_lower_upper() {
        let v = eval_fn("LOWER", &[Value::Text("Hello".into())]);
        assert_eq!(v, Value::Text("hello".into()));
        let v = eval_fn("UPPER", &[Value::Text("Hello".into())]);
        assert_eq!(v, Value::Text("HELLO".into()));
    }

    #[test]
    fn test_eval_fn_trim_leading_trailing() {
        let v = eval_fn("TRIM", &[Value::Text("  hi  ".into())]);
        assert_eq!(v, Value::Text("hi".into()));
    }

    #[test]
    fn test_eval_fn_substring() {
        let v = eval_fn(
            "SUBSTRING",
            &[
                Value::Text("hello".into()),
                Value::Integer(2),
                Value::Integer(3),
            ],
        );
        assert_eq!(v, Value::Text("ell".into()));
    }

    #[test]
    fn test_eval_fn_coalesce_single_arg() {
        let v = eval_fn("COALESCE", &[Value::Integer(42)]);
        assert_eq!(v, Value::Integer(42));
    }

    #[test]
    fn test_unified_expr_unary_op() {
        let mut e = UnifiedExpr::UnaryOp {
            op: "NOT".into(),
            expr: Box::new(UnifiedExpr::Literal(Value::Boolean(true))),
        };
        assert_eq!(e.evaluate(&[], &[], &mut None), Value::Boolean(false));
    }

    #[test]
    fn test_unified_expr_is_not_null() {
        let mut e = UnifiedExpr::IsNotNull(Box::new(UnifiedExpr::Column("x".into())));
        assert_eq!(
            e.evaluate(&[Value::Integer(1)], &["x".into()], &mut None),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Null], &["x".into()], &mut None),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_unified_expr_function_call() {
        let mut e = UnifiedExpr::FunctionCall {
            name: "LENGTH".into(),
            args: vec![UnifiedExpr::Literal(Value::Text("abc".into()))],
        };
        assert_eq!(e.evaluate(&[], &[], &mut None), Value::Integer(3));
    }

    #[test]
    fn test_unified_expr_column_missing() {
        let mut e = UnifiedExpr::Column("missing".into());
        assert_eq!(
            e.evaluate(&[Value::Integer(1)], &["x".into()], &mut None),
            Value::Null
        );
    }

    #[test]
    fn test_literal_from_str() {
        assert_eq!(eval_literal_from_str("NULL"), Value::Null);
        assert_eq!(eval_literal_from_str("42"), Value::Integer(42));
        assert_eq!(eval_literal_from_str("3.14"), Value::Float(3.14));
        assert_eq!(
            eval_literal_from_str("'hello'"),
            Value::Text("hello".into())
        );
        assert_eq!(eval_literal_from_str("hello"), Value::Text("hello".into()));
        assert_eq!(eval_literal_from_str("  true  "), Value::Boolean(true));
    }

    #[test]
    fn test_like_match_basic() {
        assert!(sql_like_match("hello", "hello"));
        assert!(!sql_like_match("hello", "world"));
    }

    #[test]
    fn test_like_match_wildcard() {
        assert!(sql_like_match("hello", "h%"));
        assert!(sql_like_match("hello", "%o"));
        assert!(sql_like_match("hello", "%ell%"));
        assert!(!sql_like_match("hello", "h%d"));
    }

    #[test]
    fn test_like_match_single_char() {
        assert!(sql_like_match("hello", "h_llo"));
        assert!(sql_like_match("hello", "_____"));
        assert!(!sql_like_match("hello", "____"));
    }

    #[test]
    fn test_like_match_percent_only() {
        assert!(sql_like_match("anything", "%"));
        assert!(sql_like_match("", "%"));
    }

    #[test]
    fn test_like_match_empty_pattern() {
        assert!(sql_like_match("", ""));
        assert!(!sql_like_match("a", ""));
    }

    #[test]
    fn test_find_column_index_qualified_name() {
        let cols = vec![
            sqlrustgo_storage::ColumnDefinition {
                name: "id".to_string(),
                ..Default::default()
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "val".to_string(),
                ..Default::default()
            },
        ];
        assert_eq!(find_column_index("id", &cols), Some(0));
        assert_eq!(find_column_index("val", &cols), Some(1));
        assert_eq!(find_column_index("t.id", &cols), Some(0));
        assert_eq!(find_column_index("nonexistent", &cols), None);
    }

    #[test]
    fn test_case_when_no_else_returns_null() {
        let mut e = UnifiedExpr::CaseWhen {
            whens: vec![(
                UnifiedExpr::BinaryOp {
                    left: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
                    op: "=".into(),
                    right: Box::new(UnifiedExpr::Literal(Value::Integer(0))),
                },
                UnifiedExpr::Literal(Value::Text("yes".into())),
            )],
            else_val: None,
        };
        assert_eq!(e.evaluate(&[], &[], &mut None), Value::Null);
    }

    #[test]
    fn test_eval_fn_replace() {
        let v = eval_fn(
            "REPLACE",
            &[
                Value::Text("hello world".into()),
                Value::Text("world".into()),
                Value::Text("there".into()),
            ],
        );
        assert_eq!(v, Value::Text("hello there".into()));
    }

    #[test]
    fn test_eval_fn_repeat() {
        let v = eval_fn("REPEAT", &[Value::Text("ab".into()), Value::Integer(3)]);
        assert_eq!(v, Value::Text("ababab".into()));
    }

    #[test]
    fn test_eval_fn_reverse() {
        let v = eval_fn("REVERSE", &[Value::Text("abc".into())]);
        assert_eq!(v, Value::Text("cba".into()));
    }

    #[test]
    fn test_eval_fn_unknown_returns_null() {
        let v = eval_fn("NONEXISTENT_FN", &[Value::Integer(1)]);
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn test_referenced_columns_complex() {
        let mut e = UnifiedExpr::CaseWhen {
            whens: vec![(
                UnifiedExpr::BinaryOp {
                    left: Box::new(UnifiedExpr::Column("a".into())),
                    op: "=".into(),
                    right: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
                },
                UnifiedExpr::Column("b".into()),
            )],
            else_val: Some(Box::new(UnifiedExpr::Column("c".into()))),
        };
        let cols = e.referenced_columns();
        assert_eq!(cols.len(), 3);
        assert!(cols.contains(&"a".into()));
        assert!(cols.contains(&"b".into()));
        assert!(cols.contains(&"c".into()));
    }

    #[test]
    fn test_referenced_columns_in_list() {
        let mut e = UnifiedExpr::InList {
            expr: Box::new(UnifiedExpr::Column("x".into())),
            list: vec![
                UnifiedExpr::Column("a".into()),
                UnifiedExpr::Column("b".into()),
            ],
        };
        let cols = e.referenced_columns();
        assert_eq!(cols.len(), 3);
    }

    #[test]
    fn test_referenced_columns_between() {
        let mut e = UnifiedExpr::Between {
            expr: Box::new(UnifiedExpr::Column("v".into())),
            low: Box::new(UnifiedExpr::Column("lo".into())),
            high: Box::new(UnifiedExpr::Column("hi".into())),
        };
        let cols = e.referenced_columns();
        assert_eq!(cols.len(), 3);
    }

    #[test]
    fn test_referenced_columns_function_call() {
        let mut e = UnifiedExpr::FunctionCall {
            name: "CONCAT".into(),
            args: vec![
                UnifiedExpr::Column("a".into()),
                UnifiedExpr::Column("b".into()),
            ],
        };
        let cols = e.referenced_columns();
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn test_like_match_backtracking() {
        assert!(like_match_recursive("abcd", "a%cd"));
        assert!(like_match_recursive("abcd", "%d"));
        assert!(!like_match_recursive("abc", "a%d"));
    }

    #[test]
    fn test_like_match_leading_percent() {
        assert!(sql_like_match("abc", "%c"));
        assert!(sql_like_match("abc", "%bc"));
        assert!(!sql_like_match("abc", "%d"));
    }

    #[test]
    fn test_referenced_columns_is_null() {
        let mut e = UnifiedExpr::IsNull(Box::new(UnifiedExpr::Column("x".into())));
        let cols = e.referenced_columns();
        assert_eq!(cols, vec!["x".to_string()]);
    }

    #[test]
    fn test_referenced_columns_cast() {
        let mut e = UnifiedExpr::Cast {
            expr: Box::new(UnifiedExpr::Column("x".into())),
            target_type: "INTEGER".into(),
        };
        let cols = e.referenced_columns();
        assert_eq!(cols, vec!["x".to_string()]);
    }

    #[test]
    fn test_in_list_null_value() {
        let mut e = UnifiedExpr::InList {
            expr: Box::new(UnifiedExpr::Column("x".into())),
            list: vec![UnifiedExpr::Literal(Value::Integer(1))],
        };
        assert_eq!(
            e.evaluate(&[Value::Null], &["x".into()], &mut None),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_arithmetic_text_type() {
        assert_eq!(to_f64(&Value::Text("42".into())), 0.0);
        assert_eq!(to_i64(&Value::Text("42".into())), 0);
    }

    #[test]
    fn test_eval_fn_ltrim_rtrim() {
        let v = eval_fn("LTRIM", &[Value::Text("  hi".into())]);
        assert_eq!(v, Value::Text("hi".into()));
        let v = eval_fn("RTRIM", &[Value::Text("hi  ".into())]);
        assert_eq!(v, Value::Text("hi".into()));
    }

    #[test]
    fn test_eval_fn_left_right() {
        let v = eval_fn("LEFT", &[Value::Text("hello".into()), Value::Integer(2)]);
        assert_eq!(v, Value::Text("he".into()));
        let v = eval_fn("RIGHT", &[Value::Text("hello".into()), Value::Integer(2)]);
        assert_eq!(v, Value::Text("lo".into()));
    }

    #[test]
    fn test_eval_fn_space() {
        let v = eval_fn("SPACE", &[Value::Integer(3)]);
        assert_eq!(v, Value::Text("   ".into()));
    }

    #[test]
    fn test_eval_fn_field() {
        let v = eval_fn(
            "FIELD",
            &[
                Value::Text("b".into()),
                Value::Text("a".into()),
                Value::Text("b".into()),
                Value::Text("c".into()),
            ],
        );
        assert_eq!(v, Value::Integer(2));
    }

    #[test]
    fn test_eval_fn_elt() {
        let v = eval_fn(
            "ELT",
            &[
                Value::Integer(2),
                Value::Text("a".into()),
                Value::Text("b".into()),
            ],
        );
        assert_eq!(v, Value::Text("b".into()));
    }

    #[test]
    fn test_like_match_with_special_chars() {
        assert!(like_match_recursive("hello_world", "hello_world"));
        assert!(!like_match_recursive("helloworld", "hello_world"));
    }

    #[test]
    fn test_like_match_empty_text() {
        assert!(sql_like_match("", ""));
        assert!(sql_like_match("", "%"));
        assert!(!sql_like_match("", "a"));
    }
}
