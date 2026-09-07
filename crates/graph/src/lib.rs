//! SQLRustGo Graph Engine
//!
//! A property graph store with Cypher query language support.
//! Provides two storage backends:
//!
//! - [`InMemoryGraphStore`] — fast, non-persistent, suitable for tests and small graphs.
//! - [`DiskGraphStore`] — disk-backed with write-ahead log (WAL) for durability,
//!   suitable for production use. Uses `sqlrustgo_storage` for page-level persistence.
//!
//! Cypher language support is provided by the [`cypher`] module:
//! - `MATCH (n)-[r]->(m) WHERE ... RETURN n, m, r LIMIT n`
//! - `OPTIONAL MATCH`
//! - `UNION` / `UNION ALL`
//! - Subqueries (via `WITH` clauses and nested patterns)
//!
//! ## Example
//!
//! M1 has no in-memory store yet; full usage example will land with M2.
//!
//! ```
//! use sqlrustgo_graph::{NodeId, PropertyMap, PropertyValue};
//! let id = NodeId(42);
//! let mut props = PropertyMap::new();
//! props.insert("name", "Alice");
//! assert_eq!(props.get("name"), Some(&PropertyValue::String("Alice".into())));
//! assert_eq!(id.raw(), 42);
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod cypher;
pub mod disk_store;
pub mod store;
pub mod types;
pub mod wal;

pub use cypher::{execute, CypherQuery, ExecutionResult};
pub use disk_store::{DiskGraphStore, SyncMode};
pub use store::{Edge, GraphStore, InMemoryGraphStore, Node};
pub use types::{EdgeId, GraphError, GraphResult, Label, NodeId, PropertyMap, PropertyValue};
