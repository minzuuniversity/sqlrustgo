//! Planner types for SQL statements
//!
//! This module re-exports all statement types used by the query planner.

use crate::Expr;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Column;

    #[test]
    fn test_merge_statement_new() {
        let stmt = MergeStatement::new(
            "target".to_string(),
            "source".to_string(),
            Expr::Column(Column::new("id".to_string())),
            None,
            None,
        );
        assert_eq!(stmt.target_table, "target");
        assert_eq!(stmt.source_table, "source");
        assert!(stmt.matched_clause.is_none());
        assert!(stmt.not_matched_clause.is_none());
    }

    #[test]
    fn test_merge_clause_new() {
        let clause = MergeClause::new(
            vec!["col1".to_string()],
            vec![Expr::Literal(sqlrustgo_types::Value::Integer(42))],
            vec!["col2".to_string()],
            vec![Expr::Literal(sqlrustgo_types::Value::Text("x".to_string()))],
        );
        assert_eq!(clause.update_columns, vec!["col1".to_string()]);
        assert_eq!(clause.insert_columns, vec!["col2".to_string()]);
        assert_eq!(clause.update_values.len(), 1);
        assert_eq!(clause.insert_values.len(), 1);
    }

    #[test]
    fn test_merge_statement_with_clauses() {
        let matched = Some(MergeClause::new(
            vec!["name".to_string()],
            vec![Expr::Literal(sqlrustgo_types::Value::Text(
                "updated".to_string(),
            ))],
            vec![],
            vec![],
        ));
        let stmt = MergeStatement::new(
            "users".to_string(),
            "staging".to_string(),
            Expr::Column(Column::new("id".to_string())),
            matched,
            None,
        );
        assert!(stmt.matched_clause.is_some());
        assert!(stmt.not_matched_clause.is_none());
    }

    #[test]
    fn test_clone_and_debug() {
        let stmt = MergeStatement::new(
            "t".to_string(),
            "s".to_string(),
            Expr::Column(Column::new("a".to_string())),
            None,
            None,
        );
        let cloned = stmt.clone();
        assert_eq!(stmt.target_table, cloned.target_table);
        let debug_str = format!("{:?}", stmt);
        assert!(debug_str.contains("MergeStatement"));
    }
}
