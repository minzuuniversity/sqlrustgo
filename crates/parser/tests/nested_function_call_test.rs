//! Regression tests for Issue #4490: parser must correctly handle single
//! and multi-argument function calls whose arguments are themselves
//! function calls (e.g. `SELECT year(now())`).
//!
//! Previously the general `Identifier(args)` parsing path
//! (`crates/parser/src/parser.rs` line ~7513) consumed an extra `)` after
//! the args loop, which broke any nested call whose last argument was a
//! function call. This was surfaced by `year(now())` and `month(now())`
//! reporting "Expected RParen, got Eof".

use sqlrustgo_parser::{parse, Expression};

fn expect_parse_ok(sql: &str) {
    match parse(sql) {
        Ok(_) => {}
        Err(e) => panic!("expected {sql:?} to parse, got error: {e}"),
    }
}

fn top_function_call_args(stmt: &sqlrustgo_parser::Statement, name: &str) -> Option<usize> {
    fn walk(expr: &Expression, name: &str, out: &mut Option<usize>) {
        match expr {
            Expression::FunctionCall(n, args) if n.eq_ignore_ascii_case(name) => {
                if out.is_none() {
                    *out = Some(args.len());
                }
            }
            Expression::BinaryOp(l, _, r) => {
                walk(l, name, out);
                walk(r, name, out);
            }
            Expression::UnaryOp(_, e) => walk(e, name, out),
            Expression::FunctionCall(_, args) => {
                for a in args {
                    walk(a, name, out);
                }
            }
            Expression::InList(e, items) | Expression::NotInList(e, items) => {
                walk(e, name, out);
                for i in items {
                    walk(i, name, out);
                }
            }
            _ => {}
        }
    }
    let mut out = None;
    if let sqlrustgo_parser::Statement::Select(s) = stmt {
        for col in &s.columns {
            if let Some(e) = &col.expression {
                walk(e, name, &mut out);
            }
        }
    }
    out
}

#[test]
fn nested_as_only_arg_year_now() {
    expect_parse_ok("SELECT year(now())");
}

#[test]
fn nested_as_only_arg_month_now() {
    expect_parse_ok("SELECT month(now())");
}

#[test]
fn zero_arg_outer() {
    expect_parse_ok("SELECT foo()");
}

#[test]
fn simple_arg() {
    expect_parse_ok("SELECT foo(x)");
}

#[test]
fn literal_arg() {
    expect_parse_ok("SELECT foo(1)");
}

#[test]
fn nested_first_of_many() {
    expect_parse_ok("SELECT foo(bar(), x)");
}

#[test]
fn nested_last_of_many() {
    expect_parse_ok("SELECT foo(x, bar())");
}

#[test]
fn deep_nested_year_month_day() {
    expect_parse_ok("SELECT year(now()), month(now()), day(now())");
}

#[test]
fn ast_year_now_shape() {
    let stmt = parse("SELECT year(now())").expect("parse");
    let argc = top_function_call_args(&stmt, "YEAR");
    assert_eq!(argc, Some(1), "YEAR should have 1 arg");
    let now_argc = top_function_call_args(&stmt, "NOW");
    assert_eq!(now_argc, Some(0), "NOW should have 0 args");
}

#[test]
fn ast_two_args_last_nested() {
    let stmt = parse("SELECT foo(x, bar())").expect("parse");
    let argc = top_function_call_args(&stmt, "FOO");
    assert_eq!(argc, Some(2));
    let barc = top_function_call_args(&stmt, "BAR");
    assert_eq!(barc, Some(0));
}

#[test]
fn cast_plain() {
    expect_parse_ok("SELECT CAST(1 AS INTEGER)");
}

#[test]
fn cast_with_substring_arg() {
    expect_parse_ok("SELECT CAST(SUBSTR(x, 1, 4) AS INTEGER)");
}

#[test]
fn cast_with_nested_function_arg() {
    expect_parse_ok("SELECT CAST(year(now()) AS INTEGER)");
}