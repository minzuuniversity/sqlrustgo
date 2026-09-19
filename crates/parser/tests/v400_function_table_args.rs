//! V400-function-table-args: coverage tests for `from_function_args` AST paths.
//!
//! Targets lines 6560-6790 of parser.rs which contain the AST conversion
//! for FROM clause with table-valued function arguments (e.g.
//! `SELECT * FROM unnest([1,2,3])`).

use sqlrustgo_parser::parse;

// ===========================================================================
// FROM unnest() with literal array args
// ===========================================================================

#[test]
fn from_unnest_with_int_array() {
    let _ = parse("SELECT * FROM unnest([1, 2, 3])");
}

#[test]
fn from_unnest_with_string_array() {
    let _ = parse("SELECT * FROM unnest(['a', 'b', 'c'])");
}

#[test]
fn from_unnest_with_mixed_array() {
    let _ = parse("SELECT * FROM unnest([1, 'two', 3.0])");
}

#[test]
fn from_unnest_with_single_arg() {
    let _ = parse("SELECT * FROM unnest([42])");
}

#[test]
fn from_unnest_with_empty_array() {
    let _ = parse("SELECT * FROM unnest([])");
}

#[test]
fn from_unnest_with_nested_array() {
    let _ = parse("SELECT * FROM unnest([[1, 2], [3, 4]])");
}

// ===========================================================================
// FROM json_each / json_tree
// ===========================================================================

#[test]
fn from_json_each_with_string() {
    let _ = parse("SELECT * FROM json_each('[1, 2, 3]')");
}

#[test]
fn from_json_each_with_object() {
    let _ = parse("SELECT * FROM json_each('{\"a\": 1, \"b\": 2}')");
}

#[test]
fn from_json_tree_with_nested() {
    let _ = parse("SELECT * FROM json_tree('{\"a\": {\"b\": 1}}')");
}

// ===========================================================================
// FROM generate_series
// ===========================================================================

#[test]
fn from_generate_series_two_args() {
    let _ = parse("SELECT * FROM generate_series(1, 10)");
}

#[test]
fn from_generate_series_three_args() {
    let _ = parse("SELECT * FROM generate_series(1, 100, 5)");
}

#[test]
fn from_generate_series_with_step_negative() {
    let _ = parse("SELECT * FROM generate_series(10, 1, -1)");
}

// ===========================================================================
// FROM function with column alias
// ===========================================================================

#[test]
fn from_function_with_alias() {
    let _ = parse("SELECT * FROM unnest([1,2,3]) AS t(x)");
}

#[test]
fn from_function_with_alias_multi_col() {
    let _ = parse("SELECT * FROM json_each('{}') AS je(key, value)");
}

// ===========================================================================
// FROM subquery (no function)
// ===========================================================================

#[test]
fn from_subquery_basic() {
    let _ = parse("SELECT * FROM (SELECT 1) AS t");
}

#[test]
fn from_subquery_with_where() {
    let _ = parse("SELECT * FROM (SELECT a FROM t WHERE a > 0) AS sub");
}

#[test]
fn from_nested_subquery() {
    let _ = parse("SELECT * FROM (SELECT * FROM (SELECT 1) AS a) AS b");
}

// ===========================================================================
// FROM clause with multiple sources
// ===========================================================================

#[test]
fn from_table_and_function() {
    let _ = parse("SELECT * FROM t, unnest([1,2])");
}

#[test]
fn from_table_cross_join_function() {
    let _ = parse("SELECT * FROM t CROSS JOIN unnest([1,2])");
}

// ===========================================================================
// JOIN with subquery
// ===========================================================================

#[test]
fn join_with_subquery_left() {
    let _ = parse("SELECT * FROM t LEFT JOIN (SELECT 1) AS sub ON TRUE");
}

#[test]
fn join_with_subquery_inner() {
    let _ = parse("SELECT * FROM t INNER JOIN (SELECT 1) AS sub ON t.id = sub.id");
}

#[test]
fn join_with_function() {
    let _ = parse("SELECT * FROM t JOIN unnest([1,2,3]) AS u(x) ON TRUE");
}

// ===========================================================================
// WITH + table function
// ===========================================================================

#[test]
fn cte_used_by_table_function() {
    let _ = parse("WITH cte AS (SELECT 1 AS a) SELECT * FROM cte, unnest([1,2])");
}

#[test]
fn table_function_in_cte_definition() {
    let _ = parse("WITH cte AS (SELECT * FROM unnest([1,2,3])) SELECT * FROM cte");
}