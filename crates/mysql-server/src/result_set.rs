//! #5271: turning an `ExecutorResult` into a result set the wire can carry.
//!
//! This exists as its own unit so the rules it enforces can be tested against
//! the code that actually runs. The logic used to live inline in the command
//! loop, where the only way to exercise it was a live MySQL client — and a
//! test that reimplements the rules proves only that the author understood
//! them.
//!
//! The invariant: a result's shape comes from its schema. Not from its rows,
//! not from the query text. An empty result over a two-column table is a
//! two-column result, and its column names are the table's.

use sqlrustgo_executor::ExecutorResult;

/// A result set that has been checked and described, ready to encode.
///
/// Deliberately separate from `ExecutorResult`: this one is allowed to exist
/// only once the rows agree with the declared columns.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedResultSet {
    /// Column names in result order.
    pub column_names: Vec<String>,
    /// MySQL type names, parallel to `column_names`.
    pub column_types: Vec<String>,
    /// The rows, unchanged.
    pub rows: Vec<Vec<sqlrustgo_types::Value>>,
}

impl PreparedResultSet {
    /// The Column Count packet field: how many Column Definitions follow.
    pub fn column_count(&self) -> usize {
        self.column_names.len()
    }
}

/// The placeholder used when a column's semantic type is not resolved.
///
/// #5271 R2: `ResultColumn::data_type` is `None` for expressions and
/// aggregates, and the wire format still needs a valid type code. This keeps
/// the encoding legal without claiming the column is a VARCHAR.
pub const UNRESOLVED_TYPE: &str = "VARCHAR(255)";

/// Validate and describe a result set. Performs no I/O.
///
/// Fails when the rows disagree with the declared columns: emitting a Column
/// Count that does not match the row width is exactly the malformed packet a
/// client reports as `ERROR 2027`, and truncating the columns or padding the
/// rows would hide the mismatch behind a well-formed-looking result.
pub fn prepare_result_set(result: &ExecutorResult) -> Result<PreparedResultSet, String> {
    if let Some(bad) = result
        .rows
        .iter()
        .position(|row| row.len() != result.columns.len())
    {
        if !result.columns.is_empty() {
            return Err(format!(
                "result schema mismatch: row {bad} has {} values but {} columns are declared",
                result.rows[bad].len(),
                result.columns.len(),
            ));
        }
    }

    Ok(PreparedResultSet {
        column_names: result.columns.iter().map(|c| c.name.clone()).collect(),
        column_types: result
            .columns
            .iter()
            .map(|c| c.data_type.clone().unwrap_or_else(|| UNRESOLVED_TYPE.to_string()))
            .collect(),
        rows: result.rows.clone(),
    })
}
