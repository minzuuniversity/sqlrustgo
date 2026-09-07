// V313-101 / Issue #4671: CREATE FUNCTION multi-statement body and
// RETURNS TABLE parsing.

use sqlrustgo_parser::parse;

#[test]
fn parse_create_function_with_begin_end_no_as() {
    let sql = "CREATE FUNCTION f2(x INT) RETURNS INT \
               BEGIN DECLARE r INT; SET r = x + 100; RETURN r; END";
    assert!(
        parse(sql).is_ok(),
        "BEGIN/END without AS must parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn parse_create_function_with_begin_end_with_as() {
    let sql = "CREATE FUNCTION f2(x INT) RETURNS INT AS \
               BEGIN DECLARE r INT; SET r = x + 100; RETURN r; END";
    assert!(
        parse(sql).is_ok(),
        "AS BEGIN/END must still parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn parse_create_function_single_expr_still_works() {
    let sql = "CREATE FUNCTION f1(x INT) RETURNS INT RETURN x * 2";
    assert!(
        parse(sql).is_ok(),
        "single-expression RETURN body must still parse"
    );
}

#[test]
fn parse_create_function_with_returns_table() {
    let sql = "CREATE FUNCTION f3() RETURNS TABLE(id INT, name TEXT) \
               RETURN SELECT x FROM t";
    assert!(
        parse(sql).is_ok(),
        "RETURNS TABLE(...) must parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn parse_create_function_with_returns_table_and_begin_end() {
    let sql = "CREATE FUNCTION f4() RETURNS TABLE(id INT) \
               BEGIN RETURN SELECT x FROM t; END";
    assert!(
        parse(sql).is_ok(),
        "RETURNS TABLE with BEGIN/END must parse: {:?}",
        parse(sql).err()
    );
}
