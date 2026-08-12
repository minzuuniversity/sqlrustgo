//! INT-3 Single Expression Engine — delegation contract tests.
//!
//! See `openspec/changes/p0-2-int3-convergence/` for the full spec.
//!
//! Each test in this file asserts the contract:
//!     `expr_utils::expression_to_value(&Expression::Foo(...))`
//!     ==
//!     `sqlrustgo_executor::expr::eval_foo(...)`  (where applicable)
//!
//! The test is a **contract test**, not a **delegation test**: it does not
//! check that `expr_utils` internally calls `executor::expr`. It checks
//! that the two paths return equal `Value`s. A future refactor is free to
//! move the delegation; the test still enforces the invariant.
//!
//! Per the OpenSpec change, this file will grow to 14 tests (one per
//! branch) as P0-2 §4.1–§4.13 are completed. This PR ships only the
//! `test_literal_delegation` (1/14); the other 13 are tracked in the
//! OpenSpec `tasks.md` and will be added in follow-up commits.

use sqlrustgo_executor::expr::eval_literal_from_str;
use sqlrustgo_parser::Expression;
use sqlrustgo_types::Value;

#[test]
fn test_literal_delegation() {
    // The test inputs mirror the legacy `expr_utils::expression_to_value`
    // Literal arm: NULL / i64 / f64 / quoted-string / unquoted-string /
    // whitespace. Each input goes through both the legacy facade and the
    // new single-source-of-truth function; the two results must be equal.

    let cases: &[(&str, &str)] = &[
        ("NULL", "null literal"),
        ("42", "i64 literal"),
        ("-7", "negative i64"),
        ("0.5", "f64 literal"),
        ("'hello'", "quoted string"),
        ("'a b c'", "quoted with spaces"),
        ("hello", "unquoted identifier-shaped"),
        ("  42  ", "whitespace-padded integer"),
    ];

    let mut failures: Vec<(&str, String, String)> = Vec::new();

    for (input, label) in cases {
        // Path 1: legacy facade (the one we are about to delegate away from)
        let from_facade =
            sqlrustgo::expr_utils::expression_to_value(&Expression::Literal(input.to_string()));
        // Path 2: the new single-source-of-truth function
        let from_evaluator = eval_literal_from_str(input);

        if from_facade != from_evaluator {
            failures.push((
                label,
                format!("{from_facade:?}"),
                format!("{from_evaluator:?}"),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 Literal delegation: expr_utils and executor::expr disagree on the following inputs:\n{}",
        failures
            .iter()
            .map(|(label, fac, ev)| format!("  - {label}: facade={fac} evaluator={ev}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_literal_known_outputs() {
    // Sanity: the new `eval_literal_from_str` matches the documented
    // behavior in the function's own doc-comment.
    assert_eq!(eval_literal_from_str("NULL"), Value::Null);
    assert_eq!(eval_literal_from_str("42"), Value::Integer(42));
    assert_eq!(eval_literal_from_str("-7"), Value::Integer(-7));
    assert_eq!(eval_literal_from_str("0.5"), Value::Float(0.5));
    assert_eq!(
        eval_literal_from_str("'hello'"),
        Value::Text("hello".into())
    );
    assert_eq!(
        eval_literal_from_str("'a b c'"),
        Value::Text("a b c".into())
    );
    assert_eq!(eval_literal_from_str("hello"), Value::Text("hello".into()));
    assert_eq!(eval_literal_from_str("  42  "), Value::Integer(42));
}

#[test]
fn test_isnull_delegation() {
    // Contract test for P0-2 §4.2 + §4.3 (IsNull / IsNotNull).
    // The two paths (legacy `expr_utils::evaluate_expression` and the
    // new `executor::expr::eval_is_null` / `eval_is_not_null`) must
    // return equal `Value`s for every input.

    use sqlrustgo_executor::expr::{eval_is_not_null, eval_is_null};
    use sqlrustgo_storage::TableInfo;

    let cases: &[(&str, Value, &str)] = &[
        ("NULL", Value::Null, "null literal"),
        ("42", Value::Integer(42), "i64 literal"),
        ("0.5", Value::Float(0.5), "f64 literal"),
        ("'hello'", Value::Text("hello".into()), "string literal"),
        ("''", Value::Text(String::new()), "empty string (not null)"),
    ];

    let mut failures: Vec<(String, String, String)> = Vec::new();

    for (input, expected_inner, label) in cases {
        // Inner expression: `Literal(input)`.
        let inner_expr = Expression::Literal(input.to_string());
        let table_info = TableInfo::default();

        // Path 1: legacy facade — build a single-row vec so the
        // evaluator can resolve the inner literal.
        let row = vec![Value::Null];
        let from_facade_isnull = sqlrustgo::expr_utils::evaluate_expression(
            &Expression::IsNull(Box::new(inner_expr.clone())),
            &row,
            &table_info,
        );
        let from_facade_isnotnull = sqlrustgo::expr_utils::evaluate_expression(
            &Expression::IsNotNull(Box::new(inner_expr.clone())),
            &row,
            &table_info,
        );

        // Path 2: new single-source-of-truth (no `Expression` parsing;
        // takes the inner `Value` directly).
        let from_evaluator_isnull = eval_is_null(expected_inner);
        let from_evaluator_isnotnull = eval_is_not_null(expected_inner);

        if from_facade_isnull != Ok(from_evaluator_isnull.clone()) {
            failures.push((
                format!("{} (IS NULL)", *label),
                format!("{from_facade_isnull:?}"),
                format!("{from_evaluator_isnull:?}"),
            ));
        }
        if from_facade_isnotnull != Ok(from_evaluator_isnotnull.clone()) {
            failures.push((
                format!("{} (IS NOT NULL)", *label),
                format!("{from_facade_isnotnull:?}"),
                format!("{from_evaluator_isnotnull:?}"),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 IsNull/IsNotNull delegation: facade and executor::expr disagree on the following inputs:\n{}",
        failures
            .iter()
            .map(|(label, fac, ev)| format!("  - {label}: facade={fac} evaluator={ev}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_isnull_known_outputs() {
    use sqlrustgo_executor::expr::{eval_is_not_null, eval_is_null};

    assert_eq!(eval_is_null(&Value::Null), Value::Boolean(true));
    assert_eq!(eval_is_null(&Value::Integer(0)), Value::Boolean(false));
    assert_eq!(eval_is_null(&Value::Float(0.5)), Value::Boolean(false));
    assert_eq!(
        eval_is_null(&Value::Text("".into())),
        Value::Boolean(false),
        "empty string is NOT null (P0-2 §4.2 invariant)"
    );
    assert_eq!(
        eval_is_null(&Value::Text("hello".into())),
        Value::Boolean(false)
    );
    assert_eq!(eval_is_null(&Value::Boolean(false)), Value::Boolean(false));

    assert_eq!(eval_is_not_null(&Value::Null), Value::Boolean(false));
    assert_eq!(eval_is_not_null(&Value::Integer(42)), Value::Boolean(true));
    assert_eq!(
        eval_is_not_null(&Value::Text("".into())),
        Value::Boolean(true)
    );
}

#[test]
fn test_aggregate_delegation() {
    // Contract test for P0-2 §4.4 (Aggregate).
    // The two paths (legacy `expr_utils::evaluate_expression` and the
    // new `executor::expr::eval_aggregate_lookup`) must return equal
    // `Value`s for every (row, column, agg_call) input.

    use sqlrustgo_executor::expr::eval_aggregate_lookup;
    use sqlrustgo_parser::{AggregateCall, AggregateFunction, Expression as ParserExpr};
    use sqlrustgo_storage::{ColumnDefinition, TableInfo};

    // Build a table_info with one column named "COUNT(*)" and one named
    // "SUM(l_quantity)" — the canonical forms produced by
    // `expression_to_string(Expression::Aggregate(Count{args:[]}))`
    // and `expression_to_string(Expression::Aggregate(Sum{[l_quantity]}))`.
    let table_info = TableInfo {
        name: "agg_test".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "COUNT(*)".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            collation: None,
            },
            ColumnDefinition {
                name: "SUM(l_quantity)".to_string(),
                data_type: "FLOAT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
            collation: None,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };

    // Pre-aggregated row (the SELECT/GROUP BY phase would have filled these).
    let row = vec![Value::Integer(42), Value::Float(1234.5)];

    // Case 1: COUNT(*)
    let agg_count = AggregateCall {
        func: AggregateFunction::Count,
        args: vec![],
        distinct: false,
    };
    let expr_count = ParserExpr::Aggregate(agg_count);

    // Path 1: legacy facade
    let from_facade_count =
        sqlrustgo::expr_utils::evaluate_expression(&expr_count, &row, &table_info);

    // Path 2: new single-source-of-truth
    let agg_name_count = sqlrustgo::expr_utils::expression_to_string(&expr_count);
    let from_evaluator_count = eval_aggregate_lookup(
        &agg_name_count,
        &row,
        &table_info
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
    );

    assert_eq!(
        from_facade_count,
        Ok(from_evaluator_count.expect("COUNT(*) should be in row")),
        "INT-3 Aggregate (COUNT(*)) delegation: facade and executor::expr disagree"
    );

    // Case 2: SUM(l_quantity)
    let agg_sum = AggregateCall {
        func: AggregateFunction::Sum,
        args: vec![ParserExpr::Identifier("l_quantity".to_string())],
        distinct: false,
    };
    let expr_sum = ParserExpr::Aggregate(agg_sum);

    let from_facade_sum = sqlrustgo::expr_utils::evaluate_expression(&expr_sum, &row, &table_info);
    let agg_name_sum = sqlrustgo::expr_utils::expression_to_string(&expr_sum);
    let from_evaluator_sum = eval_aggregate_lookup(
        &agg_name_sum,
        &row,
        &table_info
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
    );

    assert_eq!(
        from_facade_sum,
        Ok(from_evaluator_sum.expect("SUM(l_quantity) should be in row")),
        "INT-3 Aggregate (SUM(l_quantity)) delegation: facade and executor::expr disagree"
    );

    // Case 3: missing aggregate (no column named "AVG(price)")
    let agg_avg = AggregateCall {
        func: AggregateFunction::Avg,
        args: vec![ParserExpr::Identifier("price".to_string())],
        distinct: false,
    };
    let expr_avg = ParserExpr::Aggregate(agg_avg);
    let from_facade_avg = sqlrustgo::expr_utils::evaluate_expression(&expr_avg, &row, &table_info);
    let agg_name_avg = sqlrustgo::expr_utils::expression_to_string(&expr_avg);
    let from_evaluator_avg = eval_aggregate_lookup(
        &agg_name_avg,
        &row,
        &table_info
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
    );

    // Both should be None / Err with the same "Aggregate not found" message
    assert!(
        from_facade_avg.is_err(),
        "facade should return Err for missing aggregate"
    );
    assert!(
        from_evaluator_avg.is_none(),
        "evaluator should return None for missing aggregate"
    );
    let err_msg = from_facade_avg.unwrap_err();
    assert!(
        err_msg.contains(&agg_name_avg),
        "facade error should mention the missing aggregate name, got: {}",
        err_msg
    );
}

#[test]
fn test_aggregate_known_outputs() {
    use sqlrustgo_executor::expr::eval_aggregate_lookup;

    let column_names = vec!["COUNT(*)".to_string(), "SUM(x)".to_string()];
    let row = vec![Value::Integer(7), Value::Float(2.5)];

    assert_eq!(
        eval_aggregate_lookup("COUNT(*)", &row, &column_names),
        Some(Value::Integer(7))
    );
    assert_eq!(
        eval_aggregate_lookup("sum(x)", &row, &column_names),
        Some(Value::Float(2.5)),
        "case-insensitive match"
    );
    assert_eq!(
        eval_aggregate_lookup("MIN(z)", &row, &column_names),
        None,
        "missing aggregate returns None"
    );
    assert_eq!(
        eval_aggregate_lookup("COUNT(*)", &[], &column_names),
        None,
        "empty row returns None (out-of-bounds lookup)"
    );
}

#[test]
fn test_like_delegation() {
    // Contract test for P0-2 §4.5 + §4.6 (Like / NotLike).
    // The two paths (legacy `expr_utils::evaluate_expression` and the
    // new `executor::expr::sql_like_match`) must return equal `Value`s
    // for every (text, pattern) input.
    //
    // Note: Like arms internally call `sql_like_match` (now in
    // `executor::expr`). We test that the legacy shim in `expr_utils`
    // and the real implementation in `executor::expr` agree.
    use sqlrustgo_executor::expr::sql_like_match as executor_sql_like_match;
    use sqlrustgo_parser::Expression as ParserExpr;
    use sqlrustgo_storage::TableInfo;

    let table_info = TableInfo::default();
    let empty_row: Vec<Value> = vec![];

    // Pairs of (text, pattern) with expected match result.
    let cases: &[(&str, &str, bool)] = &[
        // Basic wildcards
        ("hello", "%ell%", true),
        ("hello", "world", false),
        ("hello", "hello", true), // exact match
        ("hello", "HELLO", true), // case-insensitive
        ("hello", "hell%", true), // trailing wildcard
        ("hello", "%ello", true), // leading wildcard
        ("hello", "h_llo", true), // single-char wildcard
        ("hello", "h_lo", false), // _ matches exactly 1 char
        ("hello", "%", true),     // % matches anything
        ("", "%", true),          // % matches empty
        ("", "", true),           // empty matches empty
        ("abc", "", false),       // non-empty ≠ empty
        // Quoted pattern (the literal parser adds quotes; the matcher strips them)
        ("hello", "'%ell%'", true),
        ("hello", "'world'", false),
        // TPC-H-style
        ("green tea kettle", "%green%", true),
        ("apple", "%green%", false),
    ];

    let mut failures: Vec<(String, String)> = Vec::new();

    for (text, pattern, expected) in cases {
        // Path 1: legacy facade — build a single-row vec and use
        // `evaluate_expression` with `Expression::Like`.
        // We put the text and pattern in the row as Identifiers
        // (or use Literal as inner for simplicity).
        // Simpler: build `Expression::Like(Literal(text), Literal(pattern), None)`
        // but that requires a `row` and `table_info` for the inner
        // Literal evaluation. Use empty row and default table_info.
        let expr_like = ParserExpr::Like(
            Box::new(ParserExpr::Literal(text.to_string())),
            Box::new(ParserExpr::Literal(pattern.to_string())),
            None,
        );
        let from_facade =
            sqlrustgo::expr_utils::evaluate_expression(&expr_like, &empty_row, &table_info);

        // Path 2: the new single-source-of-truth
        let from_evaluator = executor_sql_like_match(text, pattern);

        let expected_val = Value::Boolean(*expected);
        let facade_val = from_facade.clone().unwrap_or(Value::Null);
        if facade_val != expected_val || from_evaluator != *expected {
            failures.push((
                format!("text={text:?} pattern={pattern:?}"),
                format!(
                    "facade={facade_val:?} evaluator={from_evaluator} expected={expected_val:?}"
                ),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 Like delegation: facade and executor::expr disagree on the following inputs:\n{}",
        failures
            .iter()
            .map(|(label, diff)| format!("  - {label}: {diff}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_like_known_outputs() {
    use sqlrustgo_executor::expr::sql_like_match;

    // TPC-H Q9 pattern: `WHERE p_name LIKE '%green%'`
    assert!(sql_like_match("green tea kettle", "%green%"));
    assert!(sql_like_match("GREEN TEA KETTLE", "%green%")); // case-insensitive
    assert!(!sql_like_match("apple", "%green%"));

    // Single-char wildcard
    assert!(sql_like_match("hello", "h_llo"));
    assert!(!sql_like_match("hello", "h_lo"));
    assert!(!sql_like_match("hello", "hello_"));

    // Edge cases
    assert!(sql_like_match("", "%"));
    assert!(sql_like_match("anything", "%"));
    assert!(!sql_like_match("anything", ""));
    assert!(sql_like_match("", ""));

    // Quoted pattern (literal parser adds quotes)
    assert!(sql_like_match("hello", "'%ell%'"));
    assert!(!sql_like_match("hello", "'world'"));
}

#[test]
fn test_notlike_delegation() {
    // Contract test for P0-2 §4.6 (NotLike).
    use sqlrustgo_executor::expr::sql_like_match;
    use sqlrustgo_parser::Expression as ParserExpr;
    use sqlrustgo_storage::TableInfo;

    let table_info = TableInfo::default();
    let empty_row: Vec<Value> = vec![];

    let cases: &[(&str, &str)] = &[
        ("hello", "world"),
        ("hello", "hell"), // not a wildcard
        ("apple", "%green%"),
    ];

    for (text, pattern) in cases {
        let expr_notlike = ParserExpr::NotLike(
            Box::new(ParserExpr::Literal(text.to_string())),
            Box::new(ParserExpr::Literal(pattern.to_string())),
            None,
        );
        let from_facade =
            sqlrustgo::expr_utils::evaluate_expression(&expr_notlike, &empty_row, &table_info);
        let expected = Value::Boolean(!sql_like_match(text, pattern));
        assert_eq!(
            from_facade,
            Ok(expected.clone()),
            "INT-3 NotLike delegation: facade={from_facade:?} executor::expr={expected:?} for text={text:?} pattern={pattern:?}"
        );
    }
}

#[test]
fn test_between_delegation() {
    // Contract test for P0-2 §4.7 (Between).
    use sqlrustgo_executor::expr::{compare_values, eval_between};
    use sqlrustgo_parser::Expression as ParserExpr;
    use sqlrustgo_storage::TableInfo;

    let table_info = TableInfo::default();
    let empty_row: Vec<Value> = vec![];

    let cases: &[(Value, Value, Value, bool)] = &[
        // value BETWEEN low AND high (inclusive both ends)
        (
            Value::Integer(5),
            Value::Integer(1),
            Value::Integer(10),
            true,
        ),
        (
            Value::Integer(0),
            Value::Integer(1),
            Value::Integer(10),
            false,
        ),
        (
            Value::Integer(1),
            Value::Integer(1),
            Value::Integer(10),
            true,
        ), // inclusive low
        (
            Value::Integer(10),
            Value::Integer(1),
            Value::Integer(10),
            true,
        ), // inclusive high
        (
            Value::Integer(11),
            Value::Integer(1),
            Value::Integer(10),
            false,
        ),
        // Text lexicographic (TPC-H Q12: l_shipdate BETWEEN '1995-01-01' AND '1996-12-31')
        (
            Value::Text("1995-06-15".into()),
            Value::Text("1995-01-01".into()),
            Value::Text("1996-12-31".into()),
            true,
        ),
        (
            Value::Text("1994-12-31".into()),
            Value::Text("1995-01-01".into()),
            Value::Text("1996-12-31".into()),
            false,
        ),
        // NULL semantics: NULL sorts before any non-NULL
        (Value::Null, Value::Integer(1), Value::Integer(10), false),
        (Value::Integer(5), Value::Null, Value::Integer(10), true), // low=Null, 5 > Null → true
        (Value::Integer(5), Value::Integer(1), Value::Null, false), // high=Null, 5 < Null → false
    ];

    let mut failures: Vec<String> = Vec::new();

    for (value, low, high, _expected) in cases {
        // Path 1: legacy facade — build `Expression::Between(Literal(value), Literal(low), Literal(high))`.
        let expr_between = ParserExpr::Between(
            Box::new(ParserExpr::Literal(format_value_for_literal(value.clone()))),
            Box::new(ParserExpr::Literal(format_value_for_literal(low.clone()))),
            Box::new(ParserExpr::Literal(format_value_for_literal(high.clone()))),
        );
        let from_facade =
            sqlrustgo::expr_utils::evaluate_expression(&expr_between, &empty_row, &table_info);

        // Path 2: new single-source-of-truth
        let from_evaluator = eval_between(value, low, high);

        // Sanity: also verify compare_values is consistent
        let cv_low = compare_values(value, low);
        let cv_high = compare_values(value, high);
        let from_evaluator_via_cv = Value::Boolean(cv_low >= 0 && cv_high <= 0);

        if from_facade != Ok(from_evaluator.clone()) {
            failures.push(format!(
                "value={value:?} low={low:?} high={high:?}: facade={from_facade:?} evaluator={from_evaluator:?}"
            ));
        }
        if from_evaluator != from_evaluator_via_cv {
            failures.push(format!(
                "internal inconsistency: eval_between={from_evaluator:?} but compare_values logic says {from_evaluator_via_cv:?}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 Between delegation: facade and executor::expr disagree on the following inputs:\n{}",
        failures.join("\n")
    );
}

/// Helper to convert a `Value` into the literal string form expected
/// by `Expression::Literal`. The legacy `expression_to_value` parses
/// these strings back to `Value`s, so we use the same convention for
/// the test inputs.
fn format_value_for_literal(v: Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(s) => format!("'{}'", s),
        Value::Boolean(b) => if b { "TRUE" } else { "FALSE" }.to_string(),
        // For non-exhaustive enum, fall back to Debug.
        other => format!("{:?}", other),
    }
}

#[test]
fn test_between_known_outputs() {
    use sqlrustgo_executor::expr::{eval_between, eval_not_between};

    // Basic integer range
    assert_eq!(
        eval_between(&Value::Integer(5), &Value::Integer(1), &Value::Integer(10)),
        Value::Boolean(true)
    );
    assert_eq!(
        eval_between(&Value::Integer(0), &Value::Integer(1), &Value::Integer(10)),
        Value::Boolean(false)
    );

    // Inclusive boundaries
    assert_eq!(
        eval_between(&Value::Integer(1), &Value::Integer(1), &Value::Integer(10)),
        Value::Boolean(true),
        "low boundary is inclusive"
    );
    assert_eq!(
        eval_between(&Value::Integer(10), &Value::Integer(1), &Value::Integer(10)),
        Value::Boolean(true),
        "high boundary is inclusive"
    );

    // TPC-H Q1: l_quantity BETWEEN 1 AND 50
    assert_eq!(
        eval_between(&Value::Integer(25), &Value::Integer(1), &Value::Integer(50)),
        Value::Boolean(true)
    );
    assert_eq!(
        eval_between(&Value::Integer(0), &Value::Integer(1), &Value::Integer(50)),
        Value::Boolean(false)
    );

    // NULL semantics
    assert_eq!(
        eval_between(&Value::Null, &Value::Integer(1), &Value::Integer(10)),
        Value::Boolean(false),
        "NULL sorts before any non-NULL, so NULL < low"
    );

    // NotBetween
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
fn test_compare_values_known_outputs() {
    use sqlrustgo_executor::expr::compare_values;

    // Integer comparison
    assert_eq!(compare_values(&Value::Integer(1), &Value::Integer(1)), 0);
    assert_eq!(compare_values(&Value::Integer(1), &Value::Integer(2)), -1);
    assert_eq!(compare_values(&Value::Integer(2), &Value::Integer(1)), 1);

    // Float comparison
    assert_eq!(compare_values(&Value::Float(1.0), &Value::Float(1.0)), 0);
    assert_eq!(compare_values(&Value::Float(1.0), &Value::Float(2.0)), -1);

    // Text lexicographic
    assert_eq!(
        compare_values(&Value::Text("a".into()), &Value::Text("b".into())),
        -1
    );

    // NULL semantics
    assert_eq!(compare_values(&Value::Null, &Value::Null), 0);
    assert_eq!(compare_values(&Value::Null, &Value::Integer(1)), -1);
    assert_eq!(compare_values(&Value::Integer(1), &Value::Null), 1);

    // Mixed types: treated as equal
    assert_eq!(compare_values(&Value::Integer(1), &Value::Float(1.0)), 0);
}

#[test]
fn test_case_when_delegation() {
    // Contract test for P0-2 §4.9 (CaseWhen).
    use sqlrustgo_executor::expr::eval_case_when;
    use sqlrustgo_parser::parser::WhenClause;
    use sqlrustgo_parser::Expression as ParserExpr;
    use sqlrustgo_storage::TableInfo;

    let table_info = TableInfo::default();
    let empty_row: Vec<Value> = vec![];

    // TPC-H Q12-style:
    //   CASE WHEN l_shipmode IN ('MAIL', 'SHIP') THEN l_receiptdate ELSE l_commitdate END
    // Simplified: a list of when-then-else clauses.

    // Helper: build a Literal-clause (condition -> result)
    fn lit(s: &str) -> ParserExpr {
        ParserExpr::Literal(s.to_string())
    }
    fn wc(cond: &str, result: &str) -> WhenClause {
        WhenClause {
            condition: lit(cond),
            result: lit(result),
        }
    }
    fn int_lit(n: i64) -> ParserExpr {
        ParserExpr::Literal(n.to_string())
    }
    fn int_wc(cond: i64, result: i64) -> WhenClause {
        wc(&cond.to_string(), &result.to_string())
    }

    // Cases: (whens, else_val, expected)
    let cases: Vec<(Vec<WhenClause>, Option<ParserExpr>, Value)> = vec![
        // 1. TPC-H-style: WHEN 1=1 THEN 'A' WHEN 2=2 THEN 'B' ELSE 'C' END
        //    The condition is evaluated via evaluate_fn. We use literal
        //    conditions that the facade's `evaluate_expression` will
        //    parse via `eval_literal_from_str`.
        (
            vec![wc("1", "'A'"), wc("2", "'B'")],
            Some(lit("'C'")),
            Value::Text("A".into()),
        ),
        // 2. First WHEN matches via non-Boolean truthy (integer 1)
        //    The condition `1` evaluates to Integer(1) which is
        //    not Null and not Boolean(false), so it's truthy per
        //    the SQL CASE extension.
        (
            vec![int_wc(1, 100), int_wc(2, 200)],
            Some(int_lit(999)),
            Value::Integer(100),
        ),
        // 3. No WHEN matches, ELSE is taken
        //    Per the legacy `expr_utils` rule: any non-Null, non-Boolean(false)
        //    value is truthy. Integer(0) IS truthy by this rule (only
        //    `Value::Boolean(false)` is falsy; `Value::Null` is also
        //    "not truthy", see below). So this case actually MATCHES at
        //    the first WHEN.
        (
            vec![int_wc(0, 100)],
            Some(int_lit(999)),
            Value::Integer(100),
        ),
        // 4. No WHEN matches (using a NULL condition), no ELSE -> Null
        (vec![wc("NULL", "100")], None, Value::Null),
        // 5. Empty whens, ELSE returned
        (vec![], Some(lit("'X'")), Value::Text("X".into())),
        // 6. Empty whens, no ELSE -> Null
        (vec![], None, Value::Null),
    ];

    let mut failures: Vec<String> = Vec::new();

    for (i, (whens, else_val, expected)) in cases.iter().enumerate() {
        // Build `Expression::CaseWhen(whens, else_val)` for the facade.
        let expr_cw = ParserExpr::CaseWhen(whens.clone(), else_val.clone().map(Box::new));
        let from_facade =
            sqlrustgo::expr_utils::evaluate_expression(&expr_cw, &empty_row, &table_info);

        // Path 2: new single-source-of-truth
        let from_evaluator = eval_case_when(whens, else_val.as_ref(), |e| {
            sqlrustgo::expr_utils::evaluate_expression(e, &empty_row, &table_info)
        });

        if from_facade != Ok(expected.clone()) || from_evaluator != Ok(expected.clone()) {
            failures.push(format!(
                "case {i}: whens={whens:?} else={else_val:?} expected={expected:?} facade={from_facade:?} evaluator={from_evaluator:?}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 CaseWhen delegation: facade and/or executor::expr disagree on the following inputs:\n{}",
        failures.join("\n")
    );
}

#[test]
fn test_case_when_known_outputs() {
    use sqlrustgo_executor::expr::eval_case_when;
    use sqlrustgo_parser::parser::WhenClause;
    use sqlrustgo_parser::Expression as ParserExpr;

    let empty_row: Vec<Value> = vec![];
    let table_info = sqlrustgo_storage::TableInfo::default();

    fn lit(s: &str) -> ParserExpr {
        ParserExpr::Literal(s.to_string())
    }
    fn wc(cond: &str, result: &str) -> WhenClause {
        WhenClause {
            condition: lit(cond),
            result: lit(result),
        }
    }

    // 1. Single matching WHEN (Boolean(true) condition)
    let whens = vec![wc("1", "'A'")];
    let result = eval_case_when(&whens, Some(&lit("'B'")), |e| {
        sqlrustgo::expr_utils::evaluate_expression(e, &empty_row, &table_info)
    });
    assert_eq!(result, Ok(Value::Text("A".into())));

    // 2. Non-matching WHEN (Boolean(false) condition) → falls through
    let whens = vec![wc("NULL", "'A'")]; // NULL is not Boolean(true) and not truthy
    let result = eval_case_when(&whens, Some(&lit("'B'")), |e| {
        sqlrustgo::expr_utils::evaluate_expression(e, &empty_row, &table_info)
    });
    assert_eq!(result, Ok(Value::Text("B".into())));

    // 3. Multiple WHENs, second matches
    let whens = vec![wc("NULL", "'A'"), wc("5", "'B'")];
    let result = eval_case_when(&whens, Some(&lit("'C'")), |e| {
        sqlrustgo::expr_utils::evaluate_expression(e, &empty_row, &table_info)
    });
    assert_eq!(result, Ok(Value::Text("B".into())));

    // 4. Non-Boolean truthy: integer 1 → matches
    let whens = vec![WhenClause {
        condition: lit("1"),
        result: lit("100"),
    }];
    let result = eval_case_when(&whens, Some(&lit("999")), |e| {
        sqlrustgo::expr_utils::evaluate_expression(e, &empty_row, &table_info)
    });
    assert_eq!(
        result,
        Ok(Value::Integer(100)),
        "integer 1 is truthy (non-null, non-Boolean(false)) per SQL CASE extension"
    );

    // 5. No ELSE, no match → Null
    let whens = vec![wc("NULL", "'A'")];
    let result = eval_case_when(&whens, None, |e| {
        sqlrustgo::expr_utils::evaluate_expression(e, &empty_row, &table_info)
    });
    assert_eq!(result, Ok(Value::Null));
}

#[test]
fn test_identifier_delegation() {
    // Contract test for P0-2 §4.10 (Identifier).
    use sqlrustgo_executor::expr::eval_identifier;
    use sqlrustgo_parser::Expression as ParserExpr;
    use sqlrustgo_storage::{ColumnDefinition, TableInfo};

    // Table with 3 columns: simple, qualified, multi-join-accumulated
    let columns = vec![
        ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
        collation: None,
        },
        ColumnDefinition {
            name: "t.name".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        collation: None,
        },
        ColumnDefinition {
            name: "a_join_b.a.tag".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        collation: None,
        },
    ];
    let table_info = TableInfo {
        name: "test".to_string(),
        columns: columns.clone(),
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };
    let row: Vec<Value> = vec![
        Value::Integer(42),
        Value::Text("Alice".into()),
        Value::Text("tag_value".into()),
    ];

    // Cases: (input, expected)
    let cases: &[(&str, Value)] = &[
        // Simple column
        ("id", Value::Integer(42)),
        // Qualified: `t.name` → matches column `t.name` directly
        ("t.name", Value::Text("Alice".into())),
        // Multi-join: `a.tag` → trailing 2 segments match `a_join_b.a.tag`
        ("a.tag", Value::Text("tag_value".into())),
        // Bare `tag` (no qualifier) → trailing-segment match
        ("tag", Value::Text("tag_value".into())),
        // Unknown column → fallback Value::Text(name)
        ("unknown_col", Value::Text("unknown_col".into())),
    ];

    let mut failures: Vec<String> = Vec::new();

    for (input, expected) in cases {
        // Path 1: legacy facade via `evaluate_expression`
        let expr_id = ParserExpr::Identifier(input.to_string());
        let from_facade = sqlrustgo::expr_utils::evaluate_expression(&expr_id, &row, &table_info);

        // Path 2: new single-source-of-truth
        let from_evaluator = eval_identifier(input, &row, &columns);

        if from_facade != Ok(expected.clone()) {
            failures.push(format!(
                "facade mismatch for {input:?}: facade={from_facade:?} expected={expected:?}"
            ));
        }
        if from_evaluator != *expected {
            failures.push(format!(
                "evaluator mismatch for {input:?}: evaluator={from_evaluator:?} expected={expected:?}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 Identifier delegation: facade and/or executor::expr disagree:\n{}",
        failures.join("\n")
    );
}

#[test]
fn test_identifier_known_outputs() {
    use sqlrustgo_executor::expr::eval_identifier;
    use sqlrustgo_storage::ColumnDefinition;

    let columns = vec![
        ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
        collation: None,
        },
        ColumnDefinition {
            name: "user_name".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        collation: None,
        },
    ];
    let row: Vec<Value> = vec![Value::Integer(7), Value::Text("bob".into())];

    // Found
    assert_eq!(eval_identifier("id", &row, &columns), Value::Integer(7));
    assert_eq!(
        eval_identifier("user_name", &row, &columns),
        Value::Text("bob".into())
    );
    // Case-insensitive
    assert_eq!(
        eval_identifier("ID", &row, &columns),
        Value::Integer(7),
        "case-insensitive match"
    );
    assert_eq!(
        eval_identifier("USER_NAME", &row, &columns),
        Value::Text("bob".into()),
        "case-insensitive match"
    );
    // Not found → fallback
    assert_eq!(
        eval_identifier("missing", &row, &columns),
        Value::Text("missing".into()),
        "unknown column returns Value::Text(name)"
    );
    // Out of bounds
    assert_eq!(
        eval_identifier("id", &[], &columns),
        Value::Null,
        "out-of-bounds row returns Null"
    );
}

#[test]
fn test_find_column_index_known_outputs() {
    use sqlrustgo_executor::expr::find_column_index;
    use sqlrustgo_storage::ColumnDefinition;

    let columns = vec![
        ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
        collation: None,
        },
        ColumnDefinition {
            name: "t.col".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        collation: None,
        },
        ColumnDefinition {
            name: "a_join_b.a.deep".to_string(),
            data_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
        collation: None,
        },
    ];

    // Exact match
    assert_eq!(find_column_index("id", &columns), Some(0));
    // Case-insensitive
    assert_eq!(find_column_index("ID", &columns), Some(0));
    // Qualified: exact match
    assert_eq!(find_column_index("t.col", &columns), Some(1));
    // Multi-join trailing-segment match
    assert_eq!(
        find_column_index("a.deep", &columns),
        Some(2),
        "trailing 2 segments match"
    );
    // Unqualified trailing-segment match
    assert_eq!(
        find_column_index("deep", &columns),
        Some(2),
        "bare name matches trailing segment"
    );
    // Not found (qualifier doesn't match any column)
    assert_eq!(find_column_index("nope", &columns), None);
    assert_eq!(find_column_index("a.nope", &columns), None);
    // Not found (qualified name has no matching column)
    assert_eq!(find_column_index("x.col", &columns), None);
}

#[test]
fn test_function_call_delegation() {
    // Contract test for P0-2 §4.11 (FunctionCall).
    // The generic `Expression::FunctionCall` arm in evaluate_expression
    // delegates to executor::expr::eval_fn (the single source of truth
    // for the function table). The previous EXTRACT special arm has
    // been removed in this commit because it duplicated the EXTRACT
    // handling in eval_fn.
    //
    // We test 5 representative function calls and assert that
    // facade and executor agree.
    use sqlrustgo_executor::expr::eval_fn;
    use sqlrustgo_parser::Expression as ParserExpr;
    use sqlrustgo_storage::TableInfo;

    let table_info = TableInfo::default();
    let empty_row: Vec<Value> = vec![];

    // Helper: build a list-of-literals for the function-call args
    fn arg_lit(s: &str) -> ParserExpr {
        ParserExpr::Literal(s.to_string())
    }
    fn arg_int(n: i64) -> ParserExpr {
        ParserExpr::Literal(n.to_string())
    }

    // Cases: (function name, args, expected value)
    // Each case is tested via BOTH the legacy facade (using
    // evaluate_expression + Expression::FunctionCall) and the
    // new direct path (eval_fn).
    let cases: &[(&str, Vec<ParserExpr>, Value)] = &[
        // LOWER
        (
            "LOWER",
            vec![arg_lit("'HELLO'")],
            Value::Text("hello".into()),
        ),
        // UPPER
        (
            "UPPER",
            vec![arg_lit("'hello'")],
            Value::Text("HELLO".into()),
        ),
        // LENGTH (returns Integer)
        ("LENGTH", vec![arg_lit("'hello'")], Value::Integer(5)),
        // EXTRACT YEAR
        (
            "EXTRACT",
            vec![arg_lit("'YEAR'"), arg_lit("'2024-06-05'")],
            Value::Text("2024".into()),
        ),
        // EXTRACT MONTH
        (
            "EXTRACT",
            vec![arg_lit("'MONTH'"), arg_lit("'2024-06-05'")],
            Value::Text("06".into()),
        ),
        // EXTRACT DAY
        (
            "EXTRACT",
            vec![arg_lit("'DAY'"), arg_lit("'2024-06-05'")],
            Value::Text("05".into()),
        ),
        // Unknown function
        ("UNKNOWN_FN", vec![arg_int(1)], Value::Null),
    ];

    let mut failures: Vec<String> = Vec::new();

    for (name, args, _expected) in cases {
        // Build `Expression::FunctionCall(name, args)` for the facade.
        let expr_fc = ParserExpr::FunctionCall(name.to_string(), args.clone());

        // Path 1: legacy facade via evaluate_expression
        let from_facade =
            sqlrustgo::expr_utils::evaluate_expression(&expr_fc, &empty_row, &table_info);

        // Path 2: facade's argument evaluation, then executor::eval_fn
        // (this is what the unified path actually does internally)
        let evaluated_args: Vec<Value> = args
            .iter()
            .map(|a| {
                sqlrustgo::expr_utils::evaluate_expression(a, &empty_row, &table_info)
                    .unwrap_or(Value::Null)
            })
            .collect();
        let from_evaluator = eval_fn(name, &evaluated_args);

        // Both paths must agree
        if from_facade != Ok(from_evaluator.clone()) {
            failures.push(format!(
                "fn {name}: facade={from_facade:?} evaluator={from_evaluator:?} (disagreement on 1/15 branch)"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 FunctionCall delegation: facade and executor::expr disagree on the following inputs:\n{}",
        failures.join("\n")
    );
}

#[test]
fn test_function_call_known_outputs() {
    use sqlrustgo_executor::expr::eval_fn;

    // Sanity tests for `eval_fn` directly, covering the 4 most common
    // MySQL 5.7 function-dispatch paths.

    // LOWER
    assert_eq!(
        eval_fn("LOWER", &[Value::Text("HELLO".into())]),
        Value::Text("hello".into())
    );
    // UPPER
    assert_eq!(
        eval_fn("UPPER", &[Value::Text("hello".into())]),
        Value::Text("HELLO".into())
    );
    // LENGTH
    assert_eq!(
        eval_fn("LENGTH", &[Value::Text("hello".into())]),
        Value::Integer(5)
    );
    // EXTRACT YEAR/MONTH/DAY
    assert_eq!(
        eval_fn(
            "EXTRACT",
            &[Value::Text("YEAR".into()), Value::Text("2024-06-05".into()),]
        ),
        Value::Text("2024".into())
    );
    assert_eq!(
        eval_fn(
            "EXTRACT",
            &[
                Value::Text("MONTH".into()),
                Value::Text("2024-06-05".into()),
            ]
        ),
        Value::Text("06".into())
    );
    assert_eq!(
        eval_fn(
            "EXTRACT",
            &[Value::Text("DAY".into()), Value::Text("2024-06-05".into()),]
        ),
        Value::Text("05".into())
    );
    // Unknown function
    assert_eq!(eval_fn("FOO_BAR", &[Value::Integer(1)]), Value::Null);
    // Empty args
    assert_eq!(eval_fn("LOWER", &[]), Value::Null);
    // EXTRACT with malformed date
    assert_eq!(
        eval_fn(
            "EXTRACT",
            &[Value::Text("YEAR".into()), Value::Text("202".into())]
        ),
        Value::Null,
        "source too short (3 chars) for YEAR slice (needs >=4)"
    );
    assert_eq!(
        eval_fn(
            "EXTRACT",
            &[
                Value::Text("INVALID_FIELD".into()),
                Value::Text("2024-06-05".into())
            ]
        ),
        Value::Null,
        "unknown field returns Null"
    );
}

#[test]
fn test_unary_op_delegation() {
    // Contract test for P0-2 §4.12 (UnaryOp).
    use sqlrustgo_executor::expr::eval_unary_op;
    use sqlrustgo_parser::Expression as ParserExpr;
    use sqlrustgo_storage::TableInfo;

    let table_info = TableInfo::default();
    let empty_row: Vec<Value> = vec![];

    // Cases: (input_value, op, expected)
    // We test the underlying `eval_unary_op` directly and via the
    // facade through `Expression::UnaryOp`.
    let cases: &[(Value, &str, Value)] = &[
        // NOT on Boolean
        (Value::Boolean(true), "NOT", Value::Boolean(false)),
        (Value::Boolean(false), "NOT", Value::Boolean(true)),
        // NOT on Integer (uses to_bool: 0 → false, non-0 → true)
        (Value::Integer(0), "NOT", Value::Boolean(true)),
        (Value::Integer(1), "NOT", Value::Boolean(false)),
        (Value::Integer(42), "NOT", Value::Boolean(false)),
        // "!" alias for NOT
        (Value::Boolean(true), "!", Value::Boolean(false)),
        (Value::Integer(0), "!", Value::Boolean(true)),
        // Unknown operator
        (Value::Integer(5), "UNKNOWN", Value::Null),
        // NOT on Null
        (Value::Null, "NOT", Value::Boolean(true)),
        // Case-insensitive operator
        (Value::Boolean(true), "not", Value::Boolean(false)),
    ];

    let mut failures: Vec<String> = Vec::new();

    for (input, op, _expected) in cases {
        // Path 1: facade via `Expression::UnaryOp(op, Literal(value))`
        let input_str = match input {
            Value::Null => "NULL".to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Value::Float(f) => f.to_string(),
            _ => format!("{:?}", input),
        };
        let expr_un = ParserExpr::UnaryOp(op.to_string(), Box::new(ParserExpr::Literal(input_str)));
        let from_facade =
            sqlrustgo::expr_utils::evaluate_expression(&expr_un, &empty_row, &table_info);

        // Path 2: new single-source-of-truth
        let from_evaluator = eval_unary_op(input, op);

        if from_facade != Ok(from_evaluator.clone()) {
            failures.push(format!(
                "UnaryOp({op:?}, {input:?}): facade={from_facade:?} evaluator={from_evaluator:?}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "INT-3 UnaryOp delegation: facade and executor::expr disagree:\n{}",
        failures.join("\n")
    );
}
