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
