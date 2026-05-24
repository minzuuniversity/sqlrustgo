use sqlrustgo_provenance_graph::{
    Edge, EdgeType, NodeType, ProvenanceGraph, ProvenanceNode, ProvenanceResult,
};

#[test]
fn test_provenance_node_new() {
    let node = ProvenanceNode::new("node_001", NodeType::Operator);
    assert_eq!(node.id, "node_001");
    assert_eq!(node.node_type, NodeType::Operator);
    assert_eq!(node.properties, serde_json::json!({}));
}

#[test]
fn test_provenance_node_with_properties() {
    let node = ProvenanceNode::new("node_001", NodeType::Equipment)
        .with_properties(serde_json::json!({"name": "Scale A"}));
    assert_eq!(node.id, "node_001");
    assert_eq!(node.properties["name"], "Scale A");
}

#[test]
fn test_node_type_variants() {
    assert_eq!(NodeType::Equipment.as_str(), "equipment");
    assert_eq!(NodeType::Operator.as_str(), "operator");
    assert_eq!(NodeType::SOP.as_str(), "sop");
    assert_eq!(NodeType::Batch.as_str(), "batch");
    assert_eq!(NodeType::Workflow.as_str(), "workflow");
    assert_eq!(NodeType::Signature.as_str(), "signature");
}

#[test]
fn test_edge_type() {
    assert_eq!(EdgeType::OperatedBy.as_str(), "operated_by");
    assert_eq!(EdgeType::FollowsSOP.as_str(), "follows_sop");
    assert_eq!(EdgeType::SignedBy.as_str(), "signed_by");
    assert_eq!(EdgeType::CorrectedBy.as_str(), "corrected_by");
}

#[test]
fn test_provenance_graph_new() {
    let graph = ProvenanceGraph::new();
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn test_provenance_result() {
    let result = ProvenanceResult {
        path: vec!["node1".to_string(), "node2".to_string()],
        nodes: vec![],
        edges: vec![],
    };
    assert_eq!(result.path.len(), 2);
}

#[test]
fn test_edge_creation() {
    let edge = Edge {
        from: "node1".to_string(),
        to: "node2".to_string(),
        edge_type: EdgeType::OperatedBy,
    };
    assert_eq!(edge.from, "node1");
    assert_eq!(edge.to, "node2");
    assert_eq!(edge.edge_type, EdgeType::OperatedBy);
}
