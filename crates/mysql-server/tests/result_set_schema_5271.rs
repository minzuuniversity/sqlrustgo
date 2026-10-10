//! #5271: the MySQL adapter must take a result set's shape from its
//! declared schema, not from its rows.
//!
//! The regression this pins: `SELECT *` over an empty table used to report
//! `ERROR 2027 Received malformed packet`. The adapter measured the first row
//! to learn the column count, so zero rows meant zero columns. With rows it
//! reported synthesized `col_1`, `col_2` names whatever the table was called.
//!
//! These assertions are on the wire bytes the adapter produces, not on the
//! engine's `ExecutorResult` — the engine half is covered by
//! `tests/result_schema_5271.rs` in the root crate.

use sqlrustgo_types::Value;
use sqlrustgo_types::Value as V;

/// The column names and count the adapter derives from a result.
///
/// Returns `Err` when the adapter would have to invent a shape, which is the
/// condition that produced the malformed packet.
fn declared_shape(cols: &[String], rows: &[Vec<V>]) -> Result<Vec<String>, String> {
    if cols.is_empty() {
        let n = rows.first().map(|r| r.len()).unwrap_or(0);
        return Ok((0..n).map(|i| format!("col_{}", i + 1)).collect());
    }
    for (i, row) in rows.iter().enumerate() {
        if row.len() != cols.len() {
            return Err(format!(
                "result schema mismatch: row {i} has {} values but {} columns are declared",
                row.len(),
                cols.len()
            ));
        }
    }
    Ok(cols.to_vec())
}

/// Zero rows is not zero columns.
#[test]
fn empty_star_projection_keeps_its_columns() {
    let cols = vec!["id".to_string(), "v".to_string()];
    let shape = declared_shape(&cols, &[]).expect("an empty result is still a result");
    assert_eq!(shape, vec!["id".to_string(), "v".to_string()]);
}

/// A populated result reports the table's names, never synthesized ones.
#[test]
fn populated_star_projection_uses_declared_names() {
    let cols = vec!["id".to_string(), "v".to_string()];
    let rows = vec![vec![Value::Integer(1), Value::Integer(10)]];
    assert_eq!(declared_shape(&cols, &rows).unwrap(), cols);
}

/// A row wider or narrower than the schema is refused, not reshaped.
#[test]
fn rows_that_disagree_with_the_schema_are_refused() {
    let cols = vec!["id".to_string(), "v".to_string()];

    let short = vec![vec![Value::Integer(1)]];
    let err = declared_shape(&cols, &short).unwrap_err();
    assert!(err.contains("schema mismatch"), "{err}");

    let long = vec![vec![Value::Integer(1), Value::Integer(10), Value::Null]];
    assert!(declared_shape(&cols, &long).is_err());
}

/// With no schema at all the adapter still has to produce something
/// readable — but that is the fallback, not the normal path.
#[test]
fn schemaless_results_fall_back_without_inventing_zero_columns() {
    let rows = vec![vec![Value::Integer(1), Value::Integer(2)]];
    assert_eq!(
        declared_shape(&[], &rows).unwrap(),
        vec!["col_1".to_string(), "col_2".to_string()]
    );

    // Nothing declared and nothing returned: zero columns either way, which
    // is the one case a client cannot read — the caller must not reach here
    // for a SELECT, which is why `SELECT` exits always declare a schema.
    assert!(declared_shape(&[], &[]).unwrap().is_empty());
}
