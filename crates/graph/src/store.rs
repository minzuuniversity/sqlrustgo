//! In-memory graph store implementation.
//!
//! Thread-safe via `parking_lot::RwLock` (cheap read, exclusive write).
//! Suitable for tests, small graphs (< ~1M nodes), and as a reference
//! implementation that the disk-backed store is benchmarked against.

use parking_lot::RwLock;

use crate::types::{EdgeId, GraphError, GraphResult, Label, NodeId, PropertyMap};

/// A node stored in the graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Stable unique id within the store.
    pub id: NodeId,
    /// Labels classifying the node.
    pub labels: Vec<Label>,
    /// User-defined properties.
    pub properties: PropertyMap,
}

impl Node {
    /// True if this node carries the given label.
    pub fn has_label(&self, label: &Label) -> bool {
        self.labels.iter().any(|l| l == label)
    }
}

/// An edge stored in the graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// Stable unique id within the store.
    pub id: EdgeId,
    /// Source endpoint.
    pub source: NodeId,
    /// Target endpoint.
    pub target: NodeId,
    /// Relationship type (e.g. "KNOWS").
    pub rel_type: Label,
    /// User-defined properties.
    pub properties: PropertyMap,
}

/// Core graph store abstraction.
///
/// All implementations must be `Send + Sync` so they can be shared across threads.
pub trait GraphStore: Send + Sync {
    /// Creates a node, returning its assigned id.
    fn create_node(&self, labels: Vec<Label>, properties: PropertyMap) -> GraphResult<NodeId>;
    /// Creates an edge between two existing nodes.
    fn create_edge(
        &self,
        source: NodeId,
        target: NodeId,
        rel_type: Label,
        properties: PropertyMap,
    ) -> GraphResult<EdgeId>;

    /// Fetches a node by id.
    fn get_node(&self, id: NodeId) -> GraphResult<Node>;
    /// Fetches an edge by id.
    fn get_edge(&self, id: EdgeId) -> GraphResult<Edge>;

    /// Replaces a node's labels and properties.
    fn update_node(
        &self,
        id: NodeId,
        labels: Vec<Label>,
        properties: PropertyMap,
    ) -> GraphResult<()>;
    /// Replaces an edge's properties.
    fn update_edge(&self, id: EdgeId, properties: PropertyMap) -> GraphResult<()>;

    /// Deletes a node and all edges incident to it.
    fn delete_node(&self, id: NodeId) -> GraphResult<()>;
    /// Deletes a single edge.
    fn delete_edge(&self, id: EdgeId) -> GraphResult<()>;

    /// Counts nodes.
    fn node_count(&self) -> u64;
    /// Counts edges.
    fn edge_count(&self) -> u64;

    /// Returns all node ids. Order is unspecified.
    fn all_node_ids(&self) -> Vec<NodeId>;
    /// Returns all edge ids.
    fn all_edge_ids(&self) -> Vec<EdgeId>;

    /// Returns all edges where the given node is the source.
    fn outgoing_edges(&self, node: NodeId) -> GraphResult<Vec<Edge>>;
    /// Returns all edges where the given node is the target.
    fn incoming_edges(&self, node: NodeId) -> GraphResult<Vec<Edge>>;
}

/// Thread-safe in-memory graph store.
///
/// Ids are allocated monotonically and never reused within the lifetime of one store.
/// Deleting a node/edge does NOT reclaim its id, which simplifies concurrent code
/// (a reader cannot observe an id collision).
#[derive(Debug, Default)]
pub struct InMemoryGraphStore {
    inner: RwLock<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    nodes: std::collections::HashMap<NodeId, Node>,
    edges: std::collections::HashMap<EdgeId, Edge>,
    next_node_id: u64,
    next_edge_id: u64,
}

impl InMemoryGraphStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a node with a caller-chosen id, ignoring the internal counter.
    ///
    /// Intended for stores (e.g. `DiskGraphStore`) that maintain their own
    /// monotonic id allocation and need to replay WAL records whose node ids
    /// were chosen at write time. If a node with the same id already exists,
    /// returns [`GraphError::NodeNotFound`] — caller should check first.
    ///
    /// Does **not** advance the internal counter; the caller is expected to
    /// drive id allocation itself.
    pub fn insert_node_with_id(
        &self,
        id: NodeId,
        labels: Vec<Label>,
        properties: PropertyMap,
    ) -> GraphResult<()> {
        let mut g = self.inner.write();
        if g.nodes.contains_key(&id) {
            return Err(GraphError::NodeNotFound(id));
        }
        g.nodes.insert(
            id,
            Node {
                id,
                labels,
                properties,
            },
        );
        Ok(())
    }

    /// Inserts an edge with a caller-chosen id.
    pub fn insert_edge_with_id(
        &self,
        id: EdgeId,
        source: NodeId,
        target: NodeId,
        rel_type: Label,
        properties: PropertyMap,
    ) -> GraphResult<()> {
        let mut g = self.inner.write();
        if g.edges.contains_key(&id) {
            return Err(GraphError::EdgeNotFound(id));
        }
        if !g.nodes.contains_key(&source) || !g.nodes.contains_key(&target) {
            return Err(GraphError::EdgeEndpointMissing {
                src: source,
                dst: target,
            });
        }
        g.edges.insert(
            id,
            Edge {
                id,
                source,
                target,
                rel_type,
                properties,
            },
        );
        Ok(())
    }
}

impl GraphStore for InMemoryGraphStore {
    fn create_node(&self, labels: Vec<Label>, properties: PropertyMap) -> GraphResult<NodeId> {
        let mut g = self.inner.write();
        let id = NodeId(g.next_node_id);
        g.next_node_id = g
            .next_node_id
            .checked_add(1)
            .ok_or_else(|| GraphError::Storage("node id space exhausted".into()))?;
        g.nodes.insert(
            id,
            Node {
                id,
                labels,
                properties,
            },
        );
        Ok(id)
    }

    fn create_edge(
        &self,
        source: NodeId,
        target: NodeId,
        rel_type: Label,
        properties: PropertyMap,
    ) -> GraphResult<EdgeId> {
        let mut g = self.inner.write();
        if !g.nodes.contains_key(&source) || !g.nodes.contains_key(&target) {
            return Err(GraphError::EdgeEndpointMissing {
                src: source,
                dst: target,
            });
        }
        let id = EdgeId(g.next_edge_id);
        g.next_edge_id = g
            .next_edge_id
            .checked_add(1)
            .ok_or_else(|| GraphError::Storage("edge id space exhausted".into()))?;
        g.edges.insert(
            id,
            Edge {
                id,
                source,
                target,
                rel_type,
                properties,
            },
        );
        Ok(id)
    }

    fn get_node(&self, id: NodeId) -> GraphResult<Node> {
        self.inner
            .read()
            .nodes
            .get(&id)
            .cloned()
            .ok_or(GraphError::NodeNotFound(id))
    }

    fn get_edge(&self, id: EdgeId) -> GraphResult<Edge> {
        self.inner
            .read()
            .edges
            .get(&id)
            .cloned()
            .ok_or(GraphError::EdgeNotFound(id))
    }

    fn update_node(
        &self,
        id: NodeId,
        labels: Vec<Label>,
        properties: PropertyMap,
    ) -> GraphResult<()> {
        let mut g = self.inner.write();
        let node = g.nodes.get_mut(&id).ok_or(GraphError::NodeNotFound(id))?;
        node.labels = labels;
        node.properties = properties;
        Ok(())
    }

    fn update_edge(&self, id: EdgeId, properties: PropertyMap) -> GraphResult<()> {
        let mut g = self.inner.write();
        let edge = g.edges.get_mut(&id).ok_or(GraphError::EdgeNotFound(id))?;
        edge.properties = properties;
        Ok(())
    }

    fn delete_node(&self, id: NodeId) -> GraphResult<()> {
        let mut g = self.inner.write();
        if g.nodes.remove(&id).is_none() {
            return Err(GraphError::NodeNotFound(id));
        }
        // Cascade: remove all incident edges.
        g.edges.retain(|_, e| e.source != id && e.target != id);
        Ok(())
    }

    fn delete_edge(&self, id: EdgeId) -> GraphResult<()> {
        let mut g = self.inner.write();
        if g.edges.remove(&id).is_none() {
            return Err(GraphError::EdgeNotFound(id));
        }
        Ok(())
    }

    fn node_count(&self) -> u64 {
        self.inner.read().nodes.len() as u64
    }

    fn edge_count(&self) -> u64 {
        self.inner.read().edges.len() as u64
    }

    fn all_node_ids(&self) -> Vec<NodeId> {
        self.inner.read().nodes.keys().copied().collect()
    }

    fn all_edge_ids(&self) -> Vec<EdgeId> {
        self.inner.read().edges.keys().copied().collect()
    }

    fn outgoing_edges(&self, node: NodeId) -> GraphResult<Vec<Edge>> {
        let g = self.inner.read();
        if !g.nodes.contains_key(&node) {
            return Err(GraphError::NodeNotFound(node));
        }
        Ok(g.edges
            .values()
            .filter(|e| e.source == node)
            .cloned()
            .collect())
    }

    fn incoming_edges(&self, node: NodeId) -> GraphResult<Vec<Edge>> {
        let g = self.inner.read();
        if !g.nodes.contains_key(&node) {
            return Err(GraphError::NodeNotFound(node));
        }
        Ok(g.edges
            .values()
            .filter(|e| e.target == node)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PropertyValue;

    fn store_with_two_nodes() -> (InMemoryGraphStore, NodeId, NodeId) {
        let s = InMemoryGraphStore::new();
        let a = s
            .create_node(
                vec!["Person".into()],
                PropertyMap::from_pairs(vec![("name", "Alice")]),
            )
            .unwrap();
        let b = s
            .create_node(
                vec!["Person".into()],
                PropertyMap::from_pairs(vec![("name", "Bob")]),
            )
            .unwrap();
        (s, a, b)
    }

    #[test]
    fn create_and_get_node() {
        let (s, a, _b) = store_with_two_nodes();
        let node = s.get_node(a).unwrap();
        assert_eq!(node.id, a);
        assert!(node.has_label(&"Person".into()));
        assert_eq!(
            node.properties.get("name"),
            Some(&PropertyValue::String("Alice".into()))
        );
    }

    #[test]
    fn get_unknown_node_returns_error() {
        let s = InMemoryGraphStore::new();
        let err = s.get_node(NodeId(999)).unwrap_err();
        assert!(matches!(err, GraphError::NodeNotFound(NodeId(999))));
    }

    #[test]
    fn create_edge_requires_existing_endpoints() {
        let s = InMemoryGraphStore::new();
        let err = s
            .create_edge(NodeId(1), NodeId(2), "X".into(), PropertyMap::new())
            .unwrap_err();
        assert!(matches!(err, GraphError::EdgeEndpointMissing { .. }));
    }

    #[test]
    fn create_edge_and_query_incoming_outgoing() {
        let (s, a, b) = store_with_two_nodes();
        let e = s
            .create_edge(a, b, "KNOWS".into(), PropertyMap::new())
            .unwrap();
        let outgoing = s.outgoing_edges(a).unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].id, e);
        let incoming = s.incoming_edges(b).unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].id, e);
        let back = s.incoming_edges(a).unwrap();
        assert_eq!(back.len(), 0);
    }

    #[test]
    fn delete_node_cascades_to_edges() {
        let (s, a, b) = store_with_two_nodes();
        let _ = s
            .create_edge(a, b, "KNOWS".into(), PropertyMap::new())
            .unwrap();
        assert_eq!(s.edge_count(), 1);
        s.delete_node(a).unwrap();
        assert_eq!(s.node_count(), 1);
        assert_eq!(s.edge_count(), 0);
    }

    #[test]
    fn update_node_replaces_labels_and_props() {
        let (s, a, _) = store_with_two_nodes();
        s.update_node(
            a,
            vec!["Person".into(), "Friend".into()],
            PropertyMap::from_pairs(vec![("nickname", "Al")]),
        )
        .unwrap();
        let n = s.get_node(a).unwrap();
        assert_eq!(n.labels.len(), 2);
        assert_eq!(
            n.properties.get("nickname"),
            Some(&PropertyValue::String("Al".into()))
        );
        assert!(n.properties.get("name").is_none()); // replaced
    }

    #[test]
    fn all_node_ids_and_edge_ids_match_counts() {
        let (s, a, b) = store_with_two_nodes();
        s.create_edge(a, b, "KNOWS".into(), PropertyMap::new())
            .unwrap();
        assert_eq!(s.all_node_ids().len() as u64, s.node_count());
        assert_eq!(s.all_edge_ids().len() as u64, s.edge_count());
    }

    #[test]
    fn ids_are_monotonic_and_not_reused_after_delete() {
        let s = InMemoryGraphStore::new();
        let a = s.create_node(vec![], PropertyMap::new()).unwrap();
        let b = s.create_node(vec![], PropertyMap::new()).unwrap();
        assert!(b.raw() > a.raw());
        s.delete_node(a).unwrap();
        let c = s.create_node(vec![], PropertyMap::new()).unwrap();
        assert!(c.raw() > b.raw(), "ids must not be reused");
    }
}
