//! GMP Graph Writer - writes Layer 0 extraction results to Graph Store

use sqlrustgo_graph::store::{GraphStore, InMemoryGraphStore};
use sqlrustgo_graph::{EdgeId, NodeId, PropertyMap, PropertyValue};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Entity record for batch writing
#[derive(Debug, Clone)]
pub struct EntityRecord {
    pub entity_type: String,
    pub canonical_term: String,
    pub surface_form: String,
    pub doc_id: String,
    pub confidence: f64,
}

/// Relation record for batch writing
#[derive(Debug, Clone)]
pub struct RelationRecord {
    pub relation_type: String,
    pub src_entity: String,
    pub dst_entity: String,
    pub src_type: String,
    pub dst_type: String,
    pub confidence: f64,
}

/// GMP Graph Writer - writes Layer 0 entities/relations to graph
pub struct GmpGraphWriter {
    graph: Arc<RwLock<InMemoryGraphStore>>,
    entity_cache: HashMap<String, NodeId>,
    doc_cache: HashMap<String, NodeId>,
}

impl GmpGraphWriter {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(InMemoryGraphStore::new())),
            entity_cache: HashMap::new(),
            doc_cache: HashMap::new(),
        }
    }

    pub fn with_graph(graph: Arc<RwLock<InMemoryGraphStore>>) -> Self {
        Self {
            graph,
            entity_cache: HashMap::new(),
            doc_cache: HashMap::new(),
        }
    }

    pub fn graph(&self) -> Arc<RwLock<InMemoryGraphStore>> {
        self.graph.clone()
    }

    fn get_or_create_entity_node(&mut self, entity_type: &str, canonical_term: &str) -> NodeId {
        let cache_key = format!("{}:{}", entity_type, canonical_term);
        if let Some(&node_id) = self.entity_cache.get(&cache_key) {
            return node_id;
        }

        let mut props = PropertyMap::new();
        props.insert("entity_type".to_string(), PropertyValue::String(entity_type.to_string()));
        props.insert("canonical_term".to_string(), PropertyValue::String(canonical_term.to_string()));

        let mut graph = self.graph.write().unwrap();
        let node_id = graph.create_node("Entity", props);
        self.entity_cache.insert(cache_key, node_id);
        node_id
    }

    fn get_or_create_document_node(&mut self, doc_id: &str) -> NodeId {
        if let Some(&node_id) = self.doc_cache.get(doc_id) {
            return node_id;
        }

        let mut props = PropertyMap::new();
        props.insert("doc_id".to_string(), PropertyValue::String(doc_id.to_string()));

        let mut graph = self.graph.write().unwrap();
        let node_id = graph.create_node("Document", props);
        self.doc_cache.insert(doc_id.to_string(), node_id);
        node_id
    }

    pub fn write_entity(&mut self, entity_type: &str, canonical_term: &str, surface_form: &str, doc_id: &str, confidence: f64) -> NodeId {
        let doc_node_id = self.get_or_create_document_node(doc_id);
        let node_id = self.get_or_create_entity_node(entity_type, canonical_term);

        let mut props = PropertyMap::new();
        props.insert("surface_form".to_string(), PropertyValue::String(surface_form.to_string()));
        props.insert("doc_id".to_string(), PropertyValue::String(doc_id.to_string()));
        props.insert("confidence".to_string(), PropertyValue::Float(confidence));

        let mut graph = self.graph.write().unwrap();
        let _ = graph.create_edge(node_id, doc_node_id, "MENTIONED_IN", props);
        node_id
    }

    pub fn write_relation(&mut self, relation_type: &str, src_entity: &str, dst_entity: &str, src_type: &str, dst_type: &str, confidence: f64) -> Option<EdgeId> {
        let src_id = self.get_or_create_entity_node(src_type, src_entity);
        let dst_id = self.get_or_create_entity_node(dst_type, dst_entity);

        let mut props = PropertyMap::new();
        props.insert("relation_type".to_string(), PropertyValue::String(relation_type.to_string()));
        props.insert("confidence".to_string(), PropertyValue::Float(confidence));

        let mut graph = self.graph.write().unwrap();
        graph.create_edge(src_id, dst_id, relation_type, props).ok()
    }

    pub fn write_entities(&mut self, entities: &[EntityRecord]) {
        for e in entities {
            self.write_entity(&e.entity_type, &e.canonical_term, &e.surface_form, &e.doc_id, e.confidence);
        }
    }

    pub fn write_relations(&mut self, relations: &[RelationRecord]) {
        for r in relations {
            self.write_relation(&r.relation_type, &r.src_entity, &r.dst_entity, &r.src_type, &r.dst_type, r.confidence);
        }
    }

    pub fn stats(&self) -> GraphStats {
        let graph = self.graph.read().unwrap();
        GraphStats {
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            entity_types: self.entity_cache.len(),
        }
    }
}

impl Default for GmpGraphWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub entity_types: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_entity() {
        let mut writer = GmpGraphWriter::new();
        let node_id = writer.write_entity("Deviation", "偏差", "温度超标", "SOP-001", 0.95);
        assert!(node_id.0 > 0);
        let stats = writer.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
    }

    #[test]
    fn test_write_relation() {
        let mut writer = GmpGraphWriter::new();
        writer.write_entity("Deviation", "偏差", "温度超标", "SOP-001", 0.95);
        writer.write_entity("RootCause", "设备故障", "设备故障", "SOP-001", 0.90);
        let edge_id = writer.write_relation("CAUSED_BY", "偏差", "设备故障", "Deviation", "RootCause", 0.85);
        assert!(edge_id.is_some());
        let stats = writer.stats();
        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 3);
    }

    #[test]
    fn test_entity_deduplication() {
        let mut writer = GmpGraphWriter::new();
        writer.write_entity("Deviation", "偏差", "温度超标1", "SOP-001", 0.95);
        writer.write_entity("Deviation", "偏差", "温度超标2", "SOP-002", 0.90);
        assert_eq!(writer.stats().entity_types, 1);
    }
}
