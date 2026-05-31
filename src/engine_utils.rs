//! Engine utilities - predicate evaluation, schema building, and constraint validation.
//! Extracted from execution_engine.rs for modularity.

use sqlrustgo_parser::{AggregateCall, AggregateFunction, Expression};
use sqlrustgo_storage::{ColumnDefinition, SqlResult, StorageEngine, TableInfo, Value};
use sqlrustgo_types::SqlError;

/// Validate foreign key constraints for a row before insert
pub fn validate_foreign_keys(
    storage: &dyn StorageEngine,
    table_info: &sqlrustgo_storage::TableInfo,
    row: &[Value],
    insert_columns: &[String],
) -> SqlResult<()> {
    for fk in &table_info.foreign_keys {
        // Collect FK column values from the row
        let fk_values: Vec<Value> = fk
            .columns
            .iter()
            .filter_map(|col_name| {
                let col_idx = if insert_columns.is_empty() {
                    table_info
                        .columns
                        .iter()
                        .position(|c| c.name.eq_ignore_ascii_case(col_name))
                } else {
                    insert_columns
                        .iter()
                        .position(|c| c.eq_ignore_ascii_case(col_name))
                };
                col_idx.and_then(|idx| row.get(idx).cloned())
            })
            .collect();

        // Skip if any FK value is NULL (NULL FKs are allowed)
        if fk_values.iter().any(|v| matches!(v, Value::Null)) {
            continue;
        }

        // Scan parent table to verify referenced row exists
        let parent_rows = storage.scan(&fk.referenced_table)?;

        // Find referenced column indices in parent table
        let ref_col_indices: Vec<usize> = fk
            .referenced_columns
            .iter()
            .filter_map(|col_name| {
                storage
                    .get_table_info(&fk.referenced_table)
                    .ok()?
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(col_name))
            })
            .collect();

        let parent_has_match = parent_rows.iter().any(|parent_row| {
            ref_col_indices
                .iter()
                .enumerate()
                .all(|(i, &col_idx)| parent_row.get(col_idx) == fk_values.get(i))
        });

        if !parent_has_match {
            return Err(SqlError::ExecutionError(format!(
                "Foreign key constraint failed: {} ({}) references {} ({}) which does not exist",
                table_info.name,
                fk.columns.join(", "),
                fk.referenced_table,
                fk.referenced_columns.join(", ")
            )));
        }
    }
    Ok(())
}

/// Evaluate a WHERE clause expression against a row
/// Returns true if the row matches the WHERE condition
/// Evaluate a predicate expression to a boolean result
/// Phase 1: UNKNOWN is folded to FALSE for WHERE filtering
/// All NULL handling is centralized here - no NULL logic in individual operators
pub fn eval_predicate(expr: &Expression, row: &[Value], table_info: &TableInfo) -> bool {
    match expr {
        // AND short-circuits on false
        Expression::BinaryOp(left, op, right) if op.to_uppercase() == "AND" => {
            eval_predicate(left, row, table_info) && eval_predicate(right, row, table_info)
        }
        // OR short-circuits on true
        Expression::BinaryOp(left, op, right) if op.to_uppercase() == "OR" => {
            eval_predicate(left, row, table_info) || eval_predicate(right, row, table_info)
        }
        // IS NULL - always goes through evaluate_expression for value extraction
        Expression::IsNull(inner) => {
            match crate::expr_utils::evaluate_expression(inner, row, table_info) {
                Ok(val) => matches!(val, Value::Null),
                Err(_) => false,
            }
        }
        // IS NOT NULL
        Expression::IsNotNull(inner) => {
            match crate::expr_utils::evaluate_expression(inner, row, table_info) {
                Ok(val) => !matches!(val, Value::Null),
                Err(_) => false,
            }
        }
        // Legacy IS NULL (col IS NULL) - now uses new Expression::IsNull
        Expression::BinaryOp(left, op, right)
            if op.to_uppercase() == "IS"
                && matches!(right.as_ref(), Expression::Literal(s) if s.to_uppercase() == "NULL") =>
        {
            eval_predicate(&Expression::IsNull(left.clone()), row, table_info)
        }
        // Legacy IS NOT NULL
        Expression::BinaryOp(left, op, right)
            if op.to_uppercase() == "IS NOT"
                && matches!(right.as_ref(), Expression::Literal(s) if s.to_uppercase() == "NULL") =>
        {
            eval_predicate(&Expression::IsNotNull(left.clone()), row, table_info)
        }
        // All comparison operators go through sql_compare
        Expression::BinaryOp(left, op, right) => {
            let left_val = crate::expr_utils::evaluate_expression(left, row, table_info)
                .unwrap_or(Value::Null);
            let right_val = crate::expr_utils::evaluate_expression(right, row, table_info)
                .unwrap_or(Value::Null);
            sql_compare(op, &left_val, &right_val)
        }
        // For other expressions, evaluate and check if truthy
        _ => match crate::expr_utils::evaluate_expression(expr, row, table_info) {
            Ok(val) => {
                matches!(val, Value::Boolean(true))
            }
            Err(_) => false,
        },
    }
}

/// Legacy alias for compatibility
#[allow(dead_code)]
pub fn evaluate_where_clause(expr: &Expression, row: &[Value], table_info: &TableInfo) -> bool {
    eval_predicate(expr, row, table_info)
}

/// SQL comparison operator
/// Returns false if either operand is NULL (UNKNOWN semantics)
/// This is Phase 1: UNKNOWN is folded to FALSE for WHERE filtering
pub fn sql_compare(op: &str, left: &Value, right: &Value) -> bool {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return false;
    }

    match op.to_uppercase().as_str() {
        "=" | "==" => left == right,
        "!=" | "<>" => left != right,
        ">" => crate::expr_utils::compare_values(left, right) > 0,
        ">=" => crate::expr_utils::compare_values(left, right) >= 0,
        "<" => crate::expr_utils::compare_values(left, right) < 0,
        "<=" => crate::expr_utils::compare_values(left, right) <= 0,
        _ => false,
    }
}

/// Find the index of a column by name in the table schema
/// For JOIN queries with combined tables, handles qualified names like "t2.id"
/// by routing to the correct portion of the combined schema.
/// Combined table naming: left_table.col, right_table.col
pub fn find_column_index(col_name: &str, table_info: &TableInfo) -> Option<usize> {
    if let Some((_qualifier, col)) = col_name.split_once('.') {
        for (i, c) in table_info.columns.iter().enumerate() {
            if c.name.eq_ignore_ascii_case(col_name) {
                return Some(i);
            }
        }
        table_info
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col))
    } else {
        table_info
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col_name))
    }
}

pub fn build_combined_schema(
    left_info: &TableInfo,
    right_table_name: &str,
    right_info: &TableInfo,
) -> SqlResult<TableInfo> {
    let mut columns = Vec::new();

    for c in &left_info.columns {
        columns.push(ColumnDefinition {
            name: format!("{}.{}", left_info.name, c.name),
            data_type: c.data_type.clone(),
            nullable: c.nullable,
            primary_key: c.primary_key,
        });
    }

    for c in &right_info.columns {
        columns.push(ColumnDefinition {
            name: format!("{}.{}", right_table_name, c.name),
            data_type: c.data_type.clone(),
            nullable: c.nullable,
            primary_key: c.primary_key,
        });
    }

    Ok(TableInfo {
        name: format!("{}_join_{}", left_info.name, right_table_name),
        columns,
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    })
}

pub fn build_aggregate_schema(
    group_by: &[Expression],
    aggregates: &[AggregateCall],
) -> SqlResult<TableInfo> {
    let mut columns = Vec::new();

    for expr in group_by {
        columns.push(ColumnDefinition {
            name: crate::expr_utils::expression_to_string(expr),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
        });
    }

    for agg in aggregates {
        let name = match agg.func {
            AggregateFunction::Count => {
                if agg.args.is_empty() {
                    "COUNT(*)".to_string()
                } else {
                    format!(
                        "COUNT({})",
                        agg.args
                            .iter()
                            .map(crate::expr_utils::expression_to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            AggregateFunction::Sum => {
                format!(
                    "SUM({})",
                    agg.args
                        .iter()
                        .map(crate::expr_utils::expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            AggregateFunction::Avg => {
                format!(
                    "AVG({})",
                    agg.args
                        .iter()
                        .map(crate::expr_utils::expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            AggregateFunction::Min => {
                format!(
                    "MIN({})",
                    agg.args
                        .iter()
                        .map(crate::expr_utils::expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            AggregateFunction::Max => {
                format!(
                    "MAX({})",
                    agg.args
                        .iter()
                        .map(crate::expr_utils::expression_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        };
        columns.push(ColumnDefinition {
            name,
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
        });
    }

    Ok(TableInfo {
        name: "aggregate".to_string(),
        columns,
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
    })
}
