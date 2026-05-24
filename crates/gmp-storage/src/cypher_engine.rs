//! Cypher Graph Engine wrapper around sqlrustgo_graph

use anyhow::{Context, Result};
use sqlrustgo_graph::{
    execute_cypher, DiskGraphStore, GraphStore, NodeId, PropertyMap, PropertyValue,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

const GRAPH_DIR: &str = "graph_data";

/// Cypher query result
#[derive(Debug, Clone)]
pub struct CypherQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<PropertyValue>>,
}

/// Wrapper around sqlrustgo_graph for GMP storage (disk-backed with WAL)
///
/// Uses DiskGraphStore: WAL-enabled persistent storage with crash recovery.
/// 图数据在服务重启后自动从 WAL 恢复。
pub struct CypherEngine {
    store: Mutex<DiskGraphStore>,
    /// Cached graph directory path for clear/clone recreation
    graph_dir: PathBuf,
}

impl CypherEngine {
    /// Create a new Cypher engine with disk-backed WAL storage
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        let graph_dir = data_dir.join(GRAPH_DIR);
        let store = DiskGraphStore::load(graph_dir.clone())
            .context("Failed to load DiskGraphStore from disk")?;
        Ok(Self {
            store: Mutex::new(store),
            graph_dir,
        })
    }

    /// Create a new in-memory Cypher engine (no persistence — for testing)
    pub fn in_memory() -> Self {
        let graph_dir = tempfile::TempDir::new()
            .expect("TempDir for in_memory engine should not fail")
            .into_path();
        let store = DiskGraphStore::new_without_wal(graph_dir.clone())
            .expect("DiskGraphStore::new_without_wal should not fail for temp dir");
        Self {
            store: Mutex::new(store),
            graph_dir,
        }
    }

    /// Create a node with label and properties
    pub fn create_node(
        &self,
        label: &str,
        properties: HashMap<String, PropertyValue>,
    ) -> Result<u64> {
        let mut props = PropertyMap::new();
        for (k, v) in properties {
            props.insert(k, v);
        }
        let mut store = self.store.lock().unwrap();
        let node_id = store.create_node(label, props);
        Ok(node_id.as_u64())
    }

    /// Create an edge between two nodes
    pub fn create_edge(
        &self,
        from_id: u64,
        to_id: u64,
        label: &str,
        properties: HashMap<String, PropertyValue>,
    ) -> Result<()> {
        let mut props = PropertyMap::new();
        for (k, v) in properties {
            props.insert(k, v);
        }
        let mut store = self.store.lock().unwrap();
        let _ = store.create_edge(NodeId::new(from_id), NodeId::new(to_id), label, props);
        Ok(())
    }

    /// Execute a Cypher query and return results
    pub fn execute(&self, query: &str) -> Result<CypherQueryResult> {
        let store = self.store.lock().unwrap();
        execute_cypher(query, &*store)
            .map(|result| CypherQueryResult {
                columns: result.columns,
                rows: result.rows,
            })
            .context("Failed to execute Cypher query")
    }

    /// Get nodes by label
    pub fn get_nodes_by_label(&self, label: &str) -> Vec<u64> {
        let store = self.store.lock().unwrap();
        store
            .nodes_by_label(label)
            .into_iter()
            .map(|id| id.as_u64())
            .collect()
    }

    /// Get the total node count
    pub fn node_count(&self) -> usize {
        let store = self.store.lock().unwrap();
        store.node_count()
    }

    /// Get node property
    pub fn get_node_property(&self, node_id: u64, property: &str) -> Option<PropertyValue> {
        let store = self.store.lock().unwrap();
        store
            .get_node(NodeId::new(node_id))
            .and_then(|n| n.properties.get(property).cloned())
    }

    /// Clear all graph data (creates fresh WAL-less store)
    pub fn clear(&self) {
        let new_store = DiskGraphStore::new_without_wal(self.graph_dir.clone())
            .expect("DiskGraphStore::new_without_wal should not fail in clear");
        let mut store = self.store.lock().unwrap();
        *store = new_store;
    }

    /// Search nodes by property value
    pub fn find_nodes_by_property(
        &self,
        label: &str,
        property: &str,
        value: &PropertyValue,
    ) -> Vec<u64> {
        let store = self.store.lock().unwrap();
        store
            .nodes_by_label(label)
            .into_iter()
            .filter(|id| {
                store
                    .get_node(*id)
                    .and_then(|n| n.properties.get(property).cloned())
                    .map(|v| v == *value)
                    .unwrap_or(false)
            })
            .map(|id| id.as_u64())
            .collect()
    }

    /// Reload graph from disk (re-read WAL and JSON files)
    pub fn reload(&self, data_dir: PathBuf) -> Result<()> {
        let graph_dir = data_dir.join(GRAPH_DIR);
        let new_store = DiskGraphStore::load(graph_dir)
            .context("Failed to reload DiskGraphStore from disk")?;
        let mut store = self.store.lock().unwrap();
        *store = new_store;
        Ok(())
    }

    /// Path to the graph data directory
    pub fn graph_path(&self) -> PathBuf {
        self.graph_dir.clone()
    }
}

impl Clone for CypherEngine {
    fn clone(&self) -> Self {
        let new_store = DiskGraphStore::new_without_wal(self.graph_dir.clone())
            .expect("DiskGraphStore::new_without_wal should not fail in clone");
        Self {
            store: Mutex::new(new_store),
            graph_dir: self.graph_dir.clone(),
        }
    }
}

impl Default for CypherEngine {
    fn default() -> Self {
        Self::in_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_graph::PropertyValue;
    use tempfile::TempDir;

    #[test]
    fn test_cypher_create_and_query() {
        let engine = CypherEngine::in_memory();

        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            PropertyValue::String("SOP-001".to_string()),
        );
        props.insert("version".to_string(), PropertyValue::Int(1));
        let node_id = engine.create_node("SOP", props).unwrap();
        assert_eq!(node_id, 0);

        let result = engine.execute("MATCH (n:SOP) RETURN n").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_cypher_empty_graph() {
        let engine = CypherEngine::in_memory();
        let result = engine.execute("MATCH (n) RETURN n").unwrap();
        assert_eq!(result.rows.len(), 0);
    }

    #[test]
    fn test_cypher_with_where() {
        let engine = CypherEngine::in_memory();

        let mut props1 = HashMap::new();
        props1.insert("age".to_string(), PropertyValue::Int(30));
        engine.create_node("User", props1).unwrap();

        let mut props2 = HashMap::new();
        props2.insert("age".to_string(), PropertyValue::Int(25));
        engine.create_node("User", props2).unwrap();

        let result = engine
            .execute("MATCH (n:User) WHERE n.age > 28 RETURN n")
            .unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_cypher_disk_store_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let engine = CypherEngine::new(data_dir.clone()).unwrap();

        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            PropertyValue::String("TestNode".to_string()),
        );
        engine.create_node("Product", props).unwrap();

        let result = engine.execute("MATCH (n:Product) RETURN n").unwrap();
        assert_eq!(result.rows.len(), 1);

        engine.reload(data_dir).unwrap();
        let result = engine.execute("MATCH (n:Product) RETURN n").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_cypher_clone_independent() {
        let engine = CypherEngine::in_memory();

        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            PropertyValue::String("Original".to_string()),
        );
        engine.create_node("Test", props).unwrap();

        let engine2 = engine.clone();
        let result = engine2.execute("MATCH (n:Test) RETURN n").unwrap();
        assert_eq!(result.rows.len(), 0);
    }

    #[test]
    fn test_cypher_clear() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let engine = CypherEngine::new(data_dir).unwrap();

        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            PropertyValue::String("ToBeCleared".to_string()),
        );
        engine.create_node("Item", props).unwrap();
        assert_eq!(engine.node_count(), 1);

        engine.clear();
        assert_eq!(engine.node_count(), 0);
    }

    #[test]
    fn test_cypher_wal_entry_types() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let engine = CypherEngine::new(data_dir.clone()).unwrap();

        let mut props1 = HashMap::new();
        props1.insert(
            "name".to_string(),
            PropertyValue::String("Node1".to_string()),
        );
        let id1 = engine.create_node("Label", props1).unwrap();

        let mut props2 = HashMap::new();
        props2.insert(
            "name".to_string(),
            PropertyValue::String("Node2".to_string()),
        );
        let id2 = engine.create_node("Label", props2).unwrap();

        engine.create_edge(id1, id2, "REL", HashMap::new()).unwrap();

        engine.reload(data_dir).unwrap();
        let result = engine.execute("MATCH (n:Label) RETURN n").unwrap();
        assert_eq!(result.rows.len(), 2);

        let edge_result = engine
            .execute("MATCH (a:Label)-[:REL]->(b:Label) RETURN a, b")
            .unwrap();
        assert_eq!(edge_result.rows.len(), 1);
    }

    #[test]
    fn test_graph_path() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let engine = CypherEngine::new(data_dir.clone()).unwrap();
        assert_eq!(engine.graph_path(), data_dir.join(GRAPH_DIR));
    }
}