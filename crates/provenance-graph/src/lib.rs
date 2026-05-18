//! Provenance Knowledge Graph for SQLRustGo GMP
//!
//! Tracks the complete lineage of GMP entities:
//! - Equipment → Operator → SOP → Batch → Workflow → Deviation → CAPA → Signature

pub mod graph;
pub mod nodes;
pub mod edges;
pub mod query;

pub use graph::ProvenanceGraph;
pub use nodes::{NodeType, GMPNode};
pub use edges::{Edge, EdgeType};
pub use query::ProvenanceQuery;

/// A node in the provenance graph
#[derive(Debug, Clone)]
pub struct ProvenanceNode {
    pub id: String,
    pub node_type: NodeType,
    pub properties: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ProvenanceNode {
    pub fn new(id: &str, node_type: NodeType) -> Self {
        Self {
            id: id.to_string(),
            node_type,
            properties: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_properties(mut self, props: serde_json::Value) -> Self {
        self.properties = props;
        self
    }
}

/// Query result from provenance traversal
#[derive(Debug, Clone)]
pub struct ProvenanceResult {
    pub path: Vec<String>,
    pub nodes: Vec<ProvenanceNode>,
    pub edges: Vec<Edge>,
}
