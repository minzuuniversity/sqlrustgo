use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::Value;
use std::cell::RefCell;
use std::collections::HashMap;

// ============================================================================
// V312-58 / Issue #4512: scalar UDF (user-defined function) registry.
//
// `eval_fn` is a stateless function (no engine reference), so UDFs are
// stored in a thread-local registry. The active `ExecutionEngine` writes
// to it on `CREATE FUNCTION` / `DROP FUNCTION`; `eval_fn` reads from it
// after the built-in arms miss. Each `ExecutionEngine::execute` runs in
// the calling thread, so single-threaded test/repl usage is the natural
// fit. Multi-threaded scenarios would need to thread the registry
// through `evaluate(...)` — out of scope for v3.12.
// ============================================================================
#[derive(Debug, Clone)]
pub struct UdfDefinition {
    pub params: Vec<String>,
    /// Return type as declared in the UDF header (e.g. `INTEGER`,
    /// `VARCHAR(64)`). Stored verbatim for diagnostics and future
    /// strict-typing enforcement — not consumed by `invoke_udf` in
    /// v3.12 since `evaluate` infers the value's type from the
    /// resulting `Value` directly.
    #[allow(dead_code)]
    pub return_type: String,
    pub body_expr: String,
}

thread_local! {
    static UDF_REGISTRY: RefCell<HashMap<String, UdfDefinition>> =
        RefCell::new(HashMap::new());
}

/// V312-58 / Issue #4512: register a scalar UDF under `name`. Overwrites
/// any existing definition with the same case-insensitive name (matches
/// MySQL `CREATE OR REPLACE FUNCTION` semantics for the simple form).
pub fn register_udf(name: &str, params: Vec<String>, return_type: String, body_expr: String) {
    UDF_REGISTRY.with(|cell| {
        cell.borrow_mut().insert(
            name.to_uppercase(),
            UdfDefinition {
                params,
                return_type,
                body_expr,
            },
        );
    });
}

/// V312-58 / Issue #4512: remove a UDF by case-insensitive name. Returns
/// true if the UDF existed, false otherwise.
pub fn drop_udf(name: &str) -> bool {
    UDF_REGISTRY.with(|cell| cell.borrow_mut().remove(&name.to_uppercase()).is_some())
}

/// V312-58 / Issue #4512: true iff a UDF with this name (case-insensitive)
/// is currently registered. Used by `eval_fn` to decide whether to fall
/// through to the UDF evaluator.
pub fn has_udf(name: &str) -> bool {
    UDF_REGISTRY.with(|cell| cell.borrow().contains_key(&name.to_uppercase()))
}

/// V312-58 / Issue #4512: snapshot of the entire UDF registry. Tests and
/// diagnostics consume it to assert state without taking a mutable
/// borrow.
pub fn udf_snapshot() -> Vec<(String, UdfDefinition)> {
    UDF_REGISTRY.with(|cell| {
        cell.borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    })
}

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
            // MySQL system variable references: `@@version_comment`,
            // `@@autocommit`, etc. Resolved to a scalar literal at plan
            // time — we don't track session/scope state, so each variable
            // is resolved to a fixed string. This makes
            // `SELECT @@version_comment LIMIT 1` return a single
            // column with a single value, which is what mysql CLI 8.0+
            // expects during its boot probe.
            Expression::SystemVariable(name) => UnifiedExpr::Literal(resolve_system_variable(name)),
            _ => UnifiedExpr::Literal(Value::Null),
        }
    }
}

/// Resolve a MySQL system variable name (lowercase, no `@@` prefix) to
/// its current scalar value.
///
/// This is the executor-side counterpart to the lexer's `@@` tokenization
/// (see `crates/parser/src/lexer.rs`). It returns a `Value::Text` so
/// that `SELECT @@version_comment LIMIT 1` produces a text column when
/// the result set is rendered. Variables we don't model explicitly fall
/// back to empty string rather than NULL so mysql CLI 8.0+ does not
/// hang waiting for column values that never arrive.
pub fn resolve_system_variable(name: &str) -> Value {
    let key = name.to_ascii_lowercase();
    match key.as_str() {
        // mysql CLI 8.0+ boot probe expects a non-NULL scalar value.
        "version_comment" => Value::Text("SQLRustGo".to_string()),
        "version" => Value::Text(env!("CARGO_PKG_VERSION").to_string()),
        "version_compile_os" => Value::Text(std::env::consts::OS.to_string()),
        "version_compile_machine" => Value::Text(std::env::consts::ARCH.to_string()),
        // Session variables that are always on in our single-session mode.
        "autocommit" => Value::Integer(1),
        "sql_mode" => Value::Text(String::new()),
        // Anything else we don't model is returned as empty text rather
        // than NULL — this keeps the result set column count consistent
        // with row count (mysql CLI 8.0+ asserts columns == values per row).
        _ => Value::Text(String::new()),
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
        // Issue #4492: trim trailing whitespace on TEXT-vs-TEXT
        // comparison so CHAR(n) vs short string ordering is consistent
        // with the blank-padded equality rule.
        (Value::Text(l), Value::Text(r)) => {
            let lt = l.trim_end();
            let rt = r.trim_end();
            lt.cmp(rt) as i32
        }
        // (Value::Null, Value::Null) must compare equal so that
        // `compare_values` matches the documented doc-comment contract
        // (previously an explicit arm here; the trim-end refactor
        // accidentally let the catch-all `Null/_ => -1` shadow it).
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
    // V313-followup-1 / Issue #4154: case-exact first, fallback
    // case-insensitive.
    if let Some(idx) = columns.iter().position(|c| c.name == col_name) {
        return Some(idx);
    }
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
    // V312-26 / #4019-#4020 wire-protocol fix: when an identifier
    // arrives as `@@var` (e.g. the lexer tokenised it as Identifier
    // rather than SystemVariable), strip the `@@` prefix and resolve
    // it through the same scalar lookup as `Expression::SystemVariable`.
    if let Some(stripped) = name.strip_prefix("@@") {
        return Ok(resolve_system_variable(&stripped.to_ascii_lowercase()));
    }
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
///
/// Backtracking invariant: `star.0` is the *next* text index to try as
/// the start of the pattern segment following the most-recently-seen
/// `%` (i.e. `star.1`). On mismatch, we rewind `t_idx` to `star.0`,
/// try the segment again at that position, and advance `star.0` by
/// one so the *following* mismatch starts one position later. This
/// rewinds correctly even after the algorithm has consumed several
/// characters past the `%` (e.g. when the segment is "customer" and
/// the text contains back-to-back `c`s like "ironicCustomer" — the
/// naive `t_idx += 1` version skips re-trying the second `c`).
///
/// Regression for V312-48 #4278: TPC-H Q16's
/// `s_comment LIKE '%Customer%Complaints%'` was returning false on
/// supplier 358 (whose comment contains "ironicCustomer"). The naive
/// algorithm matched the `c` ending "ironic" against the first `c` of
/// "Customer", then failed to extend to `u`, then backtracked to
/// `t_idx += 1` which skipped the second `c` (start of "Customer").
fn like_match_recursive(text: &str, pattern: &str) -> bool {
    let mut t_idx = 0;
    let mut p_idx = 0;
    let t_bytes = text.as_bytes();
    let p_bytes = pattern.as_bytes();
    // star = (next text position to try as the segment start,
    //         pattern position of the segment, i.e. just after %).
    let mut star: Option<(usize, usize)> = None;

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
                    // Mismatch — rewind to the saved start position for
                    // the segment after the most-recent `%`, advance the
                    // saved start by one so the next retry starts at the
                    // next text position, then reset p_idx to the segment
                    // start. This re-tries `pat[star.1]` at `text[star.0]`
                    // and on subsequent failures advances star.0 one
                    // position further. Crucially: if t_idx has been
                    // advanced past star.0 by a (now-failed) prefix match,
                    // we still rewind back to star.0 so the failed
                    // position gets re-tried as a fresh segment start.
                    if let Some((st, ps)) = star {
                        star = Some((st + 1, ps));
                        t_idx = st;
                        p_idx = ps;
                        continue;
                    }
                    return false;
                }
            }
        } else {
            // Pattern exhausted but text has more. If we have a prior
            // `%`, rewind and advance one more text position for the
            // next retry.
            if let Some((st, ps)) = star {
                star = Some((st + 1, ps));
                t_idx = st;
                p_idx = ps;
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
        "=" | "==" => Value::Boolean(eq_cross(left, right)),
        "!=" | "<>" => Value::Boolean(!eq_cross(left, right)),
        ">" | "<" | ">=" | "<=" => compare_cmp(left, right, op),
        "AND" | "&&" => Value::Boolean(to_bool(left) && to_bool(right)),
        "OR" | "||" => Value::Boolean(to_bool(left) || to_bool(right)),
        // V312-22b / Issue #4036: include `%` so `i % 2` evaluates to a real
        // value. Without this arm, `i % 2` returns `Value::Null`, breaking
        // TPC-H Q4 / sqllogictest `WHERE i % 2 <> 0` filtering (NULL is
        // UNKNOWN, matches no rows under SQL three-valued logic).
        "+" | "-" | "*" | "/" | "%" => eval_arithmetic(left, right, op),
        "->" => json_extract(left, right, false),
        "->>" => json_extract(left, right, true),
        _ => Value::Null,
    }
}

/// Cross-type equality for SQL. Two values compare equal if:
/// - They share a tag (derive PartialEq succeeds), OR
/// - One is Boolean and the other is a numeric type with the same
///   truthiness/zero-ness (e.g. `TRUE = 1`, `FALSE = 0`), OR
/// - One is Integer and the other is Float and they represent the
///   same numeric value.
///
/// Returns false if either side is NULL (SQL three-valued logic:
/// NULL = NULL is NULL/false, not true).
fn eq_cross(left: &Value, right: &Value) -> bool {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return false;
    }
    // Issue #4492: SQL standard blank-padded equality for CHAR(n) columns.
    // `'F ' = 'F'` must compare true (MySQL semantics); we trim trailing
    // whitespace on both sides before delegating to the strict PartialEq.
    // This branch is intentionally limited to TEXT-vs-TEXT so it does NOT
    // affect Hash/sort/group-by (which use PartialEq directly via
    // `Value::eq`).
    if let (Value::Text(a), Value::Text(b)) = (left, right) {
        let a_trim = a.trim_end();
        let b_trim = b.trim_end();
        return a_trim == b_trim;
    }
    if left == right {
        return true;
    }
    match (left, right) {
        (Value::Boolean(a), b) | (b, Value::Boolean(a)) => to_bool(b) == *a,
        (Value::Integer(a), Value::Float(b)) | (Value::Float(b), Value::Integer(a)) => {
            (*a as f64) == *b
        }
        // V312-bug-report-3120 / BUG-4: MySQL CHAR(n) is blank-padded
        // on store (e.g. CHAR(2) of 'F' is stored as "F "), so an
        // equality / inequality against the bare literal 'F' must
        // ignore trailing spaces. SQLite has the same behaviour.
        // Trimming here (not in PartialEq/Hash) preserves the
        // hash/eq invariant for HashMap-backed GROUP BY / DISTINCT.
        (Value::Text(l), Value::Text(r)) => l.trim_end() == r.trim_end(),
        _ => false,
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
        if r == 0.0 && (op == "/" || op == "%") {
            return Value::Null;
        }
        let result = match op {
            "+" => l + r,
            "-" => l - r,
            "*" => l * r,
            "/" => l / r,
            "%" => l % r,
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
            "%" => {
                if r == 0 {
                    0
                } else {
                    l % r
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
        // V312-59-D / Issue #4572: MySQL 2/124 — text→numeric implicit
        // coercion. Strings that parse as a number return that number;
        // non-numeric strings return 0 (MySQL's legacy behavior; SQLite
        // is stricter and returns 0 too). Float parse: full string must
        // match (allow leading/trailing whitespace); failed parse = 0.0.
        Value::Text(s) => s.trim().parse::<f64>().unwrap_or(0.0),
        Value::Null | Value::Blob(_) | Value::Point(_, _) | Value::Json(_) => 0.0,
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
        // V312-59-D / Issue #4572: text→integer implicit coercion. Numeric
        // text parses directly. Decimal text (e.g. "3.7") truncates
        // toward zero (matches MySQL CAST AS SIGNED). Empty / non-numeric
        // → 0. Float input is also coerced (rounds toward zero via
        // as_i64 cast). Match MySQL 2/124 legacy semantics.
        Value::Text(s) => {
            let trimmed = s.trim();
            trimmed.parse::<i64>().unwrap_or_else(|_| {
                // Try decimal parse → truncate to i64 (MySQL CAST AS SIGNED).
                trimmed.parse::<f64>().map(|f| f as i64).unwrap_or(0)
            })
        }
        Value::Float(f) => *f as i64,
        Value::Null | Value::Blob(_) | Value::Point(_, _) | Value::Json(_) => 0,
    }
}

/// V312-59-D / Issue #4572: explicit type coercion for CAST(... AS TYPE)
/// and CONVERT(expr, TYPE). Returns the source value converted to the
/// target type. The target type string is case-insensitive and accepts
/// MySQL's type aliases.
///
/// Supported targets (case-insensitive):
///   SIGNED, INTEGER, INT           → Integer (text → i64; float → as i64)
///   UNSIGNED                        → Integer (text/float → i64; negatives clamped to 0)
///   FLOAT, DOUBLE, REAL, DECIMAL    → Float (text → f64; integer → as f64)
///   CHAR, TEXT, VARCHAR             → Text (string repr; trim trailing spaces per MySQL)
///   DATE, DATETIME, TIME            → pass through as Text (full date/time
///                                    conversion is out of scope; sqlrustgo
///                                    does not yet have a native date type)
///   BINARY, BLOB                    → pass through as Blob if Text → bytes
///   JSON                            → parse text as JSON if possible, else Null
///   anything else                   → pass through unchanged (no-op)
///
/// MySQL semantics: non-numeric text → 0 (not error). sqlrustgo matches
/// MySQL 5.7 / MariaDB here; SQLite 3.x is stricter.
fn cast_value(val: &Value, target: &str) -> Value {
    let target = target.trim();
    match target.to_uppercase().as_str() {
        "SIGNED" | "INTEGER" | "INT" => Value::Integer(to_i64(val)),
        "UNSIGNED" => {
            let n = to_i64(val);
            Value::Integer(if n < 0 { 0 } else { n })
        }
        "FLOAT" | "DOUBLE" | "REAL" | "DECIMAL" | "NUMERIC" => Value::Float(to_f64(val)),
        "CHAR" | "TEXT" | "VARCHAR" => {
            // MySQL CHAR trims trailing spaces per column width; for our
            // purpose (text→text) trim all trailing whitespace.
            Value::Text(val.to_sql_string().trim_end().to_string())
        }
        "DATE" | "DATETIME" | "TIMESTAMP" | "TIME" => {
            // sqlrustgo has no native date/time type; preserve the text form.
            Value::Text(val.to_sql_string().trim().to_string())
        }
        "BINARY" | "BLOB" | "VARBINARY" => {
            // No native blob from text in this path; preserve text round-trip.
            // (Binary-coercion across types is out of scope for #4572.)
            Value::Text(val.to_sql_string().trim().to_string())
        }
        "JSON" => match val {
            Value::Json(v) => Value::Json(v.clone()),
            Value::Text(s) => match serde_json::from_str::<serde_json::Value>(s) {
                Ok(v) => Value::Json(v),
                Err(_) => Value::Null,
            },
            _ => Value::Null,
        },
        "BOOLEAN" | "BOOL" => Value::Boolean(to_i64(val) != 0),
        "YEAR" => Value::Integer(to_i64(val)),
        _ => val.clone(),
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
///
/// MySQL 5.7 JSON path operators: `->` and `->>`.
/// `unquote` = false → returns `Value::Json`, true → returns `Value::Text`.
fn json_extract(left: &Value, right: &Value, unquote: bool) -> Value {
    let doc = match left {
        Value::Json(v) => v.clone(),
        Value::Text(s) => match serde_json::from_str(s) {
            Ok(v) => v,
            Err(_) => return Value::Null,
        },
        _ => return Value::Null,
    };

    let path = match right {
        Value::Text(s) => s.clone(),
        Value::Json(serde_json::Value::String(s)) => s.clone(),
        _ => return Value::Null,
    };

    // serde_json::Value::pointer uses RFC 6901 JSON Pointer syntax
    // (e.g. `/foo/0/bar`), but the SQL surface is MySQL JSONPath
    // (e.g. `$.foo[0].bar`). Normalize: drop leading `$.` and rewrite
    // each `.`/bracket segment into an RFC 6901 `/`-prefixed token.
    let json_pointer = if path == "$" {
        String::new()
    } else if let Some(rest) = path.strip_prefix("$.") {
        let mut p = String::new();
        for segment in rest.split('.') {
            if let Some(idx_start) = segment.find('[') {
                let name = &segment[..idx_start];
                let idx_part = &segment[idx_start..];
                p.push('/');
                p.push_str(name);
                let cleaned: String = idx_part
                    .chars()
                    .filter(|c| *c != '[' && *c != ']')
                    .collect();
                p.push('/');
                p.push_str(&cleaned);
            } else {
                p.push('/');
                p.push_str(segment);
            }
        }
        p
    } else if path.starts_with('$') {
        // Bare `$` already handled; `$[N]` style — strip the `$`.
        path.strip_prefix('$').unwrap_or(&path).to_string()
    } else {
        // Caller supplied an RFC 6901 pointer directly; pass through.
        path.clone()
    };

    match doc.pointer(&json_pointer) {
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
        // V312-26 / #4020: MySQL wire-client compatibility. The MySQL
        // CLI sends `SELECT DATABASE()` after every connect to
        // determine the current schema before sending `USE db`. Returning
        // an empty string (or NULL) lets the client proceed to send
        // subsequent queries; sqlrustgo's `USE <db>` is a no-op in v3.12
        // (single-database mode), so the actual current schema is the
        // empty string. See execute_use_database in
        // src/execution_engine.rs.
        "DATABASE" | "SCHEMA" => Value::Text(String::new()),
        // V312-58 / #4518: MySQL compat — CURRENT_USER / USER / etc.
        // sqlrustgo runs in single-user mode; return the canonical
        // default user identifier "openclaw@%" so SHOW GRANTS / wire
        // protocol probes (mysql CLI 8.0+) receive a non-NULL
        // identifier instead of an empty string. Real session-scoped
        // user state requires auth wiring that is not yet plumbed
        // through the eval_fn surface.
        "USER" | "CURRENT_USER" | "SESSION_USER" | "SYSTEM_USER" => {
            Value::Text("openclaw@%".to_string())
        }
        // V312-58 / #4518: utility scalar functions required by
        // 清华 MySQL 课程 A 轨上机 (第 7/9/12 章). Each arm preserves
        // MySQL semantics:
        //   IFNULL(a, b)         = first non-NULL
        //   CONCAT(a, b, ...)    = string concat, NULL treated as ''
        //   CHAR_LENGTH / LEN(s)  = char count (UTF-8 codepoints, not bytes)
        //   LAST_INSERT_ID()     = 0 (stateless; no AUTO_INCREMENT tracking)
        //   CONVERT(expr, type)  = passthrough CAST semantics
        "IFNULL" => args
            .iter()
            .find(|v| !matches!(v, Value::Null))
            .cloned()
            .unwrap_or(Value::Null),
        "CONCAT" => Value::Text(
            args.iter()
                .map(|v| match v {
                    Value::Null => String::new(),
                    _ => v.to_sql_string(),
                })
                .collect::<Vec<_>>()
                .join(""),
        ),
        "CHAR_LENGTH" | "CHARACTER_LENGTH" => args
            .first()
            .map(|v| Value::Integer(v.to_sql_string().chars().count() as i64))
            .unwrap_or(Value::Null),
        "LAST_INSERT_ID" => Value::Integer(0),
        // V312-59-D / Issue #4572: CAST(expr AS TYPE) — proper MySQL
        // type coercion. The parser discards the target type on the
        // primary route (see crates/parser/src/parser.rs CAST arm),
        // but the secondary route (where CAST is parsed as
        // FunctionCall with the type identifier as a second
        // Expression::Literal arg) lands here. We handle both:
        //   args[0] = source value
        //   args[1] = target type (Literal text like "SIGNED", "INTEGER",
        //                       "TEXT", "CHAR", "DATE", "FLOAT",
        //                       "DOUBLE", "DECIMAL", "BINARY")
        // MySQL 2/124 compatibility: CAST('123' AS SIGNED) → Integer(123),
        // CAST('abc' AS SIGNED) → Integer(0), CAST(1.5 AS SIGNED) → Integer(1).
        // MySQL 8 CAST AS CHAR behaves as CAST AS TEXT in sqlrustgo.
        "CAST" => {
            let val = args.first().cloned().unwrap_or(Value::Null);
            let target = args
                .get(1)
                .map(|v| v.to_sql_string().to_uppercase())
                .unwrap_or_else(|| "TEXT".to_string());
            cast_value(&val, &target)
        }
        // V312-59-D / Issue #4572: CONVERT(expr, type) — equivalent to
        // CAST in MySQL. The parser's CONVERT path already routes
        // TYPE as Expression::Literal (see parser.rs Token::Convert
        // handler). Use the same cast_value helper as CAST.
        "CONVERT" => {
            let val = args.first().cloned().unwrap_or(Value::Null);
            let target = args
                .get(1)
                .map(|v| v.to_sql_string().to_uppercase())
                .unwrap_or_else(|| "TEXT".to_string());
            cast_value(&val, &target)
        }
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
        // V312-bug-report-3120 / BUG-2b: MySQL date/time + numeric
        // functions required by 清华 MySQL 课程 A 轨上机. Previously
        // these returned Null silently because the arms were missing.
        // NOW()/SYSDATE() -> "YYYY-MM-DD HH:MM:SS" (server local time).
        // We use civil_from_days on the unix epoch second count so we
        // don't pull in chrono just for these scalar functions.
        "NOW" | "SYSDATE" | "CURRENT_TIMESTAMP" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let (y, m, d) = civil_from_days(secs / 86400);
            let tod = secs.rem_euclid(86400);
            let h = tod / 3600;
            let mi = (tod % 3600) / 60;
            let s = tod % 60;
            Value::Text(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                y, m, d, h, mi, s
            ))
        }
        "CURDATE" | "CURRENT_DATE" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let (y, m, d) = civil_from_days(secs / 86400);
            Value::Text(format!("{:04}-{:02}-{:02}", y, m, d))
        }
        "CURTIME" | "CURRENT_TIME" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let tod = secs.rem_euclid(86400);
            let h = tod / 3600;
            let mi = (tod % 3600) / 60;
            let s = tod % 60;
            Value::Text(format!("{:02}:{:02}:{:02}", h, mi, s))
        }
        // YEAR/MONTH/DAY(date) -> Integer. Accept 'YYYY-MM-DD' or any
        // prefix that has the field bytes available. Returns Null on
        // bad input rather than panicking on a slice boundary.
        "YEAR" => parse_date_field(args, 0, 4),
        "MONTH" => parse_date_field(args, 5, 7),
        "DAY" | "DAYOFMONTH" => parse_date_field(args, 8, 10),
        // DATEDIFF(d1, d2) -> Integer days (d1 - d2), MySQL semantics.
        "DATEDIFF" => {
            if args.len() != 2 {
                return Value::Null;
            }
            match (
                parse_date_to_days(&args[0].to_sql_string()),
                parse_date_to_days(&args[1].to_sql_string()),
            ) {
                (Some(a), Some(b)) => Value::Integer(a - b),
                _ => Value::Null,
            }
        }
        // Issue #4579 / BustubX-EDU B-track case 15: strftime(format, date)
        // — SQLite-compatible date formatting. Accepts the same
        // 'YYYY-MM-DD' / 'YYYY-MM-DD HH:MM:SS' shape as the other date
        // helpers, plus the literal 'now' for the current UTC instant.
        // Supported specifiers: %Y %m %d %H %M %S %j %w %% (literal %).
        // Unknown specifiers pass through verbatim so callers can extend
        // without an engine change.
        "STRFTIME" => strftime_value(args),
        // ROUND(x [, d]) — half-away-from-zero. d default 0.
        // MySQL returns INTEGER when d<=0, FLOAT when d>0.
        "ROUND" => {
            let x = match args.first() {
                Some(Value::Integer(i)) => *i as f64,
                Some(Value::Float(f)) => *f,
                Some(v) => v.to_sql_string().parse::<f64>().unwrap_or(0.0),
                None => return Value::Null,
            };
            let d = match args.get(1) {
                Some(Value::Integer(i)) => *i,
                Some(v) => v.to_sql_string().parse::<i64>().unwrap_or(0),
                None => 0,
            };
            let scale = 10f64.powi(d as i32);
            let rounded = (x * scale).round() / scale;
            if d <= 0 {
                Value::Integer(rounded as i64)
            } else {
                Value::Float(rounded)
            }
        }
        // RAND([seed]) -> Float in [0, 1). Uses a thread-local
        // splitmix64-style PRNG seeded from wall-clock nanoseconds so
        // each call advances state. The optional seed argument, if
        // Integer, replaces the state (matches MySQL's RAND(N) reseed
        // semantics well enough for classroom use).
        "RAND" => {
            use std::cell::Cell;
            thread_local!(static SEED: Cell<u64> = Cell::new({
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0xdead_beef_u64)
                    .wrapping_add(1)
            }));
            SEED.with(|c| {
                if let Some(Value::Integer(n)) = args.first() {
                    c.set(*n as u64);
                }
                let mut s = c.get().wrapping_add(0x9e3779b97f4a7c15);
                s = (s ^ (s >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                s = (s ^ (s >> 27)).wrapping_mul(0x94d049bb133111eb);
                s = s ^ (s >> 31);
                c.set(s);
                Value::Float((s as f64) / (u64::MAX as f64))
            })
        }
        "ABS" => match args.first() {
            Some(Value::Integer(i)) => Value::Integer(i.abs()),
            Some(Value::Float(f)) => Value::Float(f.abs()),
            Some(v) => {
                let s = v.to_sql_string();
                if let Ok(i) = s.parse::<i64>() {
                    Value::Integer(i.abs())
                } else if let Ok(f) = s.parse::<f64>() {
                    Value::Float(f.abs())
                } else {
                    Value::Null
                }
            }
            None => Value::Null,
        },
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
        // (Old CAST passthrough removed by V312-59-D / Issue #4572; the new
        // CAST arm above threads the target type through eval_fn.)
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
            if std::env::var("Q7_TRACE").is_ok() {
                eprintln!(
                    "[Q7_TRACE] EXTRACT field={:?} source={:?} (len={})",
                    field,
                    source,
                    source.len()
                );
            }
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
                st_contains as gis_st_contains, st_distance as gis_st_distance,
                st_intersects as gis_st_intersects, st_within as gis_st_within, Point as GisPoint,
                Polygon as GisPolygon,
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
            let path = args
                .get(1)
                .map(|v| v.to_sql_string())
                .unwrap_or_else(|| "$".to_string());
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
        // ====================================================================
        // Issue #4490 — register commonly-missing SQL scalar functions so they
        // no longer fall through to the Null default branch.
        //
        // V312-bugfix / #4490 PR #4508 originally added these arms here.
        // After rebase on top of develop/v3.12.0 (which already carries
        // PR #4493's BUG-2b matching arms earlier in the same eval_fn
        // match block), this block became unreachable. Rust's match
        // dispatch routes the earlier arms first. Removed during
        // PR #4508 merge-conflict resolution to silence the
        // "unreachable pattern" warnings. The PR #4493 implementation
        // is now the single source of truth.
        // ====================================================================
        // ====================================================================
        // V312-58 / Issue #4512: scalar UDF dispatch. After all built-in
        // arms miss, look up the (case-insensitive) name in the
        // thread-local UDF registry. If found, re-parse the stored
        // body expression (substituting call-site arg values for the
        // declared parameter names) and evaluate it. Re-parsing keeps
        // us on the existing expression evaluator — no need to teach
        // UDFs about row/column/storage contexts because the body only
        // sees its own argument values plus literals.
        // ====================================================================
        _ => {
            let udf = UDF_REGISTRY.with(|cell| cell.borrow().get(&name.to_uppercase()).cloned());
            if let Some(def) = udf {
                return invoke_udf(&def, args);
            }
            Value::Null
        }
    }
}

/// V312-58 / Issue #4512: invoke a registered scalar UDF with the given
/// arguments. The function:
/// 1. Re-tokenizes + re-parses the stored body expression text.
/// 2. Walks the resulting `sqlrustgo_parser::Expression` and substitutes
///    every `Identifier` whose name matches a declared parameter with a
///    `Literal` carrying the canonical SQL text form of the matching
///    argument value (e.g. `Value::Text("o'b")` becomes
///    `Literal("'o''b'")`).
/// 3. Converts the resulting `Expression` into a `UnifiedExpr` and
///    evaluates it in an empty row context.
///
/// Any failure (re-parse error, arity mismatch, evaluation error)
/// collapses to `Value::Null` rather than a hard error — scalar UDFs are
/// expected to degrade gracefully so a single buggy function does not
/// bring down the surrounding statement.
fn invoke_udf(def: &UdfDefinition, args: &[Value]) -> Value {
    use sqlrustgo_parser::parse_expression_str;
    let raw_expr = match parse_expression_str(&def.body_expr) {
        Ok(e) => e,
        Err(_) => return Value::Null,
    };
    if args.len() != def.params.len() {
        return Value::Null;
    }
    let substituted = substitute_udf_params(&raw_expr, &def.params, args);
    let mut uexpr: UnifiedExpr = UnifiedExpr::from(&substituted);
    uexpr.evaluate(&[], &[], &mut None)
}

/// V312-58 / Issue #4512: walk an `Expression` and replace each
/// `Identifier` that matches one of the declared parameter names with
/// the matching argument's value. Comparison is case-insensitive, with
/// the parameter list taking precedence over column names that happen
/// to share a name (matches MySQL's local-variable-shadowing-column
/// semantics inside stored functions).
fn substitute_udf_params(
    expr: &sqlrustgo_parser::Expression,
    params: &[String],
    args: &[Value],
) -> sqlrustgo_parser::Expression {
    use sqlrustgo_parser::Expression;
    let lookup = |name: &str| -> Option<String> {
        let upper = name.to_ascii_uppercase();
        params
            .iter()
            .position(|p| p.to_ascii_uppercase() == upper)
            .map(|idx| value_to_sql_literal(&args[idx]))
    };
    match expr {
        Expression::Identifier(name) => match lookup(name) {
            Some(lit) => Expression::Literal(lit),
            None => Expression::Identifier(name.clone()),
        },
        Expression::Literal(_)
        | Expression::JsonLiteral(_)
        | Expression::SystemVariable(_)
        | Expression::SequenceNextVal(_)
        | Expression::SequenceCurrval(_)
        | Expression::Subquery(_)
        | Expression::SubqueryField(_, _)
        | Expression::In(_, _)
        | Expression::NotIn(_, _)
        | Expression::Exists(_)
        | Expression::NotExists(_)
        | Expression::QuantifiedOp(_, _, _)
        | Expression::Aggregate(_)
        | Expression::WindowCall(_)
        | Expression::ArrayLiteral(_) => expr.clone(),
        Expression::BinaryOp(l, op, r) => Expression::BinaryOp(
            Box::new(substitute_udf_params(l, params, args)),
            op.clone(),
            Box::new(substitute_udf_params(r, params, args)),
        ),
        Expression::UnaryOp(op, e) => {
            Expression::UnaryOp(op.clone(), Box::new(substitute_udf_params(e, params, args)))
        }
        Expression::IsNull(e) => {
            Expression::IsNull(Box::new(substitute_udf_params(e, params, args)))
        }
        Expression::IsNotNull(e) => {
            Expression::IsNotNull(Box::new(substitute_udf_params(e, params, args)))
        }
        Expression::InList(e, list) => Expression::InList(
            Box::new(substitute_udf_params(e, params, args)),
            list.iter()
                .map(|i| substitute_udf_params(i, params, args))
                .collect(),
        ),
        Expression::NotInList(e, list) => Expression::NotInList(
            Box::new(substitute_udf_params(e, params, args)),
            list.iter()
                .map(|i| substitute_udf_params(i, params, args))
                .collect(),
        ),
        Expression::Like(e, p, esc) => Expression::Like(
            Box::new(substitute_udf_params(e, params, args)),
            Box::new(substitute_udf_params(p, params, args)),
            *esc,
        ),
        Expression::NotLike(e, p, esc) => Expression::NotLike(
            Box::new(substitute_udf_params(e, params, args)),
            Box::new(substitute_udf_params(p, params, args)),
            *esc,
        ),
        Expression::Between(e, lo, hi) => Expression::Between(
            Box::new(substitute_udf_params(e, params, args)),
            Box::new(substitute_udf_params(lo, params, args)),
            Box::new(substitute_udf_params(hi, params, args)),
        ),
        Expression::NotBetween(e, lo, hi) => Expression::NotBetween(
            Box::new(substitute_udf_params(e, params, args)),
            Box::new(substitute_udf_params(lo, params, args)),
            Box::new(substitute_udf_params(hi, params, args)),
        ),
        Expression::NotRegexp(e, p) => Expression::NotRegexp(
            Box::new(substitute_udf_params(e, params, args)),
            Box::new(substitute_udf_params(p, params, args)),
        ),
        Expression::CaseWhen(whens, else_val) => Expression::CaseWhen(
            whens
                .iter()
                .map(|w| sqlrustgo_parser::WhenClause {
                    condition: substitute_udf_params(&w.condition, params, args),
                    result: substitute_udf_params(&w.result, params, args),
                })
                .collect(),
            else_val
                .as_ref()
                .map(|e| Box::new(substitute_udf_params(e, params, args))),
        ),
        Expression::FunctionCall(name, fargs) => Expression::FunctionCall(
            name.clone(),
            fargs
                .iter()
                .map(|a| substitute_udf_params(a, params, args))
                .collect(),
        ),
    }
}

/// V312-58 / Issue #4512: render a `Value` as the canonical SQL literal
/// text the re-parser expects. `NULL` / `TRUE` / `FALSE` / numerics /
/// single-quoted strings — every form is produced so that
/// `parse_lit` round-trips it correctly.
fn value_to_sql_literal(v: &Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Boolean(true) => "TRUE".to_string(),
        Value::Boolean(false) => "FALSE".to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Json(j) => j.to_string(),
        Value::Blob(b) => {
            // V312-58: `hex` crate isn't a direct dependency of the
            // executor crate, so we encode inline rather than depending
            // on a transitive path. Values are already bytes so a
            // simple nibble map is sufficient.
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let mut s = String::with_capacity(b.len() * 2);
            for byte in b {
                s.push(HEX[(byte >> 4) as usize] as char);
                s.push(HEX[(byte & 0x0f) as usize] as char);
            }
            format!("X'{}'", s)
        }
        Value::Point(x, y) => format!("POINT({}, {})", x, y),
    }
}

/// DATE_ADD / DATE_SUB helper. Operates on text dates in YYYY-MM-DD form.
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
// =====================================================================
// Issue #4490 — date/time helpers (NOW / CURDATE / CURTIME / YEAR / MONTH /
// DAY / DATEDIFF eval_fn arms) were originally introduced here by PR #4508.
// After rebase on top of develop/v3.12.0 (which already carries PR #4493's
// matching arms at the start of the eval_fn match block), the entire helper
// set is dead code: the matching arms in eval_fn route first, so this file's
// `parse_date_to_days` / `wall_clock_secs` / `current_timestamp_text` /
// `extract_time_component` / `extract_date_component_opt` are never reached.
// Removed during PR #4508 merge-conflict resolution to eliminate the
// duplicate-`parse_date_to_days` E0428 compile error. PR #4493's helpers
// further down (`parse_date_field` / `parse_date_to_days` / `civil_from_days` /
// `days_from_civil`) remain the single source of truth.
// =====================================================================

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

/// V312-bug-report-3120 / BUG-2b: extract an Integer field from a
/// 'YYYY-MM-DD'-shaped string at byte offsets [start, end). Returns
/// Null on missing arg, short string, non-digit content, or slice
/// not on a char boundary. Uses str::get so we never panic.
fn parse_date_field(args: &[Value], start: usize, end: usize) -> Value {
    let s = match args.first() {
        Some(v) => v.to_sql_string(),
        None => return Value::Null,
    };
    s.get(start..end)
        .and_then(|slice| slice.parse::<i64>().ok())
        .map(Value::Integer)
        .unwrap_or(Value::Null)
}

/// Issue #4579 / BustubX-EDU B-track case 15: SQLite-compatible
/// `strftime(format, date)`. Returns Null on any error (no exception
/// surface). Recognizes 'now' / 'now' suffix as the current UTC
/// instant for the second argument; otherwise parses 'YYYY-MM-DD' or
/// 'YYYY-MM-DD HH:MM:SS'.
///
/// Supported format specifiers:
///   %Y 4-digit year      %m 2-digit month     %d 2-digit day
///   %H 2-digit hour      %M 2-digit minute    %S 2-digit second
///   %j day-of-year       %w weekday 0..6      %% literal '%'
///
/// Unknown specifiers pass through verbatim, matching SQLite's
/// tolerance for forward-compat with future format codes.
fn strftime_value(args: &[Value]) -> Value {
    if args.len() != 2 {
        return Value::Null;
    }
    let fmt = args[0].to_sql_string();
    let raw = args[1].to_sql_string();
    let trimmed = raw.trim();

    // Resolve date source: 'now' keyword uses current UTC; otherwise
    // parse the textual timestamp into (year, month, day, h, m, s).
    let mut y: i64 = 1970;
    let mut mo: i64 = 1;
    let mut d: i64 = 1;
    let mut hh: i64 = 0;
    let mut mi: i64 = 0;
    let mut ss: i64 = 0;

    let mut parsed = false;
    let lower = trimmed.to_ascii_lowercase();
    if lower == "now" {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|t| t.as_secs() as i64)
            .unwrap_or(0);
        let (yy, mm, dd) = civil_from_days(secs / 86400);
        let tod = secs.rem_euclid(86400);
        y = yy;
        mo = mm;
        d = dd;
        hh = tod / 3600;
        mi = (tod % 3600) / 60;
        ss = tod % 60;
        parsed = true;
    } else if trimmed.len() >= 10 {
        // 'YYYY-MM-DD' or 'YYYY-MM-DD HH:MM:SS'
        if let (Some(yy), Some(mm), Some(dd)) = (
            trimmed.get(0..4).and_then(|s| s.parse::<i64>().ok()),
            trimmed.get(5..7).and_then(|s| s.parse::<i64>().ok()),
            trimmed.get(8..10).and_then(|s| s.parse::<i64>().ok()),
        ) {
            if (1..=12).contains(&mm) && (1..=31).contains(&dd) {
                y = yy;
                mo = mm;
                d = dd;
                parsed = true;
            }
        }
        // Optional ' HH:MM:SS' suffix
        if parsed && trimmed.len() >= 19 {
            if let (Some(h), Some(m), Some(s)) = (
                trimmed.get(11..13).and_then(|s| s.parse::<i64>().ok()),
                trimmed.get(14..16).and_then(|s| s.parse::<i64>().ok()),
                trimmed.get(17..19).and_then(|s| s.parse::<i64>().ok()),
            ) {
                if (0..24).contains(&h) && m < 60 && s < 60 {
                    hh = h;
                    mi = m;
                    ss = s;
                }
            }
        }
    }
    if !parsed {
        return Value::Null;
    }

    // Day-of-year (1..=366) via days_from_civil epoch.
    let doy = days_from_civil(y, mo, d);

    // Weekday (0=Sunday..6=Saturday). days_from_civil(1970,1,1) was
    // Thursday (weekday=4). Adjust by hand from a known anchor.
    let weekday = ((doy + 3).rem_euclid(7)) as i64;

    let mut out = String::with_capacity(fmt.len() + 8);
    let bytes = fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            let spec = bytes[i + 1] as char;
            let repl: Option<String> = match spec {
                'Y' => Some(format!("{:04}", y)),
                'm' => Some(format!("{:02}", mo)),
                'd' => Some(format!("{:02}", d)),
                'H' => Some(format!("{:02}", hh)),
                'M' => Some(format!("{:02}", mi)),
                'S' => Some(format!("{:02}", ss)),
                'j' => Some(format!("{:03}", doy)),
                'w' => Some(format!("{}", weekday)),
                '%' => Some("%".to_string()),
                _ => None, // unknown: pass through verbatim
            };
            if let Some(s) = repl {
                out.push_str(&s);
                i += 2;
                continue;
            }
            // unknown specifier: fall through to literal
        }
        // Push one UTF-8 char (format strings are ASCII in practice;
        // a multi-byte char passes through unchanged via char boundary).
        let ch = fmt[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    Value::Text(out)
}

/// V312-bug-report-3120 / BUG-2b: parse 'YYYY-MM-DD' (optionally
/// followed by ' HH:MM:SS') into days-since-Unix-epoch via
/// days_from_civil. None on malformed input.
fn parse_date_to_days(s: &str) -> Option<i64> {
    let date_part = s.get(..10.min(s.len()))?;
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y = parts[0].parse::<i64>().ok()?;
    let m = parts[1].parse::<i64>().ok()?;
    let d = parts[2].parse::<i64>().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
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
    // Issue #4492: trim trailing whitespace on TEXT operands before
    // comparing so that ordering is consistent with the blank-padded
    // equality rule (CHAR(n) vs short string).
    let cmp = match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(b) as i64,
        (Value::Text(a), Value::Text(b)) => {
            let at = a.trim_end();
            let bt = b.trim_end();
            at.cmp(bt) as i64
        }
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
    fn test_blank_padded_equality() {
        // Issue #4492: trailing-space string vs short string.
        assert_eq!(
            eval_binary_op(
                &Value::Text("F".to_string()),
                &Value::Text("F ".to_string()),
                "=",
            ),
            Value::Boolean(true),
        );
        assert_eq!(
            eval_binary_op(
                &Value::Text("abc".to_string()),
                &Value::Text("abc ".to_string()),
                "=",
            ),
            Value::Boolean(true),
        );
        // Leading whitespace preserved.
        assert_eq!(
            eval_binary_op(
                &Value::Text(" abc".to_string()),
                &Value::Text("abc".to_string()),
                "=",
            ),
            Value::Boolean(false),
        );
        // Distinct content.
        assert_eq!(
            eval_binary_op(
                &Value::Text("abc".to_string()),
                &Value::Text("def".to_string()),
                "=",
            ),
            Value::Boolean(false),
        );
        // <>, <= ordering use trimmed compare.
        assert_eq!(
            eval_binary_op(
                &Value::Text("abc".to_string()),
                &Value::Text("abc ".to_string()),
                "<=",
            ),
            Value::Boolean(true),
        );
    }

    #[test]
    fn test_partial_eq_strict_preserved() {
        // Hash/sort/group-by must NOT collapse 'F' and 'F '.
        let a = Value::Text("F".to_string());
        let b = Value::Text("F ".to_string());
        assert_ne!(a, b);
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

    /// V312-22b / Issue #4036: regression tests for the modulo operator
    /// (`%`). Before this fix, `eval_binary_op` did not handle `"%"`,
    /// returning `Value::Null` for any modulo expression, which broke
    /// TPC-H Q4 / sqllogictest `WHERE i % 2 <> 0` filtering (NULL is
    /// UNKNOWN in SQL three-valued logic and matches no rows).
    #[test]
    fn test_eval_arithmetic_modulo_integer() {
        assert_eq!(
            eval_arithmetic(&Value::Integer(7), &Value::Integer(3), "%"),
            Value::Integer(1)
        );
        assert_eq!(
            eval_arithmetic(&Value::Integer(8), &Value::Integer(4), "%"),
            Value::Integer(0)
        );
        assert_eq!(
            eval_arithmetic(&Value::Integer(1), &Value::Integer(2), "%"),
            Value::Integer(1)
        );
    }

    #[test]
    fn test_eval_arithmetic_modulo_float() {
        // Float operands (e.g. `5.5 % 2.0`) should also evaluate via `%`.
        let result = eval_arithmetic(&Value::Float(5.5), &Value::Float(2.0), "%");
        if let Value::Float(f) = result {
            assert!((f - 1.5).abs() < 1e-9, "expected 1.5, got {}", f);
        } else {
            panic!("expected Value::Float, got {:?}", result);
        }
    }

    #[test]
    fn test_eval_arithmetic_modulo_by_zero_returns_null() {
        // Division/modulo by zero → Null (per standard SQL semantics for Float;
        // Integer parity with SQLite/MariaDB returns 0; matches the
        // existing `/` behavior so the fix stays symmetric).
        assert_eq!(
            eval_arithmetic(&Value::Integer(5), &Value::Integer(0), "%"),
            Value::Integer(0)
        );
        assert_eq!(
            eval_arithmetic(&Value::Float(5.0), &Value::Float(0.0), "%"),
            Value::Null
        );
    }

    #[test]
    fn test_eval_binary_op_modulo_dispatch() {
        // End-to-end: `eval_binary_op` routes `"%"` through `eval_arithmetic`.
        // This is the path that actually fires from `WHERE i % 2 <> 0`.
        assert_eq!(
            eval_binary_op(&Value::Integer(7), &Value::Integer(3), "%"),
            Value::Integer(1)
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
        // V312-59-D / Issue #4572: Float now coerces via `as i64` (truncate
        // toward zero, matches MySQL CAST AS SIGNED). Pre-#4572 returned 0.
        assert_eq!(to_i64(&Value::Float(3.14)), 3);
        // Non-numeric text returns 0 (MySQL legacy); numeric text parses.
        assert_eq!(to_i64(&Value::Text("hello".into())), 0);
        assert_eq!(to_i64(&Value::Text("42".into())), 42);
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

    // Regression for #4278: TPC-H Q16 has supplier comments like
    // "ans. ironicCustomer  requests cajole carefullyComplaintsy regular reque"
    // (note `ironicCustomer` has a back-to-back `cc` pair where the first `c`
    // ends "ironic" and the second starts "Customer"). A naive iterative
    // backtracking matcher can fail to re-try the second `c` after matching
    // the first `c` and failing to extend it to "ustomer". The pattern
    // `%Customer%Complaints%` should match.
    #[test]
    fn test_like_match_double_cc_q16() {
        // Back-to-back 'c' in text (ir**o**nicCust**o**mer).
        let text = "ans. ironicCustomer  requests cajole carefullyComplaintsy regular reque";
        assert!(sql_like_match(text, "%Customer%Complaints%"));
        assert!(sql_like_match(text, "%customer%complaints%"));
        // Single-token patterns still work.
        assert!(sql_like_match(text, "%Customer%"));
        assert!(sql_like_match(text, "%Complaints%"));
        // Suppress unused-var lint on rewritten comments above.
        let _ = text;
    }

    // Regression for #4278: edge case where the second wildcard consumes
    // the rest of the pattern.
    #[test]
    fn test_like_match_trailing_percent_after_substr() {
        assert!(sql_like_match("axb", "%a%b%"));
        assert!(sql_like_match("ab", "%a%b%"));
        assert!(sql_like_match("xxxabxxxcdxxx", "%ab%cd%"));
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
                auto_increment: false,
                ..Default::default()
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "val".to_string(),
                auto_increment: false,
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
        // V312-59-D / Issue #4572: text→numeric coercion now parses
        // numeric strings (MySQL 2/124). Pre-#4572 text returned 0/0.0.
        assert_eq!(to_f64(&Value::Text("42".into())), 42.0);
        assert_eq!(to_i64(&Value::Text("42".into())), 42);
        // Non-numeric text still returns 0 (MySQL legacy semantics).
        assert_eq!(to_f64(&Value::Text("abc".into())), 0.0);
        assert_eq!(to_i64(&Value::Text("abc".into())), 0);
        // Decimal text truncates toward zero in to_i64.
        assert_eq!(to_i64(&Value::Text("3.7".into())), 3);
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
