//! QMD Bridge data types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of data in QMD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QmdDataType {
    /// Vector data
    Vector,
    /// Graph data
    Graph,
    /// Document/text data
    Document,
    /// Mixed type
    Mixed,
}

/// Type of query
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    /// K-nearest neighbors search
    Knn,
    /// Breadth-first search
    Bfs,
    /// Depth-first search
    Dfs,
    /// Range search
    Range,
    /// Hybrid search
    Hybrid,
}

/// Data format for QMD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmdData {
    /// Unique identifier
    pub id: String,
    /// Type of data
    pub data_type: QmdDataType,
    /// Vector data (for vector type)
    pub vector: Option<Vec<f32>>,
    /// Graph structure (for graph type)
    pub graph: Option<GraphData>,
    /// Text content (for document type)
    pub text: Option<String>,
    /// Metadata key-value pairs
    pub metadata: HashMap<String, String>,
    /// Timestamp
    pub timestamp: i64,
}

impl QmdData {
    /// Create a new vector data entry
    pub fn new_vector(id: &str, vector: Vec<f32>) -> Self {
        Self {
            id: id.to_string(),
            data_type: QmdDataType::Vector,
            vector: Some(vector),
            graph: None,
            text: None,
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    /// Create a new document data entry
    pub fn new_document(id: &str, text: &str) -> Self {
        Self {
            id: id.to_string(),
            data_type: QmdDataType::Document,
            vector: None,
            graph: None,
            text: Some(text.to_string()),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}

/// Graph data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    /// Nodes with their properties
    pub nodes: Vec<GraphNode>,
    /// Edges between nodes
    pub edges: Vec<GraphEdge>,
}

/// A node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub properties: HashMap<String, String>,
}

/// An edge in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub properties: HashMap<String, String>,
}

/// Query for QMD search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmdQuery {
    /// Type of query
    pub query_type: QueryType,
    /// Vector for vector search
    pub vector: Option<Vec<f32>>,
    /// Graph pattern for graph search
    pub graph_pattern: Option<String>,
    /// Text for text search
    pub text: Option<String>,
    /// Filters to apply
    pub filters: Vec<Filter>,
    /// Maximum results to return
    pub limit: usize,
    /// Distance threshold
    pub threshold: Option<f32>,
}

/// Filter condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: String,
}

/// Filter operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    Contains,
}

/// Search result from QMD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Result ID
    pub id: String,
    /// Similarity score
    pub score: f32,
    /// Result data
    pub data: QmdData,
}

/// Sync status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Last sync timestamp
    pub last_sync: i64,
    /// Number of items synced
    pub items_synced: u64,
    /// Sync state
    pub state: SyncState,
    /// Error message if failed
    pub error: Option<String>,
}

/// State of sync
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    /// Idle, no sync in progress
    Idle,
    /// Sync in progress
    Syncing,
    /// Sync completed successfully
    Completed,
    /// Sync failed
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qmd_data_type_variants() {
        assert_eq!(format!("{:?}", QmdDataType::Vector), "Vector");
        assert_eq!(format!("{:?}", QmdDataType::Graph), "Graph");
        assert_eq!(format!("{:?}", QmdDataType::Document), "Document");
        assert_eq!(format!("{:?}", QmdDataType::Mixed), "Mixed");
        assert_eq!(QmdDataType::Vector as u8, QmdDataType::Vector as u8);
    }

    #[test]
    fn test_query_type_variants() {
        assert_eq!(QueryType::Knn as u8, QueryType::Knn as u8);
        assert_ne!(QueryType::Knn, QueryType::Bfs);
        assert_ne!(QueryType::Dfs, QueryType::Range);
    }

    #[test]
    fn test_qmd_data_new_vector() {
        let data = QmdData::new_vector("vec1", vec![1.0, 2.0, 3.0]);
        assert_eq!(data.id, "vec1");
        assert_eq!(data.data_type, QmdDataType::Vector);
        assert_eq!(data.vector, Some(vec![1.0, 2.0, 3.0]));
        assert!(data.graph.is_none());
        assert!(data.text.is_none());
    }

    #[test]
    fn test_qmd_data_new_document() {
        let data = QmdData::new_document("doc1", "hello world");
        assert_eq!(data.id, "doc1");
        assert_eq!(data.data_type, QmdDataType::Document);
        assert_eq!(data.text, Some("hello world".to_string()));
        assert!(data.vector.is_none());
    }

    #[test]
    fn test_qmd_data_timestamp() {
        let before = std::time::SystemTime::now();
        let data = QmdData::new_vector("t1", vec![0.0]);
        let after = std::time::SystemTime::now();
        assert!(
            data.timestamp
                >= before
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
        );
        assert!(
            data.timestamp
                <= after
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
        );
    }

    #[test]
    fn test_qmd_data_clone() {
        let data = QmdData::new_vector("c1", vec![1.0]);
        let cloned = data.clone();
        assert_eq!(data.id, cloned.id);
        assert_eq!(data.data_type, cloned.data_type);
        assert_eq!(data.vector, cloned.vector);
    }

    #[test]
    fn test_graph_data_new() {
        let mut nodes = Vec::new();
        nodes.push(GraphNode {
            id: "n1".to_string(),
            label: "person".to_string(),
            properties: HashMap::new(),
        });
        let graph = GraphData {
            nodes,
            edges: vec![GraphEdge {
                from: "n1".to_string(),
                to: "n2".to_string(),
                relation: "knows".to_string(),
                properties: HashMap::new(),
            }],
        };
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_graph_node_clone() {
        let mut props = HashMap::new();
        props.insert("key".to_string(), "value".to_string());
        let node = GraphNode {
            id: "n1".to_string(),
            label: "label".to_string(),
            properties: props,
        };
        let cloned = node.clone();
        assert_eq!(node.id, cloned.id);
    }

    #[test]
    fn test_graph_edge_clone() {
        let edge = GraphEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            relation: "edge".to_string(),
            properties: HashMap::new(),
        };
        let cloned = edge.clone();
        assert_eq!(edge.from, cloned.from);
        assert_eq!(edge.to, cloned.to);
        assert_eq!(edge.relation, cloned.relation);
    }

    #[test]
    fn test_filter_operator_variants() {
        assert_ne!(FilterOperator::Eq, FilterOperator::Ne);
        assert_ne!(FilterOperator::Gt, FilterOperator::Lt);
        assert_ne!(FilterOperator::Gte, FilterOperator::Lte);
    }

    #[test]
    fn test_filter_new() {
        let filter = Filter {
            field: "age".to_string(),
            operator: FilterOperator::Gt,
            value: "18".to_string(),
        };
        assert_eq!(filter.field, "age");
        assert_eq!(filter.operator, FilterOperator::Gt);
    }

    #[test]
    fn test_qmd_query_new_knn() {
        let query = QmdQuery {
            query_type: QueryType::Knn,
            vector: Some(vec![0.1, 0.2]),
            graph_pattern: None,
            text: None,
            filters: vec![],
            limit: 10,
            threshold: Some(0.5),
        };
        assert_eq!(query.query_type, QueryType::Knn);
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_qmd_query_new_graph() {
        let query = QmdQuery {
            query_type: QueryType::Bfs,
            vector: None,
            graph_pattern: Some("A -> B -> C".to_string()),
            text: None,
            filters: vec![],
            limit: 100,
            threshold: None,
        };
        assert_eq!(query.query_type, QueryType::Bfs);
        assert_eq!(query.graph_pattern, Some("A -> B -> C".to_string()));
    }

    #[test]
    fn test_search_result_new() {
        let result = SearchResult {
            id: "r1".to_string(),
            score: 0.95,
            data: QmdData::new_vector("vec1", vec![1.0]),
        };
        assert_eq!(result.id, "r1");
        assert!((result.score - 0.95).abs() < 1e-6);
    }

    #[test]
    fn test_search_result_clone() {
        let result = SearchResult {
            id: "r1".to_string(),
            score: 0.99,
            data: QmdData::new_document("doc1", "test"),
        };
        let cloned = result.clone();
        assert_eq!(result.id, cloned.id);
        assert_eq!(result.score, cloned.score);
    }

    #[test]
    fn test_sync_status_new() {
        let status = SyncStatus {
            last_sync: 1000,
            items_synced: 50,
            state: SyncState::Completed,
            error: None,
        };
        assert_eq!(status.last_sync, 1000);
        assert_eq!(status.items_synced, 50);
        assert_eq!(status.state, SyncState::Completed);
    }

    #[test]
    fn test_sync_status_failed() {
        let status = SyncStatus {
            last_sync: 0,
            items_synced: 0,
            state: SyncState::Failed,
            error: Some("connection refused".to_string()),
        };
        assert_eq!(status.state, SyncState::Failed);
        assert_eq!(status.error, Some("connection refused".to_string()));
    }

    #[test]
    fn test_sync_state_variants() {
        assert_ne!(SyncState::Idle, SyncState::Syncing);
        assert_ne!(SyncState::Completed, SyncState::Failed);
        assert_eq!(SyncState::Idle as u8, SyncState::Idle as u8);
    }

    #[test]
    fn test_sync_status_clone() {
        let status = SyncStatus {
            last_sync: 9999,
            items_synced: 100,
            state: SyncState::Syncing,
            error: None,
        };
        let cloned = status.clone();
        assert_eq!(status.last_sync, cloned.last_sync);
        assert_eq!(status.state, cloned.state);
    }

    #[test]
    fn test_qmd_data_serde_roundtrip() {
        let data = QmdData::new_vector("serde1", vec![1.0, 2.0]);
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: QmdData = serde_json::from_str(&json).unwrap();
        assert_eq!(data.id, deserialized.id);
        assert_eq!(data.vector, deserialized.vector);
        assert_eq!(data.data_type, deserialized.data_type);
    }

    #[test]
    fn test_qmd_data_type_serde() {
        let json = serde_json::to_string(&QmdDataType::Vector).unwrap();
        assert_eq!(json, "\"vector\"");
        let deserialized: QmdDataType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, QmdDataType::Vector);
    }

    #[test]
    fn test_query_type_serde() {
        let json = serde_json::to_string(&QueryType::Knn).unwrap();
        assert_eq!(json, "\"knn\"");
        let deserialized: QueryType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, QueryType::Knn);
    }

    #[test]
    fn test_sync_state_serde() {
        let json = serde_json::to_string(&SyncState::Completed).unwrap();
        assert_eq!(json, "\"completed\"");
        let deserialized: SyncState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SyncState::Completed);
    }

    #[test]
    fn test_graph_data_serde_roundtrip() {
        let mut nodes = Vec::new();
        nodes.push(GraphNode {
            id: "n1".to_string(),
            label: "test".to_string(),
            properties: HashMap::new(),
        });
        let graph = GraphData {
            nodes,
            edges: vec![GraphEdge {
                from: "n1".to_string(),
                to: "n2".to_string(),
                relation: "rel".to_string(),
                properties: HashMap::new(),
            }],
        };
        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: GraphData = serde_json::from_str(&json).unwrap();
        assert_eq!(graph.nodes.len(), deserialized.nodes.len());
    }
}
