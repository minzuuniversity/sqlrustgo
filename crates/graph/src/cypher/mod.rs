//! Cypher query language support.
//!
//! M4 implements a hand-written recursive-descent parser and an iterative
//! executor for a substantial Cypher subset:
//!
//! * `MATCH` (with optional variable bindings, labels, and inline property filters)
//! * `OPTIONAL MATCH` (left-join semantics; unmatched RHS binds to NULL)
//! * `WHERE` (arbitrary predicate over bound variables)
//! * `RETURN` / `WITH` with `DISTINCT` / `SKIP` / `LIMIT`
//! * `UNION` / `UNION ALL`
//! * Multi-hop patterns: `(a)-[r]->(b)-[r2]->(c)`
//! * `count()` aggregation in WITH/RETURN projections
//! * Property access via dot: `n.name`
//!
//! Out of scope (not implemented): write clauses (CREATE/MERGE/DELETE/SET),
//! variable-length paths, most aggregate functions, CALL procedures, UNWIND,
//! arithmetic beyond comparison.
//!
//! ## Example
//!
//! ```
//! use sqlrustgo_graph::{InMemoryGraphStore, GraphStore, PropertyMap, PropertyValue};
//! use sqlrustgo_graph::{execute, parse};
//!
//! let s = InMemoryGraphStore::new();
//! let mut pm = PropertyMap::new();
//! pm.insert("name", "Alice");
//! pm.insert("age", 30_i64);
//! let a = s.create_node(vec!["Person".into()], pm).unwrap();
//! let mut pm = PropertyMap::new();
//! pm.insert("name", "Bob");
//! pm.insert("age", 25_i64);
//! let b = s.create_node(vec!["Person".into()], pm).unwrap();
//! s.create_edge(a, b, "KNOWS".into(), PropertyMap::new()).unwrap();
//!
//! let q = parse("MATCH (n:Person) WHERE n.age >= 25 RETURN n.name AS name").unwrap();
//! let result = execute(&s, &q).unwrap();
//! assert_eq!(result.columns, vec!["name".to_string()]);
//! assert_eq!(result.rows.len(), 2);
//! ```

pub mod ast;
pub mod executor;
pub mod lexer;
pub mod parser;

use crate::store::GraphStore;
use crate::types::{GraphError, GraphResult, PropertyValue};

/// Parse a Cypher query from source.
pub fn parse(source: &str) -> GraphResult<ast::Query> {
    parser::parse(source)
}

/// Result of executing a Cypher query.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionResult {
    /// Column names in the order returned.
    pub columns: Vec<String>,
    /// Row data, each row aligned with `columns`.
    pub rows: Vec<Vec<PropertyValue>>,
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

/// Execute a parsed Cypher query against the given store.
pub fn execute<S: GraphStore>(store: &S, query: &ast::Query) -> GraphResult<ExecutionResult> {
    executor::execute(store, query)
}

// Convenience: turn a "no rows matched" condition into a clearer error.
impl From<GraphError> for ExecutionResult {
    fn from(_: GraphError) -> Self {
        Self::empty()
    }
}
