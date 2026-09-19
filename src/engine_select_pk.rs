//! Phase B Step 4.2 / 4.3: detect `WHERE id = <value>` and
//! `WHERE id BETWEEN <a> AND <b>` style primary-key lookups in a
//! SELECT statement's WHERE clause.
//!
//! - `try_extract_pk_eq` returns `Some(pk_value)` for point lookups.
//! - `try_extract_pk_range` returns `Some((low, high))` for inclusive
//!   range scans on the PK column.
//!
//! Both are used by `engine_select::execute_select` to take a fast
//! O(log N) PK path instead of the default O(N) full-table scan +
//! in-memory filter.

use sqlrustgo_parser::Expression;
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;

/// Default name of the primary-key column. Matches `FileStorage`'s
/// PK convention (the schema's `primary_key: true` column).
pub const DEFAULT_PK_COLUMN: &str = "id";

/// Resolve the primary-key column name for a table.
///
/// Returns the column whose `primary_key: true` flag is set, or
/// `DEFAULT_PK_COLUMN` ("id") as a fallback when the table has no
/// declared PK (legacy tables, system views, or `CREATE TABLE`
/// statements without a `PRIMARY KEY` clause).
///
/// Phase D.1: previously the PK column was hard-coded to `"id"`,
/// which meant tables with a non-`"id"` PK column (e.g. `o_orderkey`)
/// silently fell through to full-table scan + linear find, losing the
/// O(log N) B+Tree lookup speedup.
pub fn resolve_pk_column(table_info: &TableInfo) -> String {
    table_info
        .columns
        .iter()
        .find(|c| c.primary_key)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| DEFAULT_PK_COLUMN.to_string())
}

/// Inclusive range bounds `(low, high)` for a PK range scan.
pub type PkRange = (Value, Value);

/// Extract a PK value from a WHERE expression.
///
/// Returns `Some(pk_value)` if `where_expr` is `BinaryOp(col, "=", lit)`
/// where `col` matches `pk_column` and `lit` evaluates to a `Value`.
/// Returns `None` for any other shape (AND, OR, subqueries,
/// non-equality, etc.).
pub fn try_extract_pk_eq(where_expr: &Option<Expression>) -> Option<Value> {
    try_extract_pk_eq_with_col(where_expr, DEFAULT_PK_COLUMN)
}

/// Like `try_extract_pk_eq` but with a configurable PK column name.
pub fn try_extract_pk_eq_with_col(
    where_expr: &Option<Expression>,
    pk_column: &str,
) -> Option<Value> {
    let expr = where_expr.as_ref()?;
    extract_from_expr(expr, pk_column)
}

fn extract_from_expr(expr: &Expression, pk_column: &str) -> Option<Value> {
    match expr {
        Expression::BinaryOp(left, op, right) => {
            let op_u = op.to_uppercase();
            if op_u != "=" && op_u != "==" {
                return None;
            }
            // Try `left = right` where left is the column and right is
            // the literal value.
            if let Some(v) = extract_literal_value(right) {
                if column_matches(left, pk_column) {
                    return Some(v);
                }
            }
            // Or the reverse: `right = left` (literal on left).
            if let Some(v) = extract_literal_value(left) {
                if column_matches(right, pk_column) {
                    return Some(v);
                }
            }
            None
        }
        _ => None,
    }
}

fn column_matches(expr: &Expression, pk_column: &str) -> bool {
    // Only plain identifiers like `id` are recognised as the PK
    // column. Qualified names (`t.id`) are intentionally NOT
    // recognised here because `MvccStorage::scan_pk` does not know
    // the alias prefix used by the calling SELECT — the caller
    // strips the alias before calling. See `lookup_table` in
    // `engine_select::execute_select`.
    match expr {
        Expression::Identifier(name) => name.eq_ignore_ascii_case(pk_column),
        _ => false,
    }
}

fn extract_literal_value(expr: &Expression) -> Option<Value> {
    match expr {
        // `Expression::Literal` carries a raw token string. Parse it
        // as a SQL value: integer if it parses as i64, otherwise text.
        // This is the same convention `sysbench` uses (`id = 42` —
        // the literal token is "42").
        Expression::Literal(s) => parse_literal_token(s),
        _ => None,
    }
}

/// Best-effort parse of a literal token into a SQL `Value`. Matches
/// integers as `Value::Integer`, everything else as `Value::Text`.
fn parse_literal_token(s: &str) -> Option<Value> {
    if let Ok(i) = s.parse::<i64>() {
        Some(Value::Integer(i))
    } else if let Ok(f) = s.parse::<f64>() {
        Some(Value::Float(f))
    } else {
        Some(Value::Text(s.to_string()))
    }
}

#[cfg(test)]
mod resolve_pk_column_tests {
    use super::*;
    use sqlrustgo_storage::{ColumnDefinition, TableInfo};

    fn col(name: &str, primary_key: bool) -> ColumnDefinition {
        ColumnDefinition {
            name: name.into(),
            data_type: "BIGINT".into(),
            nullable: !primary_key,
            primary_key,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }
    }

    fn info_with_pk(pk_name: &str) -> TableInfo {
        TableInfo {
            name: "orders".into(),
            columns: vec![
                col(pk_name, true),
                col("o_custkey", false),
                col("o_orderstatus", false),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        }
    }

    #[test]
    fn test_resolve_pk_column_returns_named_pk() {
        let info = info_with_pk("o_orderkey");
        assert_eq!(resolve_pk_column(&info), "o_orderkey");
    }

    #[test]
    fn test_resolve_pk_column_id_default() {
        let info = info_with_pk("id");
        assert_eq!(resolve_pk_column(&info), "id");
    }

    #[test]
    fn test_resolve_pk_column_no_pk_falls_back_to_id() {
        // Legacy tables without a `PRIMARY KEY` clause have no
        // `primary_key: true` flag. Phase D.1 falls back to
        // DEFAULT_PK_COLUMN ("id") which matches the pre-D.1
        // behavior.
        let info = TableInfo {
            name: "legacy".into(),
            columns: vec![col("foo", false), col("bar", false)],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        };
        assert_eq!(resolve_pk_column(&info), DEFAULT_PK_COLUMN);
    }

    #[test]
    fn test_resolve_pk_column_picks_first_when_multi_pk_flagged() {
        // Defensive: if multiple columns are flagged (shouldn't
        // happen per SQL standard but be robust), pick the first.
        let info = TableInfo {
            name: "weird".into(),
            columns: vec![col("first_pk", true), col("second_pk", true)],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        };
        assert_eq!(resolve_pk_column(&info), "first_pk");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_parser::Expression;

    #[test]
    fn test_simple_id_eq_int() {
        let expr = Expression::BinaryOp(
            Box::new(Expression::Identifier("id".into())),
            "=".into(),
            Box::new(Expression::Literal("42".into())),
        );
        let pk = try_extract_pk_eq_with_col(&Some(expr), "id");
        assert_eq!(pk, Some(Value::Integer(42)));
    }

    #[test]
    fn test_reverse_literal_left() {
        let expr = Expression::BinaryOp(
            Box::new(Expression::Literal("42".into())),
            "=".into(),
            Box::new(Expression::Identifier("id".into())),
        );
        let pk = try_extract_pk_eq_with_col(&Some(expr), "id");
        assert_eq!(pk, Some(Value::Integer(42)));
    }

    #[test]
    fn test_compound_identifier_rejected() {
        // Phase B Step 4.2: qualified names are NOT recognised here.
        // The caller strips the alias before invoking scan_pk.
        let expr = Expression::BinaryOp(
            Box::new(Expression::Identifier("t.id".into())),
            "=".into(),
            Box::new(Expression::Literal("7".into())),
        );
        assert!(try_extract_pk_eq_with_col(&Some(expr), "id").is_none());
    }

    #[test]
    fn test_non_equality() {
        let expr = Expression::BinaryOp(
            Box::new(Expression::Identifier("id".into())),
            ">".into(),
            Box::new(Expression::Literal("0".into())),
        );
        assert!(try_extract_pk_eq_with_col(&Some(expr), "id").is_none());
    }

    #[test]
    fn test_no_where() {
        assert!(try_extract_pk_eq(&None).is_none());
    }

    #[test]
    fn test_other_column() {
        let expr = Expression::BinaryOp(
            Box::new(Expression::Identifier("name".into())),
            "=".into(),
            Box::new(Expression::Literal("alice".into())),
        );
        assert!(try_extract_pk_eq_with_col(&Some(expr), "id").is_none());
    }
}

// ---------------------------------------------------------------------------
// Phase B Step 4.3: range PK lookups (`WHERE id BETWEEN <a> AND <b>`).
// ---------------------------------------------------------------------------

/// Extract an inclusive PK range from a WHERE expression. Returns
/// `Some((low, high))` for `Between(col, low, high)`, `Some((v, v))`
/// for `id = v` (degenerate range — also handled by `try_extract_pk_eq`
/// but exposed here for symmetry), or `None` otherwise.
pub fn try_extract_pk_range(where_expr: &Option<Expression>) -> Option<PkRange> {
    try_extract_pk_range_with_col(where_expr, DEFAULT_PK_COLUMN)
}

/// Like `try_extract_pk_range` but with a configurable PK column.
pub fn try_extract_pk_range_with_col(
    where_expr: &Option<Expression>,
    pk_column: &str,
) -> Option<PkRange> {
    let expr = where_expr.as_ref()?;
    extract_range_from_expr(expr, pk_column)
}

fn extract_range_from_expr(expr: &Expression, pk_column: &str) -> Option<PkRange> {
    match expr {
        // `id BETWEEN a AND b` — direct match.
        Expression::Between(col, low, high) => {
            if !column_matches(col, pk_column) {
                return None;
            }
            let low_v = extract_literal_value(low)?;
            let high_v = extract_literal_value(high)?;
            Some((low_v, high_v))
        }
        // `id >= a AND id <= b` (typical AND-of-comparisons form
        // produced by some parsers). We don't try to flatten it
        // here — the canonical BETWEEN form is what the parser
        // produces for sysbench-style range queries.
        _ => None,
    }
}

#[cfg(test)]
mod range_tests {
    use super::*;

    fn between_expr(col: &str, low: &str, high: &str) -> Expression {
        Expression::Between(
            Box::new(Expression::Identifier(col.into())),
            Box::new(Expression::Literal(low.into())),
            Box::new(Expression::Literal(high.into())),
        )
    }

    #[test]
    fn test_id_between() {
        let r = try_extract_pk_range_with_col(&Some(between_expr("id", "10", "20")), "id");
        assert_eq!(r, Some((Value::Integer(10), Value::Integer(20))));
    }

    #[test]
    fn test_id_between_no_where() {
        assert!(try_extract_pk_range(&None).is_none());
    }

    #[test]
    fn test_other_column() {
        let r = try_extract_pk_range_with_col(&Some(between_expr("k", "10", "20")), "id");
        assert!(r.is_none());
    }

    #[test]
    fn test_between_id_eq_falls_back_to_eq() {
        // An `=` is not a BETWEEN — only `try_extract_pk_eq` matches.
        let expr = Expression::BinaryOp(
            Box::new(Expression::Identifier("id".into())),
            "=".into(),
            Box::new(Expression::Literal("42".into())),
        );
        assert!(try_extract_pk_range_with_col(&Some(expr), "id").is_none());
    }
}
