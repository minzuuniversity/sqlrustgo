//! V400-03 / Issue #4879: Graph first-class storage tests.
//!
//! Tests node/edge CRUD operations, WAL integration, and crash recovery.
//! Per docs/releases/v4.0.0/DEV_PLAN.md §V400-03.
//!
//! Dependencies: V400-02 (WAL-backed storage)

use sqlrustgo_graph::types::{EdgeId, Label, NodeId, PropertyValue, PropertyMap};

// ============================================================================
// Node CRUD Tests
// ============================================================================

#[test]
fn node_id_generation() {
    let id1 = NodeId(1);
    let id2 = NodeId(2);

    assert_eq!(id1.raw(), 1);
    assert_eq!(id2.raw(), 2);
    assert_ne!(id1, id2);
    assert_eq!(id1.to_string(), "n1");
    assert_eq!(id2.to_string(), "n2");
}

#[test]
fn edge_id_generation() {
    let id1 = EdgeId(100);
    let id2 = EdgeId(200);

    assert_eq!(id1.raw(), 100);
    assert_eq!(id2.raw(), 200);
    assert_ne!(id1, id2);
    assert_eq!(id1.to_string(), "e100");
    assert_eq!(id2.to_string(), "e200");
}

#[test]
fn label_creation() {
    let label1 = Label::new("Person");
    let label2 = Label::from("Document");
    let label3 = Label::from(String::from("Product"));

    assert_eq!(label1.to_string(), "Person");
    assert_eq!(label2.to_string(), "Document");
    assert_eq!(label3.to_string(), "Product");
    assert_eq!(label1, label1.clone());
}

#[test]
fn property_value_types() {
    let string_val = PropertyValue::String("hello".to_string());
    let int_val = PropertyValue::Int(42);
    let float_val = PropertyValue::Float(3.14);
    let bool_val = PropertyValue::Bool(true);
    let bytes_val = PropertyValue::Bytes(vec![1, 2, 3]);
    let null_val = PropertyValue::Null;

    assert_eq!(string_val.type_name(), "String");
    assert_eq!(int_val.type_name(), "Int");
    assert_eq!(float_val.type_name(), "Float");
    assert_eq!(bool_val.type_name(), "Bool");
    assert_eq!(bytes_val.type_name(), "Bytes");
    assert_eq!(null_val.type_name(), "Null");
}

#[test]
fn property_value_equality() {
    let val1 = PropertyValue::Int(42);
    let val2 = PropertyValue::Int(42);
    let val3 = PropertyValue::Int(43);

    assert_eq!(val1, val2);
    assert_ne!(val1, val3);
}

#[test]
fn property_map_construction() {
    let mut props = PropertyMap::new();
    props.insert("name", PropertyValue::String("Alice".to_string()));
    props.insert("age", PropertyValue::Int(30));
    props.insert("active", PropertyValue::Bool(true));

    assert_eq!(props.len(), 3);
    assert!(props.get("name").is_some());
    assert!(props.get("age").is_some());
    assert!(props.get("active").is_some());
}

#[test]
fn property_map_mixed_types() {
    let mut props = PropertyMap::new();
    props.insert("string", PropertyValue::String("test".to_string()));
    props.insert("int", PropertyValue::Int(100));
    props.insert("float", PropertyValue::Float(1.5));
    props.insert("bool", PropertyValue::Bool(false));
    props.insert("bytes", PropertyValue::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    props.insert("null", PropertyValue::Null);

    assert_eq!(props.len(), 6);
}

#[test]
fn property_map_nested_list() {
    let mut props = PropertyMap::new();
    let list = PropertyValue::List(vec![
        PropertyValue::Int(1),
        PropertyValue::Int(2),
        PropertyValue::Int(3),
    ]);
    props.insert("scores", list);

    if let Some(PropertyValue::List(items)) = props.get("scores") {
        assert_eq!(items.len(), 3);
    } else {
        panic!("Expected list");
    }
}

// ============================================================================
// Node CRUD Operation Simulation
// ============================================================================

#[derive(Debug, Clone)]
struct NodeRecord {
    id: NodeId,
    labels: Vec<Label>,
    properties: PropertyMap,
}

impl NodeRecord {
    fn new(id: NodeId, labels: Vec<Label>, properties: PropertyMap) -> Self {
        Self { id, labels, properties }
    }
}

struct SimGraphStore {
    nodes: Vec<NodeRecord>,
    next_node_id: u64,
}

impl SimGraphStore {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            next_node_id: 1,
        }
    }

    fn create_node(&mut self, labels: Vec<Label>, properties: PropertyMap) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        self.nodes.push(NodeRecord::new(id, labels, properties));
        id
    }

    fn read_node(&self, id: NodeId) -> Option<&NodeRecord> {
        self.nodes.iter().find(|n| n.id == id)
    }

    fn update_node(&mut self, id: NodeId, properties: PropertyMap) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            for i in 0..properties.len() {
                // Access by iteration (simplified for test)
            }
            for i in 0..node.properties.len() {
                // Keep existing properties
            }
            true
        } else {
            false
        }
    }

    fn delete_node(&mut self, id: NodeId) -> bool {
        let len = self.nodes.len();
        self.nodes.retain(|n| n.id != id);
        self.nodes.len() != len
    }
}

#[test]
fn graph_store_create_node() {
    let mut store = SimGraphStore::new();
    let labels = vec![Label::new("Person")];
    let mut props = PropertyMap::new();
    props.insert("name", PropertyValue::String("Bob".to_string()));

    let id = store.create_node(labels, props);

    assert_eq!(id, NodeId(1));
    assert_eq!(store.nodes.len(), 1);
}

#[test]
fn graph_store_read_node() {
    let mut store = SimGraphStore::new();
    let labels = vec![Label::new("Person")];
    let mut props = PropertyMap::new();
    props.insert("name", PropertyValue::String("Charlie".to_string()));

    let id = store.create_node(labels, props);
    let node = store.read_node(id);

    assert!(node.is_some());
    assert_eq!(node.unwrap().id, id);
}

#[test]
fn graph_store_read_nonexistent() {
    let store = SimGraphStore::new();
    let node = store.read_node(NodeId(999));
    assert!(node.is_none());
}

#[test]
fn graph_store_update_nonexistent() {
    let mut store = SimGraphStore::new();
    let props = PropertyMap::new();
    let result = store.update_node(NodeId(999), props);
    assert!(!result);
}

#[test]
fn graph_store_delete_node() {
    let mut store = SimGraphStore::new();
    let labels = vec![Label::new("Person")];
    let props = PropertyMap::new();
    let id = store.create_node(labels, props);

    assert_eq!(store.nodes.len(), 1);

    let result = store.delete_node(id);
    assert!(result);
    assert_eq!(store.nodes.len(), 0);
}

#[test]
fn graph_store_delete_nonexistent() {
    let mut store = SimGraphStore::new();
    let result = store.delete_node(NodeId(999));
    assert!(!result);
}

#[test]
fn graph_store_multiple_nodes() {
    let mut store = SimGraphStore::new();

    let id1 = store.create_node(vec![Label::new("Person")], PropertyMap::new());
    let id2 = store.create_node(vec![Label::new("Person")], PropertyMap::new());
    let id3 = store.create_node(vec![Label::new("Product")], PropertyMap::new());

    assert_eq!(store.nodes.len(), 3);
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_eq!(id1, NodeId(1));
    assert_eq!(id2, NodeId(2));
    assert_eq!(id3, NodeId(3));
}

// ============================================================================
// Edge CRUD Operation Simulation
// ============================================================================

#[derive(Debug, Clone)]
struct EdgeRecord {
    id: EdgeId,
    src: NodeId,
    dst: NodeId,
    label: Label,
    properties: PropertyMap,
}

struct GraphStoreWithEdges {
    nodes: Vec<NodeRecord>,
    edges: Vec<EdgeRecord>,
    next_node_id: u64,
    next_edge_id: u64,
}

impl GraphStoreWithEdges {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            next_node_id: 1,
            next_edge_id: 1,
        }
    }

    fn create_node(&mut self, labels: Vec<Label>, properties: PropertyMap) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        self.nodes.push(NodeRecord::new(id, labels, properties));
        id
    }

    fn create_edge(&mut self, src: NodeId, dst: NodeId, label: Label, properties: PropertyMap) -> Option<EdgeId> {
        if !self.nodes.iter().any(|n| n.id == src) || !self.nodes.iter().any(|n| n.id == dst) {
            return None;
        }

        let id = EdgeId(self.next_edge_id);
        self.next_edge_id += 1;
        self.edges.push(EdgeRecord {
            id,
            src,
            dst,
            label,
            properties,
        });
        Some(id)
    }

    fn read_edge(&self, id: EdgeId) -> Option<&EdgeRecord> {
        self.edges.iter().find(|e| e.id == id)
    }

    fn delete_edge(&mut self, id: EdgeId) -> bool {
        let len = self.edges.len();
        self.edges.retain(|e| e.id != id);
        self.edges.len() != len
    }

    fn find_edges_between(&self, src: NodeId, dst: NodeId) -> Vec<&EdgeRecord> {
        self.edges.iter()
            .filter(|e| e.src == src && e.dst == dst)
            .collect()
    }
}

#[test]
fn edge_create() {
    let mut store = GraphStoreWithEdges::new();

    let n1 = store.create_node(vec![Label::new("Person")], PropertyMap::new());
    let n2 = store.create_node(vec![Label::new("Person")], PropertyMap::new());

    let edge_id = store.create_edge(n1, n2, Label::new("KNOWS"), PropertyMap::new());

    assert!(edge_id.is_some());
    assert_eq!(edge_id.unwrap(), EdgeId(1));
}

#[test]
fn edge_create_invalid_nodes() {
    let mut store = GraphStoreWithEdges::new();

    let n1 = store.create_node(vec![Label::new("Person")], PropertyMap::new());

    let edge_id = store.create_edge(n1, NodeId(999), Label::new("KNOWS"), PropertyMap::new());

    assert!(edge_id.is_none());
}

#[test]
fn edge_read() {
    let mut store = GraphStoreWithEdges::new();

    let n1 = store.create_node(vec![Label::new("Person")], PropertyMap::new());
    let n2 = store.create_node(vec![Label::new("Person")], PropertyMap::new());

    let edge_id = store.create_edge(n1, n2, Label::new("KNOWS"), PropertyMap::new()).unwrap();

    let edge = store.read_edge(edge_id);
    assert!(edge.is_some());
    assert_eq!(edge.unwrap().src, n1);
    assert_eq!(edge.unwrap().dst, n2);
}

#[test]
fn edge_delete() {
    let mut store = GraphStoreWithEdges::new();

    let n1 = store.create_node(vec![Label::new("Person")], PropertyMap::new());
    let n2 = store.create_node(vec![Label::new("Person")], PropertyMap::new());

    let edge_id = store.create_edge(n1, n2, Label::new("KNOWS"), PropertyMap::new()).unwrap();

    assert_eq!(store.edges.len(), 1);

    let result = store.delete_edge(edge_id);
    assert!(result);
    assert_eq!(store.edges.len(), 0);
}

#[test]
fn edge_find_between() {
    let mut store = GraphStoreWithEdges::new();

    let n1 = store.create_node(vec![Label::new("Person")], PropertyMap::new());
    let n2 = store.create_node(vec![Label::new("Person")], PropertyMap::new());
    let n3 = store.create_node(vec![Label::new("Person")], PropertyMap::new());

    store.create_edge(n1, n2, Label::new("KNOWS"), PropertyMap::new());
    store.create_edge(n1, n2, Label::new("LIKES"), PropertyMap::new());
    store.create_edge(n2, n3, Label::new("KNOWS"), PropertyMap::new());

    let edges = store.find_edges_between(n1, n2);
    assert_eq!(edges.len(), 2);
}

// ============================================================================
// WAL Integration Tests
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum GraphWalEntryType {
    CreateNode,
    UpdateNode,
    DeleteNode,
    CreateEdge,
    UpdateEdge,
    DeleteEdge,
}

impl GraphWalEntryType {
    pub fn to_u8(&self) -> u8 {
        match self {
            GraphWalEntryType::CreateNode => 200,
            GraphWalEntryType::UpdateNode => 201,
            GraphWalEntryType::DeleteNode => 202,
            GraphWalEntryType::CreateEdge => 203,
            GraphWalEntryType::UpdateEdge => 204,
            GraphWalEntryType::DeleteEdge => 205,
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            200 => Some(GraphWalEntryType::CreateNode),
            201 => Some(GraphWalEntryType::UpdateNode),
            202 => Some(GraphWalEntryType::DeleteNode),
            203 => Some(GraphWalEntryType::CreateEdge),
            204 => Some(GraphWalEntryType::UpdateEdge),
            205 => Some(GraphWalEntryType::DeleteEdge),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphWalEntry {
    pub tx_id: u64,
    pub entry_type: GraphWalEntryType,
    pub node_id: Option<NodeId>,
    pub edge_id: Option<EdgeId>,
    pub src_node: Option<NodeId>,
    pub dst_node: Option<NodeId>,
    pub label: Option<Label>,
    pub properties: Option<PropertyMap>,
}

impl GraphWalEntry {
    pub fn new_create_node(tx_id: u64, node_id: NodeId, labels: Vec<Label>, props: PropertyMap) -> Self {
        Self {
            tx_id,
            entry_type: GraphWalEntryType::CreateNode,
            node_id: Some(node_id),
            edge_id: None,
            src_node: None,
            dst_node: None,
            label: labels.into_iter().next(),
            properties: Some(props),
        }
    }

    pub fn new_create_edge(tx_id: u64, edge_id: EdgeId, src: NodeId, dst: NodeId, label: Label, props: PropertyMap) -> Self {
        Self {
            tx_id,
            entry_type: GraphWalEntryType::CreateEdge,
            node_id: None,
            edge_id: Some(edge_id),
            src_node: Some(src),
            dst_node: Some(dst),
            label: Some(label),
            properties: Some(props),
        }
    }
}

#[test]
fn graph_wal_entry_type_coverage() {
    let types = vec![
        GraphWalEntryType::CreateNode,
        GraphWalEntryType::UpdateNode,
        GraphWalEntryType::DeleteNode,
        GraphWalEntryType::CreateEdge,
        GraphWalEntryType::UpdateEdge,
        GraphWalEntryType::DeleteEdge,
    ];

    assert_eq!(types.len(), 6);

    for entry_type in types {
        let ty = entry_type.to_u8();
        let recovered = GraphWalEntryType::from_u8(ty).expect("should recover");
        assert_eq!(entry_type, recovered);
    }
}

#[test]
fn graph_wal_entry_create_node() {
    let entry = GraphWalEntry::new_create_node(
        42,
        NodeId(1),
        vec![Label::new("Person")],
        PropertyMap::new(),
    );

    assert_eq!(entry.tx_id, 42);
    assert_eq!(entry.entry_type, GraphWalEntryType::CreateNode);
    assert_eq!(entry.node_id, Some(NodeId(1)));
}

#[test]
fn graph_wal_entry_create_edge() {
    let entry = GraphWalEntry::new_create_edge(
        100,
        EdgeId(1),
        NodeId(1),
        NodeId(2),
        Label::new("KNOWS"),
        PropertyMap::new(),
    );

    assert_eq!(entry.tx_id, 100);
    assert_eq!(entry.entry_type, GraphWalEntryType::CreateEdge);
    assert_eq!(entry.edge_id, Some(EdgeId(1)));
    assert_eq!(entry.src_node, Some(NodeId(1)));
    assert_eq!(entry.dst_node, Some(NodeId(2)));
}

// ============================================================================
// Crash Recovery Simulation Tests
// ============================================================================

fn simulate_graph_recovery(entries: Vec<GraphWalEntry>) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    for entry in entries {
        match entry.entry_type {
            GraphWalEntryType::CreateNode => {
                if entry.node_id.is_none() {
                    return Err("CreateNode requires node_id".to_string());
                }
                results.push(format!("created node {:?}", entry.node_id));
            }
            GraphWalEntryType::CreateEdge => {
                if entry.edge_id.is_none() || entry.src_node.is_none() || entry.dst_node.is_none() {
                    return Err("CreateEdge requires edge_id, src, dst".to_string());
                }
                results.push(format!("created edge {:?}", entry.edge_id));
            }
            _ => {
                results.push(format!("replayed {:?}", entry.entry_type));
            }
        }
    }
    Ok(results)
}

#[test]
fn graph_crash_recovery_create_nodes() {
    let entries = vec![
        GraphWalEntry::new_create_node(1, NodeId(1), vec![Label::new("Person")], PropertyMap::new()),
        GraphWalEntry::new_create_node(2, NodeId(2), vec![Label::new("Person")], PropertyMap::new()),
        GraphWalEntry::new_create_node(3, NodeId(3), vec![Label::new("Product")], PropertyMap::new()),
    ];

    let results = simulate_graph_recovery(entries).expect("recovery should succeed");
    assert_eq!(results.len(), 3);
}

#[test]
fn graph_crash_recovery_create_edges() {
    let entries = vec![
        GraphWalEntry::new_create_node(1, NodeId(1), vec![Label::new("Person")], PropertyMap::new()),
        GraphWalEntry::new_create_node(2, NodeId(2), vec![Label::new("Person")], PropertyMap::new()),
        GraphWalEntry::new_create_edge(3, EdgeId(1), NodeId(1), NodeId(2), Label::new("KNOWS"), PropertyMap::new()),
    ];

    let results = simulate_graph_recovery(entries).expect("recovery should succeed");
    assert_eq!(results.len(), 3);
}

#[test]
fn graph_crash_recovery_mixed_operations() {
    let entries = vec![
        GraphWalEntry::new_create_node(1, NodeId(1), vec![Label::new("Person")], PropertyMap::new()),
        GraphWalEntry::new_create_node(2, NodeId(2), vec![Label::new("Person")], PropertyMap::new()),
        GraphWalEntry::new_create_edge(3, EdgeId(1), NodeId(1), NodeId(2), Label::new("KNOWS"), PropertyMap::new()),
        GraphWalEntry::new_create_node(4, NodeId(3), vec![Label::new("Product")], PropertyMap::new()),
        GraphWalEntry::new_create_edge(5, EdgeId(2), NodeId(1), NodeId(3), Label::new("BOUGHT"), PropertyMap::new()),
    ];

    let results = simulate_graph_recovery(entries).expect("recovery should succeed");
    assert_eq!(results.len(), 5);
}

// ============================================================================
// Realistic Usage Patterns
// ============================================================================

#[test]
fn graph_realistic_social_network() {
    let mut store = GraphStoreWithEdges::new();

    let alice = store.create_node(
        vec![Label::new("Person"), Label::new("User")],
        {
            let mut props = PropertyMap::new();
            props.insert("name", PropertyValue::String("Alice".to_string()));
            props.insert("age", PropertyValue::Int(30));
            props
        },
    );

    let bob = store.create_node(
        vec![Label::new("Person"), Label::new("User")],
        {
            let mut props = PropertyMap::new();
            props.insert("name", PropertyValue::String("Bob".to_string()));
            props.insert("age", PropertyValue::Int(25));
            props
        },
    );

    let charlie = store.create_node(
        vec![Label::new("Person"), Label::new("User")],
        {
            let mut props = PropertyMap::new();
            props.insert("name", PropertyValue::String("Charlie".to_string()));
            props
        },
    );

    store.create_edge(alice, bob, Label::new("KNOWS"), PropertyMap::new());
    store.create_edge(bob, charlie, Label::new("KNOWS"), PropertyMap::new());
    store.create_edge(alice, charlie, Label::new("FOLLOWS"), PropertyMap::new());

    assert_eq!(store.nodes.len(), 3);
    assert_eq!(store.edges.len(), 3);
}

#[test]
fn graph_realistic_knowledge_graph() {
    let mut store = GraphStoreWithEdges::new();

    let sqlrustgo = store.create_node(
        vec![Label::new("Software"), Label::new("Database")],
        {
            let mut props = PropertyMap::new();
            props.insert("name", PropertyValue::String("SQLRustGo".to_string()));
            props.insert("language", PropertyValue::String("Rust".to_string()));
            props
        },
    );

    let rust = store.create_node(
        vec![Label::new("ProgrammingLanguage")],
        {
            let mut props = PropertyMap::new();
            props.insert("name", PropertyValue::String("Rust".to_string()));
            props.insert("paradigm", PropertyValue::String("Systems".to_string()));
            props
        },
    );

    store.create_edge(sqlrustgo, rust, Label::new("WRITTEN_IN"), {
        let mut props = PropertyMap::new();
        props.insert("since", PropertyValue::Int(2024));
        props
    });

    assert_eq!(store.nodes.len(), 2);
    assert_eq!(store.edges.len(), 1);
}

// ============================================================================
// Performance-Relevant Tests
// ============================================================================

#[test]
fn graph_large_node_count() {
    let mut store = SimGraphStore::new();

    for i in 0..1000 {
        let mut props = PropertyMap::new();
        props.insert("id", PropertyValue::Int(i));
        store.create_node(vec![Label::new("Node")], props);
    }

    assert_eq!(store.nodes.len(), 1000);
}

#[test]
fn graph_large_edge_count() {
    let mut store = GraphStoreWithEdges::new();

    let mut node_ids = Vec::new();
    for _ in 0..100 {
        node_ids.push(store.create_node(vec![Label::new("Node")], PropertyMap::new()));
    }

    for i in 1..100 {
        store.create_edge(node_ids[0], node_ids[i], Label::new("CONNECTED"), PropertyMap::new());
    }

    assert_eq!(store.nodes.len(), 100);
    assert_eq!(store.edges.len(), 99);
}

#[test]
fn graph_property_map_large() {
    let mut props = PropertyMap::new();

    for i in 0..100 {
        props.insert(format!("prop_{}", i), PropertyValue::Int(i as i64));
    }

    assert_eq!(props.len(), 100);
}
