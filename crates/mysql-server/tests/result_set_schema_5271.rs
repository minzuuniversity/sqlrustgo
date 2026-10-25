//! #5271: the rules the MySQL adapter enforces on a result set.
//!
//! These call `prepare_result_set` — the same function the command loop
//! calls before it writes anything to the socket. The earlier version of
//! this file reimplemented those rules in a local `declared_shape()` helper,
//! which proved only that the author had understood them; a change to the
//! production path would have left the test green.

use sqlrustgo_executor::{ExecutorResult, ResultColumn};
use sqlrustgo_mysql_server::result_set::{prepare_result_set, UNRESOLVED_TYPE};
use sqlrustgo_types::Value;

/// Build a result by hand rather than through `with_columns`.
///
/// `with_columns` debug-asserts that rows agree with the schema, so it panics
/// in a test build before the adapter can refuse the result. The adapter is
/// the last line of defence and has to be reachable with a mismatched result —
/// which is exactly what a future SELECT exit could hand it.
fn inconsistent(columns: Vec<ResultColumn>, rows: Vec<Vec<Value>>) -> ExecutorResult {
    ExecutorResult {
        rows,
        affected_rows: 0,
        columns,
    }
}

fn cols(names: &[&str]) -> Vec<ResultColumn> {
    names
        .iter()
        .map(|n| ResultColumn {
            name: (*n).to_string(),
            data_type: Some("INT".to_string()),
        })
        .collect()
}

/// Zero rows is not zero columns. This is the case the client reported as
/// `ERROR 2027` when the column count came from the first row.
#[test]
fn empty_star_projection_keeps_its_columns() {
    let r = ExecutorResult::with_columns(cols(&["id", "v"]), vec![], 0);
    let p = prepare_result_set(&r).expect("an empty result is still a result");
    assert_eq!(p.column_count(), 2);
    assert_eq!(p.column_names, vec!["id", "v"]);
    assert!(p.rows.is_empty());
}

/// A populated result reports the table's names and the declared types.
#[test]
fn populated_star_projection_uses_declared_names_and_types() {
    let rows = vec![vec![Value::Integer(1), Value::Integer(10)]];
    let r = ExecutorResult::with_columns(cols(&["id", "v"]), rows, 1);
    let p = prepare_result_set(&r).unwrap();
    assert_eq!(p.column_names, vec!["id", "v"]);
    assert_eq!(p.column_types, vec!["INT", "INT"]);
}

/// Projection order is the result's, not the table's.
#[test]
fn projection_order_is_preserved() {
    let r = ExecutorResult::with_columns(cols(&["v", "id"]), vec![], 0);
    assert_eq!(
        prepare_result_set(&r).unwrap().column_names,
        vec!["v", "id"]
    );
}

/// An aggregate keeps its column: `COUNT(*)` over an empty table returns a
/// row, but the shape comes from the projection either way.
#[test]
fn aggregate_keeps_its_declared_column() {
    let r = ExecutorResult::with_columns(
        vec![ResultColumn::unnamed_type("n")],
        vec![vec![Value::Integer(0)]],
        1,
    );
    let p = prepare_result_set(&r).unwrap();
    assert_eq!(p.column_names, vec!["n"]);
    assert_eq!(p.column_types, vec![UNRESOLVED_TYPE]);
}

/// Rows narrower than the schema are refused rather than reshaped.
#[test]
fn short_rows_are_refused() {
    let r = inconsistent(cols(&["id", "v"]), vec![vec![Value::Integer(1)]]);
    let err = prepare_result_set(&r).expect_err("a 1-wide row cannot fill 2 columns");
    assert!(err.contains("schema mismatch"), "{err}");
}

/// Rows wider than the schema are refused too.
#[test]
fn wide_rows_are_refused() {
    let rows = vec![vec![Value::Integer(1), Value::Integer(10), Value::Null]];
    let r = inconsistent(cols(&["id", "v"]), rows);
    assert!(prepare_result_set(&r).is_err());
}

/// A column whose type is unresolved still gets a valid encoding, and does
/// not claim to be a VARCHAR of a particular width.
#[test]
fn unresolved_types_encode_without_claiming_semantics() {
    let r = ExecutorResult::with_columns(
        vec![ResultColumn::unnamed_type("expr")],
        vec![vec![Value::Integer(1)]],
        1,
    );
    let p = prepare_result_set(&r).unwrap();
    assert_eq!(p.column_types, vec![UNRESOLVED_TYPE]);
}

/// A command reply — no declared columns — describes nothing rather than
/// inventing a width from whatever rows happen to be attached.
#[test]
fn schemaless_results_describe_no_columns() {
    let r = ExecutorResult::new(vec![], 3);
    let p = prepare_result_set(&r).unwrap();
    assert_eq!(p.column_count(), 0);
    assert!(p.rows.is_empty());
}
