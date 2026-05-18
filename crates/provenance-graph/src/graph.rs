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
        self.nodes.iter().filter(|n| n.node_type == node_type).collect()
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
