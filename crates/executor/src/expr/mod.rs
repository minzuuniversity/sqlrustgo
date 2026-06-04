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
}

impl UnifiedExpr {
    pub fn evaluate(&self, row: &[Value], columns: &[String]) -> Value {
        match self {
            UnifiedExpr::Literal(v) => v.clone(),
            UnifiedExpr::Column(name) => columns
                .iter()
                .position(|c| c == name)
                .and_then(|i| row.get(i).cloned())
                .unwrap_or(Value::Null),
            UnifiedExpr::BinaryOp { left, op, right } => {
                let l = left.evaluate(row, columns);
                let r = right.evaluate(row, columns);
                eval_binary_op(&l, &r, op)
            }
            UnifiedExpr::UnaryOp { op, expr } => {
                let v = expr.evaluate(row, columns);
                eval_unary_op(&v, op)
            }
            UnifiedExpr::IsNull(expr) => {
                Value::Boolean(matches!(expr.evaluate(row, columns), Value::Null))
            }
            UnifiedExpr::IsNotNull(expr) => {
                Value::Boolean(!matches!(expr.evaluate(row, columns), Value::Null))
            }
            UnifiedExpr::FunctionCall { name, args } => {
                let vals: Vec<Value> = args.iter().map(|a| a.evaluate(row, columns)).collect();
                eval_fn(name, &vals)
            }
            UnifiedExpr::InList { expr, list } => {
                let val = expr.evaluate(row, columns);
                Value::Boolean(
                    list.iter().any(|item| {
                        val == item.evaluate(row, columns) && !matches!(&val, Value::Null)
                    }),
                )
            }
            UnifiedExpr::Between { expr, low, high } => {
                let v = expr.evaluate(row, columns);
                let l = low.evaluate(row, columns);
                let h = high.evaluate(row, columns);
                Value::Boolean(
                    eval_binary_op(&v, &l, ">=") == Value::Boolean(true)
                        && eval_binary_op(&v, &h, "<=") == Value::Boolean(true),
                )
            }
            UnifiedExpr::CaseWhen { whens, else_val } => {
                for (cond, result) in whens {
                    if cond.evaluate(row, columns) == Value::Boolean(true) {
                        return result.evaluate(row, columns);
                    }
                }
                else_val
                    .as_ref()
                    .map(|e| e.evaluate(row, columns))
                    .unwrap_or(Value::Null)
            }
            UnifiedExpr::Cast { expr, target_type } => {
                let v = expr.evaluate(row, columns);
                cast_val(&v, target_type)
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
fn parse_lit(s: &str) -> Value {
    let s = s.trim();
    if s.eq_ignore_ascii_case("NULL") {
        return Value::Null;
    }
    if s.eq_ignore_ascii_case("TRUE") {
        return Value::Integer(1);
    }
    if s.eq_ignore_ascii_case("FALSE") {
        return Value::Integer(0);
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
    Value::Text(unquoted.to_string())
}

fn eval_binary_op(left: &Value, right: &Value, op: &str) -> Value {
    match op.to_uppercase().as_str() {
        "=" | "==" => Value::Boolean(left == right && !matches!(left, Value::Null)),
        "!=" | "<>" => Value::Boolean(left != right && !matches!(left, Value::Null)),
        ">" | "<" | ">=" | "<=" => compare_cmp(left, right, op),
        "AND" | "&&" => Value::Boolean(to_bool(left) && to_bool(right)),
        "OR" | "||" => Value::Boolean(to_bool(left) || to_bool(right)),
        "+" => Value::Integer(left.as_integer().unwrap_or(0) + right.as_integer().unwrap_or(0)),
        "-" => Value::Integer(left.as_integer().unwrap_or(0) - right.as_integer().unwrap_or(0)),
        "*" => Value::Integer(left.as_integer().unwrap_or(0) * right.as_integer().unwrap_or(0)),
        "/" => {
            Value::Integer(left.as_integer().unwrap_or(0) / right.as_integer().unwrap_or(0).max(1))
        }
        _ => Value::Null,
    }
}

fn eval_unary_op(val: &Value, op: &str) -> Value {
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
        "TRIM" => args
            .first()
            .map(|v| Value::Text(v.to_sql_string().trim().to_string()))
            .unwrap_or(Value::Null),
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
        "DATE_ADD" | "ADDDATE" => date_add_sub(&args, true),
        // DATE_SUB(date, INTERVAL n unit)
        "DATE_SUB" | "SUBDATE" => date_add_sub(&args, false),
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
        _ => Value::Null,
    }
}

/// DATE_ADD / DATE_SUB helper. Operates on text dates in YYYY-MM-DD form.
/// Accepts args in either order:
///   - [date_text, n, unit_text]
///   - [date_text, n] (default unit = DAY)
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
            let mut total_days = days_from_civil(y, m, d) + sign * n;
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
    let yoe = y.rem_euclid(400) as i64; // [0, 399]
    let m = if m > 2 { m - 3 } else { m + 9 }; // [0, 11]
    let doy = (153 * m + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097) as i64;
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

fn cast_val(val: &Value, target_type: &str) -> Value {
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
        let v = eval_fn(
            "IF",
            &[Value::Null, Value::Integer(1), Value::Integer(2)],
        );
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
            &[Value::Integer(0), Value::Text("yes".into()), Value::Text("no".into())],
        );
        assert_eq!(v, Value::Text("no".into()));
    }

    // ----- COALESCE -----
    #[test]
    fn test_coalesce_first_non_null() {
        let v = eval_fn(
            "COALESCE",
            &[Value::Null, Value::Null, Value::Integer(3), Value::Integer(4)],
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
            &[Value::Text("nope".into()), Value::Integer(1), Value::Text("DAY".into())],
        );
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn test_literal_int() {
        assert_eq!(
            UnifiedExpr::Literal(Value::Integer(42)).evaluate(&[], &[]),
            Value::Integer(42)
        );
    }

    #[test]
    fn test_column() {
        let e = UnifiedExpr::Column("x".into());
        assert_eq!(
            e.evaluate(&[Value::Integer(5)], &["x".into()]),
            Value::Integer(5)
        );
    }

    #[test]
    fn test_eq() {
        let e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Column("a".into())),
            op: "=".into(),
            right: Box::new(UnifiedExpr::Literal(Value::Integer(5))),
        };
        assert_eq!(
            e.evaluate(&[Value::Integer(5)], &["a".into()]),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(3)], &["a".into()]),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_and() {
        let e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
            op: "AND".into(),
            right: Box::new(UnifiedExpr::Literal(Value::Integer(0))),
        };
        assert_eq!(e.evaluate(&[], &[]), Value::Boolean(false));
    }

    #[test]
    fn test_or() {
        let e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
            op: "OR".into(),
            right: Box::new(UnifiedExpr::Literal(Value::Integer(0))),
        };
        assert_eq!(e.evaluate(&[], &[]), Value::Boolean(true));
    }

    #[test]
    fn test_is_null() {
        let e = UnifiedExpr::IsNull(Box::new(UnifiedExpr::Column("x".into())));
        assert_eq!(
            e.evaluate(&[Value::Null], &["x".into()]),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Boolean(true)], &["x".into()]),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_in_list() {
        let e = UnifiedExpr::InList {
            expr: Box::new(UnifiedExpr::Column("x".into())),
            list: vec![
                UnifiedExpr::Literal(Value::Integer(1)),
                UnifiedExpr::Literal(Value::Integer(3)),
            ],
        };
        assert_eq!(
            e.evaluate(&[Value::Integer(3)], &["x".into()]),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(2)], &["x".into()]),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_between() {
        let e = UnifiedExpr::Between {
            expr: Box::new(UnifiedExpr::Column("age".into())),
            low: Box::new(UnifiedExpr::Literal(Value::Integer(18))),
            high: Box::new(UnifiedExpr::Literal(Value::Integer(65))),
        };
        assert_eq!(
            e.evaluate(&[Value::Integer(30)], &["age".into()]),
            Value::Boolean(true)
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(15)], &["age".into()]),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_case_when() {
        let e = UnifiedExpr::CaseWhen {
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
            e.evaluate(&[Value::Integer(95)], &["s".into()]),
            Value::Text("A".into())
        );
        assert_eq!(
            e.evaluate(&[Value::Integer(50)], &["s".into()]),
            Value::Text("F".into())
        );
    }

    #[test]
    fn test_cast() {
        let e = UnifiedExpr::Cast {
            expr: Box::new(UnifiedExpr::Literal(Value::Text("42".into()))),
            target_type: "INTEGER".into(),
        };
        assert_eq!(e.evaluate(&[], &[]), Value::Integer(42));
    }

    #[test]
    fn test_referenced_columns() {
        let e = UnifiedExpr::BinaryOp {
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
        assert_eq!(parse_lit("TRUE"), Value::Integer(1));
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
}
