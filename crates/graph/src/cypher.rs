//! Cypher query language support.
//!
//! Filled in by M4. M1 placeholder so the crate compiles.

#![allow(dead_code)]

use crate::types::{GraphError, GraphResult};

/// A parsed Cypher query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CypherQuery {
    /// Original query source (for diagnostics).
    pub source: String,
}

impl CypherQuery {
    /// Parse a Cypher query string. M4 will replace this stub with a real parser.
    pub fn parse(_src: &str) -> GraphResult<Self> {
        Err(GraphError::CypherParse {
            line: 1,
            col: 1,
            message: "Cypher parser not implemented yet (M4)".to_string(),
        })
    }
}

/// Result of executing a Cypher query.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionResult {
    /// Column names in the order returned.
    pub columns: Vec<String>,
    /// Row data, each row aligned with `columns`.
    pub rows: Vec<Vec<crate::types::PropertyValue>>,
}

impl ExecutionResult {
    /// Empty result with no columns and no rows.
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }
}

/// Execute a Cypher query against the given store. M4 will replace this stub.
pub fn execute<S>(_store: &S, _query: &CypherQuery) -> GraphResult<ExecutionResult>
where
    S: crate::store::GraphStore,
{
    Err(GraphError::CypherExec(
        "Cypher executor not implemented yet (M4)".to_string(),
    ))
}
