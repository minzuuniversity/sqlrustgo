use sqlrustgo_types::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum UnifiedExpr {
    Literal(Value),
    Column(String),
    BinaryOp { left: Box<UnifiedExpr>, op: String, right: Box<UnifiedExpr> },
    FunctionCall { name: String, args: Vec<UnifiedExpr> },
    UnaryOp { op: String, expr: Box<UnifiedExpr> },
    IsNull(Box<UnifiedExpr>),
    IsNotNull(Box<UnifiedExpr>),
    InList { expr: Box<UnifiedExpr>, list: Vec<UnifiedExpr> },
    Between { expr: Box<UnifiedExpr>, low: Box<UnifiedExpr>, high: Box<UnifiedExpr> },
    CaseWhen { whens: Vec<(UnifiedExpr, UnifiedExpr)>, else_val: Option<Box<UnifiedExpr>> },
    Cast { expr: Box<UnifiedExpr>, target_type: String },
}

impl UnifiedExpr {
    pub fn evaluate(&self, row: &[Value], columns: &[String]) -> Value {
        match self {
            UnifiedExpr::Literal(v) => v.clone(),
            UnifiedExpr::Column(name) => {
                columns.iter().position(|c| c == name)
                    .and_then(|i| row.get(i).cloned())
                    .unwrap_or(Value::Null)
            }
            UnifiedExpr::BinaryOp { left, op, right } => {
                let l = left.evaluate(row, columns);
                let r = right.evaluate(row, columns);
                eval_binary_op(&l, &r, op)
            }
            UnifiedExpr::UnaryOp { op, expr } => {
                let v = expr.evaluate(row, columns);
                eval_unary_op(&v, op)
            }
            UnifiedExpr::IsNull(expr) => Value::Boolean(matches!(expr.evaluate(row, columns), Value::Null)),
            UnifiedExpr::IsNotNull(expr) => Value::Boolean(!matches!(expr.evaluate(row, columns), Value::Null)),
            UnifiedExpr::FunctionCall { name, args } => {
                let vals: Vec<Value> = args.iter().map(|a| a.evaluate(row, columns)).collect();
                eval_fn(name, &vals)
            }
            UnifiedExpr::InList { expr, list } => {
                let val = expr.evaluate(row, columns);
                Value::Boolean(list.iter().any(|item| {
                    val == item.evaluate(row, columns) && !matches!(&val, Value::Null)
                }))
            }
            UnifiedExpr::Between { expr, low, high } => {
                let v = expr.evaluate(row, columns);
                let l = low.evaluate(row, columns);
                let h = high.evaluate(row, columns);
                Value::Boolean(eval_binary_op(&v, &l, ">=") == Value::Boolean(true)
                    && eval_binary_op(&v, &h, "<=") == Value::Boolean(true))
            }
            UnifiedExpr::CaseWhen { whens, else_val } => {
                for (cond, result) in whens {
                    if cond.evaluate(row, columns) == Value::Boolean(true) {
                        return result.evaluate(row, columns);
                    }
                }
                else_val.as_ref().map(|e| e.evaluate(row, columns)).unwrap_or(Value::Null)
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
        UnifiedExpr::BinaryOp { left, right, .. } => { collect_cols(left, acc); collect_cols(right, acc); }
        UnifiedExpr::UnaryOp { expr: e, .. } => collect_cols(e, acc),
        UnifiedExpr::IsNull(e) | UnifiedExpr::IsNotNull(e) => collect_cols(e, acc),
        UnifiedExpr::InList { expr: e, list } => { collect_cols(e, acc); for i in list { collect_cols(i, acc); } }
        UnifiedExpr::Between { expr: e, low, high } => { collect_cols(e, acc); collect_cols(low, acc); collect_cols(high, acc); }
        UnifiedExpr::CaseWhen { whens, else_val } => {
            for (c, r) in whens { collect_cols(c, acc); collect_cols(r, acc); }
            if let Some(e) = else_val { collect_cols(e, acc); }
        }
        UnifiedExpr::Cast { expr: e, .. } => collect_cols(e, acc),
        UnifiedExpr::FunctionCall { args, .. } => { for a in args { collect_cols(a, acc); } }
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
                op: op.clone(), right: Box::new(UnifiedExpr::from(right.as_ref())),
            },
            Expression::UnaryOp(op, e) => UnifiedExpr::UnaryOp {
                op: op.clone(), expr: Box::new(UnifiedExpr::from(e.as_ref())),
            },
            Expression::IsNull(e) => UnifiedExpr::IsNull(Box::new(UnifiedExpr::from(e.as_ref()))),
            Expression::IsNotNull(e) => UnifiedExpr::IsNotNull(Box::new(UnifiedExpr::from(e.as_ref()))),
            Expression::InList(expr, list) => UnifiedExpr::InList {
                expr: Box::new(UnifiedExpr::from(expr.as_ref())),
                list: list.iter().map(|e| UnifiedExpr::from(e)).collect(),
            },
            Expression::Between(expr, low, high) => UnifiedExpr::Between {
                expr: Box::new(UnifiedExpr::from(expr.as_ref())),
                low: Box::new(UnifiedExpr::from(low.as_ref())),
                high: Box::new(UnifiedExpr::from(high.as_ref())),
            },
            Expression::Like(expr, pattern, _) => UnifiedExpr::FunctionCall {
                name: "LIKE".into(),
                args: vec![UnifiedExpr::from(expr.as_ref()), UnifiedExpr::from(pattern.as_ref())],
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
    if s.eq_ignore_ascii_case("NULL") { return Value::Null; }
    if s.eq_ignore_ascii_case("TRUE") { return Value::Integer(1); }
    if s.eq_ignore_ascii_case("FALSE") { return Value::Integer(0); }
    let unquoted = if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        &s[1..s.len()-1]
    } else { s };
    if let Ok(i) = unquoted.parse::<i64>() { return Value::Integer(i); }
    if let Ok(f) = unquoted.parse::<f64>() { return Value::Integer(f as i64); }
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
        "/" => Value::Integer(left.as_integer().unwrap_or(0) / right.as_integer().unwrap_or(0).max(1)),
        _ => Value::Null,
    }
}

fn eval_unary_op(val: &Value, op: &str) -> Value {
    match op.to_uppercase().as_str() {
        "NOT" | "!" => Value::Boolean(!to_bool(val)),
        _ => Value::Null,
    }
}

fn eval_fn(name: &str, args: &[Value]) -> Value {
    match name.to_uppercase().as_str() {
        "LOWER" => args.first().map(|v| Value::Text(v.to_sql_string().to_lowercase())).unwrap_or(Value::Null),
        "UPPER" => args.first().map(|v| Value::Text(v.to_sql_string().to_uppercase())).unwrap_or(Value::Null),
        "LENGTH" | "LEN" => args.first().map(|v| Value::Integer(v.to_sql_string().len() as i64)).unwrap_or(Value::Null),
        "TRIM" => args.first().map(|v| Value::Text(v.to_sql_string().trim().to_string())).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn cast_val(val: &Value, target_type: &str) -> Value {
    match target_type.to_uppercase().as_str() {
        "INTEGER" | "INT" => {
            match val {
                Value::Integer(i) => Value::Integer(*i),
                Value::Text(s) => s.parse::<i64>().map(Value::Integer).unwrap_or(Value::Integer(0)),
                Value::Float(f) => Value::Integer(*f as i64),
                _ => Value::Integer(0),
            }
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

    #[test]
    fn test_literal_int() { assert_eq!(UnifiedExpr::Literal(Value::Integer(42)).evaluate(&[], &[]), Value::Integer(42)); }

    #[test]
    fn test_column() {
        let e = UnifiedExpr::Column("x".into());
        assert_eq!(e.evaluate(&[Value::Integer(5)], &["x".into()]), Value::Integer(5));
    }

    #[test]
    fn test_eq() {
        let e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Column("a".into())),
            op: "=".into(), right: Box::new(UnifiedExpr::Literal(Value::Integer(5))),
        };
        assert_eq!(e.evaluate(&[Value::Integer(5)], &["a".into()]), Value::Boolean(true));
        assert_eq!(e.evaluate(&[Value::Integer(3)], &["a".into()]), Value::Boolean(false));
    }

    #[test]
    fn test_and() {
        let e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
            op: "AND".into(), right: Box::new(UnifiedExpr::Literal(Value::Integer(0))),
        };
        assert_eq!(e.evaluate(&[], &[]), Value::Boolean(false));
    }

    #[test]
    fn test_or() {
        let e = UnifiedExpr::BinaryOp {
            left: Box::new(UnifiedExpr::Literal(Value::Integer(1))),
            op: "OR".into(), right: Box::new(UnifiedExpr::Literal(Value::Integer(0))),
        };
        assert_eq!(e.evaluate(&[], &[]), Value::Boolean(true));
    }

    #[test]
    fn test_is_null() {
        let e = UnifiedExpr::IsNull(Box::new(UnifiedExpr::Column("x".into())));
        assert_eq!(e.evaluate(&[Value::Null], &["x".into()]), Value::Boolean(true));
        assert_eq!(e.evaluate(&[Value::Boolean(true)], &["x".into()]), Value::Boolean(false));
    }

    #[test]
    fn test_in_list() {
        let e = UnifiedExpr::InList {
            expr: Box::new(UnifiedExpr::Column("x".into())),
            list: vec![UnifiedExpr::Literal(Value::Integer(1)), UnifiedExpr::Literal(Value::Integer(3))],
        };
        assert_eq!(e.evaluate(&[Value::Integer(3)], &["x".into()]), Value::Boolean(true));
        assert_eq!(e.evaluate(&[Value::Integer(2)], &["x".into()]), Value::Boolean(false));
    }

    #[test]
    fn test_between() {
        let e = UnifiedExpr::Between {
            expr: Box::new(UnifiedExpr::Column("age".into())),
            low: Box::new(UnifiedExpr::Literal(Value::Integer(18))),
            high: Box::new(UnifiedExpr::Literal(Value::Integer(65))),
        };
        assert_eq!(e.evaluate(&[Value::Integer(30)], &["age".into()]), Value::Boolean(true));
        assert_eq!(e.evaluate(&[Value::Integer(15)], &["age".into()]), Value::Boolean(false));
    }

    #[test]
    fn test_case_when() {
        let e = UnifiedExpr::CaseWhen {
            whens: vec![
                (UnifiedExpr::BinaryOp {
                    left: Box::new(UnifiedExpr::Column("s".into())),
                    op: ">=".into(), right: Box::new(UnifiedExpr::Literal(Value::Integer(90))),
                }, UnifiedExpr::Literal(Value::Text("A".into()))),
            ],
            else_val: Some(Box::new(UnifiedExpr::Literal(Value::Text("F".into())))),
        };
        assert_eq!(e.evaluate(&[Value::Integer(95)], &["s".into()]), Value::Text("A".into()));
        assert_eq!(e.evaluate(&[Value::Integer(50)], &["s".into()]), Value::Text("F".into()));
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
            op: "+".into(), right: Box::new(UnifiedExpr::Column("b".into())),
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
        assert_eq!(eval_binary_op(&Value::Integer(3), &Value::Integer(4), "+"), Value::Integer(7));
        assert_eq!(eval_binary_op(&Value::Integer(3), &Value::Integer(4), "*"), Value::Integer(12));
    }
}
