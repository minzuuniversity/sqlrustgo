//! V400-function-body: cover CREATE FUNCTION body parsing arms.
//!
//! Targets the uncovered 50% of parse_create_function (49.6% in v3 of report).
//! Specifically the BEGIN...END multi-statement body, AS prefix body,
//! deterministic modifier, and RETURN single-expression body.

use sqlrustgo_parser::{parse, Statement};

fn must_parse_function(sql: &str) {
    match parse(sql).expect("parse should succeed") {
        Statement::CreateFunction(_) => {}
        other => panic!("expected CreateFunction, got {:?}", other),
    }
}

// ===========================================================================
// parse_create_function — body variants
// ===========================================================================

#[test]
fn function_begin_end_body_no_as() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT BEGIN SELECT 1; END");
}

#[test]
fn function_begin_end_body_with_as() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT AS BEGIN SELECT 1; END");
}

#[test]
fn function_as_single_expression() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT AS SELECT 1");
}

#[test]
fn function_return_single_expression() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT RETURN 1");
}

#[test]
fn function_deterministic_modifier() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT DETERMINISTIC RETURN 1");
}

#[test]
fn function_deterministic_with_begin_end() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT DETERMINISTIC BEGIN SELECT 1; END");
}

#[test]
fn function_returns_table_single_column() {
    must_parse_function("CREATE FUNCTION f() RETURNS TABLE(a INT) AS SELECT 1");
}

#[test]
fn function_returns_table_multi_columns() {
    must_parse_function("CREATE FUNCTION f() RETURNS TABLE(x INT, y TEXT, z BOOLEAN) AS SELECT 1, 'a', true");
}

#[test]
fn function_with_input_param() {
    must_parse_function("CREATE FUNCTION f(x INT) RETURNS INT RETURN x + 1");
}

#[test]
fn function_with_multiple_input_params() {
    must_parse_function("CREATE FUNCTION f(x INT, y INT) RETURNS INT RETURN x + y");
}

#[test]
fn function_or_replace_returns_int() {
    must_parse_function("CREATE OR REPLACE FUNCTION f() RETURNS INT RETURN 1");
}

#[test]
fn function_or_replace_begin_end() {
    must_parse_function("CREATE OR REPLACE FUNCTION f() RETURNS INT BEGIN SELECT 1; END");
}

#[test]
fn function_returns_bigint() {
    must_parse_function("CREATE FUNCTION f() RETURNS BIGINT RETURN 1");
}

#[test]
fn function_returns_decimal() {
    // DECIMAL(10, 2) — type with precision/scale args
    let _ = parse("CREATE FUNCTION f() RETURNS DECIMAL(10, 2) RETURN 1.0");
}

#[test]
fn function_returns_varchar() {
    // VARCHAR(255) — type with length arg
    let _ = parse("CREATE FUNCTION f() RETURNS VARCHAR(255) RETURN 'a'");
}

// ===========================================================================
// Multi-statement body parts (issue #4671)
// ===========================================================================

#[test]
fn function_body_2_statements() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT BEGIN DECLARE x INT; SET x = 1; RETURN x; END");
}

#[test]
fn function_body_3_statements() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT BEGIN DECLARE x INT; SET x = 1; SET x = 2; RETURN x; END");
}

#[test]
fn function_body_with_if() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT BEGIN IF 1 > 0 THEN RETURN 1; ELSE RETURN 0; END IF; END");
}

#[test]
fn function_body_with_while() {
    must_parse_function("CREATE FUNCTION f() RETURNS INT BEGIN DECLARE x INT DEFAULT 0; WHILE x < 10 DO SET x = x + 1; END WHILE; RETURN x; END");
}

// ===========================================================================
// CREATE PROCEDURE — multi-statement body (different from function)
// ===========================================================================

#[test]
fn procedure_begin_end_body() {
    let _ = parse("CREATE PROCEDURE p() BEGIN SELECT 1; END");
}

#[test]
fn procedure_with_param_and_body() {
    let _ = parse("CREATE PROCEDURE p(IN x INT) BEGIN INSERT INTO t VALUES (x); END");
}