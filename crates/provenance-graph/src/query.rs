//! Provenance query engine

use crate::{Edge, EdgeType, NodeType, ProvenanceGraph, ProvenanceNode, ProvenanceResult};

pub struct ProvenanceQuery<'a> {
    graph: &'a ProvenanceGraph,
}

impl<'a> ProvenanceQuery<'a> {
    pub fn new(graph: &'a ProvenanceGraph) -> Self {
        Self { graph }
    }

    /// Find all nodes connected to a given node
    pub fn connected_to(&self, node_id: &str) -> Vec<&ProvenanceNode> {
        let mut results = Vec::new();
        for edge in self.graph.outgoing_edges(node_id) {
            if let Some(node) = self.graph.get_node(&edge.to) {
                results.push(node);
            }
        }
        for edge in self.graph.incoming_edges(node_id) {
            if let Some(node) = self.graph.get_node(&edge.from) {
                results.push(node);
            }
        }
        results
    }

    /// Trace complete lineage of a batch
    pub fn trace_batch_lineage(&self, batch_id: &str) -> ProvenanceResult {
        let mut visited = std::collections::HashSet::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        self.dfs_traverse(batch_id, &mut visited, &mut nodes, &mut edges);

        let path: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        ProvenanceResult { path, nodes, edges }
    }

    fn dfs_traverse(
        &self,
        node_id: &str,
        visited: &mut std::collections::HashSet<String>,
        nodes: &mut Vec<ProvenanceNode>,
        edges: &mut Vec<Edge>,
    ) {
        if visited.contains(node_id) {
            return;
        }
        visited.insert(node_id.to_string());

        if let Some(node) = self.graph.get_node(node_id) {
            nodes.push(node.clone());
        }

        for edge in self.graph.outgoing_edges(node_id) {
            edges.push(edge.clone());
            self.dfs_traverse(&edge.to, visited, nodes, edges);
        }
    }

    /// Find all batches approved by a specific operator
    pub fn batches_by_operator(&self, operator_id: &str) -> Vec<&ProvenanceNode> {
        self.graph.nodes_of_type(NodeType::Batch)
    }
}
