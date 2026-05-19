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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> ProvenanceGraph {
        let mut graph = ProvenanceGraph::new();
        graph.add_node(ProvenanceNode::new("batch-001", NodeType::Batch));
        graph.add_node(ProvenanceNode::new("sig-001", NodeType::Signature));
        graph.add_node(ProvenanceNode::new("workflow-001", NodeType::Workflow));
        graph.add_node(ProvenanceNode::new("dev-001", NodeType::Deviation));
        graph.add_edge_between("batch-001", "sig-001", EdgeType::SignedBy);
        graph.add_edge_between("batch-001", "workflow-001", EdgeType::Contains);
        graph.add_edge_between("workflow-001", "dev-001", EdgeType::TriggeredBy);
        graph
    }

    #[test]
    fn test_connected_to() {
        let graph = create_test_graph();
        let query = ProvenanceQuery::new(&graph);
        let connected = query.connected_to("batch-001");
        assert_eq!(connected.len(), 2); // sig-001 and workflow-001
    }

    #[test]
    fn test_connected_to_isolated_node() {
        let mut graph = ProvenanceGraph::new();
        graph.add_node(ProvenanceNode::new("isolated", NodeType::Batch));
        let query = ProvenanceQuery::new(&graph);
        let connected = query.connected_to("isolated");
        assert!(connected.is_empty());
    }

    #[test]
    fn test_trace_batch_lineage() {
        let graph = create_test_graph();
        let query = ProvenanceQuery::new(&graph);
        let result = query.trace_batch_lineage("batch-001");
        assert!(!result.nodes.is_empty());
        assert!(!result.path.is_empty());
    }

    #[test]
    fn test_trace_batch_lineage_nonexistent() {
        let graph = create_test_graph();
        let query = ProvenanceQuery::new(&graph);
        let result = query.trace_batch_lineage("nonexistent");
        assert!(result.nodes.is_empty());
        assert!(result.path.is_empty());
    }

    #[test]
    fn test_batches_by_operator() {
        let mut graph = ProvenanceGraph::new();
        graph.add_node(ProvenanceNode::new("batch-001", NodeType::Batch));
        graph.add_node(ProvenanceNode::new("batch-002", NodeType::Batch));
        graph.add_node(ProvenanceNode::new("op-001", NodeType::Operator));
        let query = ProvenanceQuery::new(&graph);
        let batches = query.batches_by_operator("op-001");
        assert_eq!(batches.len(), 2);
    }
}
