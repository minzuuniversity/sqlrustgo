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
            },
            ColumnDefinition {
                name: "SUM(l_quantity)".to_string(),
                data_type: "FLOAT".to_string(),
                nullable: false,
                primary_key: false,
            },
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
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
