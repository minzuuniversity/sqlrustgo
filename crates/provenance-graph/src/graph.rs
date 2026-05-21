//! ProvenanceGraph - in-memory graph store

use crate::{Edge, EdgeType, NodeType, ProvenanceNode};

pub struct ProvenanceGraph {
    nodes: Vec<ProvenanceNode>,
    edges: Vec<Edge>,
}

impl ProvenanceGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: ProvenanceNode) {
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    pub fn add_edge_between(&mut self, from: &str, to: &str, edge_type: EdgeType) {
        self.edges.push(Edge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type,
        });
    }

    pub fn get_node(&self, id: &str) -> Option<&ProvenanceNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn nodes_of_type(&self, node_type: NodeType) -> Vec<&ProvenanceNode> {
        self.nodes
            .iter()
            .filter(|n| n.node_type == node_type)
            .collect()
    }

    pub fn outgoing_edges(&self, node_id: &str) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    pub fn incoming_edges(&self, node_id: &str) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for ProvenanceGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_graph_new() {
        let graph = ProvenanceGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = ProvenanceGraph::new();
        let node = ProvenanceNode::new("node1", NodeType::Batch);
        graph.add_node(node);
        assert_eq!(graph.node_count(), 1);
        assert!(graph.get_node("node1").is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut graph = ProvenanceGraph::new();
        let edge = Edge {
            from: "node1".to_string(),
            to: "node2".to_string(),
            edge_type: EdgeType::Contains,
        };
        graph.add_edge(edge);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_between() {
        let mut graph = ProvenanceGraph::new();
        graph.add_edge_between("n1", "n2", EdgeType::OperatedBy);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_get_node() {
        let mut graph = ProvenanceGraph::new();
        let node = ProvenanceNode::new("batch-001", NodeType::Batch);
        graph.add_node(node);
        let found = graph.get_node("batch-001");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "batch-001");
    }

    #[test]
    fn test_get_node_not_found() {
        let graph = ProvenanceGraph::new();
        assert!(graph.get_node("nonexistent").is_none());
    }

    #[test]
    fn test_nodes_of_type() {
        let mut graph = ProvenanceGraph::new();
        graph.add_node(ProvenanceNode::new("batch-001", NodeType::Batch));
        graph.add_node(ProvenanceNode::new("batch-002", NodeType::Batch));
        graph.add_node(ProvenanceNode::new("op-001", NodeType::Operator));
        let batches = graph.nodes_of_type(NodeType::Batch);
        assert_eq!(batches.len(), 2);
    }

    #[test]
    fn test_outgoing_edges() {
        let mut graph = ProvenanceGraph::new();
        graph.add_node(ProvenanceNode::new("batch-001", NodeType::Batch));
        graph.add_node(ProvenanceNode::new("sig-001", NodeType::Signature));
        graph.add_edge_between("batch-001", "sig-001", EdgeType::SignedBy);
        let outgoing = graph.outgoing_edges("batch-001");
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].to, "sig-001");
    }

    #[test]
    fn test_incoming_edges() {
        let mut graph = ProvenanceGraph::new();
        graph.add_node(ProvenanceNode::new("batch-001", NodeType::Batch));
        graph.add_node(ProvenanceNode::new("sig-001", NodeType::Signature));
        graph.add_edge_between("batch-001", "sig-001", EdgeType::SignedBy);
        let incoming = graph.incoming_edges("sig-001");
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].from, "batch-001");
    }
}
