//! Planner types for SQL statements
//!
//! This module re-exports all statement types used by the query planner.

use crate::{Expr, Schema};

/// MERGE statement planner representation
#[derive(Debug, Clone)]
pub struct MergeStatement {
    pub target_table: String,
    pub source_table: String,
    pub on_condition: Expr,
    pub matched_clause: Option<MergeClause>,
    pub not_matched_clause: Option<MergeClause>,
}

/// Clause for matched (UPDATE) or not matched (INSERT) cases
#[derive(Debug, Clone)]
pub struct MergeClause {
    pub update_columns: Vec<String>,
    pub update_values: Vec<Expr>,
    pub insert_columns: Vec<String>,
    pub insert_values: Vec<Expr>,
}

impl MergeStatement {
    pub fn new(
        target_table: String,
        source_table: String,
        on_condition: Expr,
        matched_clause: Option<MergeClause>,
        not_matched_clause: Option<MergeClause>,
    ) -> Self {
        Self {
            target_table,
            source_table,
            on_condition,
            matched_clause,
            not_matched_clause,
        }
    }
}

impl MergeClause {
    pub fn new(
        update_columns: Vec<String>,
        update_values: Vec<Expr>,
        insert_columns: Vec<String>,
        insert_values: Vec<Expr>,
    ) -> Self {
        Self {
            update_columns,
            update_values,
            insert_columns,
            insert_values,
        }
    }
}