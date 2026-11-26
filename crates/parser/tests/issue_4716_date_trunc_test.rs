//! Issue #4716 / spec `executor-date-trunc-function` — the parser must
//! reject an unsupported DATE_TRUNC unit instead of letting it reach the
//! executor, where it used to evaluate to NULL (a silent wrong answer).
//!
//! Spec: "unknown unit must error — `DATE_TRUNC` MUST reject unsupported unit
//! names with a SqlError. … a SqlError containing 'DATE_TRUNC unit must be
//! one of' is raised."

use sqlrustgo_parser::{parse, Expression, Statement, DATE_TRUNC_UNITS};

fn select_expr(sql: &str) -> Expression {
    match parse(sql).expect("statement must parse") {
        Statement::Select(sel) => sel.columns[0]
            .expression
            .clone()
            .expect("SELECT column must carry an expression"),
        other => panic!("expected a SELECT, got {other:?}"),
    }
}

#[test]
fn unsupported_unit_is_rejected_at_parse_time() {
    let err = parse("SELECT DATE_TRUNC('decade', dt) FROM t")
        .expect_err("'decade' is not a supported unit and must be rejected");
    assert!(
        err.contains("DATE_TRUNC unit must be one of"),
        "the error must name the supported units, got: {err}"
    );
}

#[test]
fn every_unsupported_unit_is_rejected() {
    // Units PostgreSQL knows but this engine does not implement: the spec
    // pins "decade" as *not recognised*, and the same rule applies to the
    // rest of that family.
    for unit in ["millennium", "century", "decade", "microsecond", "epoch"] {
        let sql = format!("SELECT DATE_TRUNC('{unit}', dt) FROM t");
        let err = parse(&sql).expect_err("unsupported unit must be rejected");
        assert!(
            err.contains("DATE_TRUNC unit must be one of"),
            "unit {unit:?} must be rejected with the spec message, got: {err}"
        );
    }
}

#[test]
fn every_supported_unit_parses() {
    for unit in DATE_TRUNC_UNITS {
        let sql = format!("SELECT DATE_TRUNC('{unit}', dt) FROM t");
        let expr = select_expr(&sql);
        match expr {
            Expression::FunctionCall(name, args) => {
                assert_eq!(name, "DATE_TRUNC");
                assert_eq!(args.len(), 2, "DATE_TRUNC takes two arguments");
                assert_eq!(args[0], Expression::Literal(format!("'{unit}'")));
            }
            other => panic!("expected a FunctionCall for unit {unit:?}, got {other:?}"),
        }
    }
}

#[test]
fn unit_is_case_insensitive() {
    // The spec's scenarios use lower-case units; the engine's other unit
    // checks (TIMESTAMPDIFF) accept any case.
    for unit in ["day", "DAY", "Day"] {
        let sql = format!("SELECT DATE_TRUNC('{unit}', dt) FROM t");
        let expr = select_expr(&sql);
        match expr {
            Expression::FunctionCall(_, args) => {
                assert_eq!(args[0], Expression::Literal(format!("'{unit}'")))
            }
            other => panic!("expected a FunctionCall, got {other:?}"),
        }
    }
}

#[test]
fn null_unit_is_not_an_error() {
    // The function is strict: DATE_TRUNC(NULL, dt) yields NULL. It must
    // not be reported as an unsupported unit.
    let expr = select_expr("SELECT DATE_TRUNC(NULL, dt) FROM t");
    match expr {
        Expression::FunctionCall(_, args) => {
            assert_eq!(args[0], Expression::Literal("NULL".to_string()))
        }
        other => panic!("expected a FunctionCall, got {other:?}"),
    }
}
