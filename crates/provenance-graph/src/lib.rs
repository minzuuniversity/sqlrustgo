//! Provenance Knowledge Graph for SQLRustGo GMP
//!
//! Tracks the complete lineage of GMP entities:
//! - Equipment → Operator → SOP → Batch → Workflow → Deviation → CAPA → Signature

pub mod edges;
pub mod graph;
pub mod nodes;
pub mod query;

pub use edges::{Edge, EdgeType};
pub use graph::ProvenanceGraph;
pub use nodes::{GMPNode, NodeType};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_node_new() {
        let node = ProvenanceNode::new("batch-001", NodeType::Batch);
        assert_eq!(node.id, "batch-001");
        assert_eq!(node.node_type, NodeType::Batch);
        assert_eq!(node.properties, serde_json::json!({}));
    }

    #[test]
    fn test_provenance_node_with_properties() {
        let node = ProvenanceNode::new("batch-001", NodeType::Batch)
            .with_properties(serde_json::json!({"status": "approved"}));
        assert_eq!(node.id, "batch-001");
        assert_eq!(node.properties["status"], "approved");
    }

    #[test]
    fn test_provenance_result_empty() {
        let result = ProvenanceResult {
            path: vec![],
            nodes: vec![],
            edges: vec![],
        };
        assert!(result.path.is_empty());
        assert!(result.nodes.is_empty());
        assert!(result.edges.is_empty());
    }

    #[test]
    fn test_provenance_result_with_data() {
        let node = ProvenanceNode::new("batch-001", NodeType::Batch);
        let edge = Edge {
            from: "batch-001".to_string(),
            to: "sig-001".to_string(),
            edge_type: EdgeType::SignedBy,
        };
        let result = ProvenanceResult {
            path: vec!["batch-001".to_string()],
            nodes: vec![node],
            edges: vec![edge],
        };
        assert_eq!(result.path.len(), 1);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.edges.len(), 1);
    }
}
