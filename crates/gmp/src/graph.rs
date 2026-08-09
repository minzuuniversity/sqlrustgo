//! GMP Graph Projection
//!
//! Provides SQL-backed graph projection for GMP evidence navigation.
//! Projects document relationships into navigable node/edge subgraphs.

use crate::chunk::get_chunks_for_version;
use crate::document::{Document, TABLE_DOCUMENTS};
use crate::relation::{
    get_neighbors, path_query, GraphPath, PathEdge, PathNode, Relation,
};
use crate::schema::RelationType;
use crate::version::get_document_versions;
use serde::{Deserialize, Serialize};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};
use std::collections::HashMap;

/// A graph node with type and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub doc_id: i64,
    pub node_type: String,
    pub title: String,
    pub status: String,
    pub effective_date: i32,
    pub version: i32,
}

/// A graph edge with type and optional chunk references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source_doc_id: i64,
    pub target_doc_id: i64,
    pub relation_type: String,
    pub source_chunk_id: Option<i64>,
    pub target_chunk_id: Option<i64>,
    pub properties: Option<String>,
}

impl From<&PathEdge> for GraphEdge {
    fn from(edge: &PathEdge) -> Self {
        GraphEdge {
            source_doc_id: edge.source_doc_id,
            target_doc_id: edge.target_doc_id,
            relation_type: edge.relation_type.as_str().to_string(),
            source_chunk_id: edge.source_chunk_id,
            target_chunk_id: edge.target_chunk_id,
            properties: edge.properties.as_ref().map(|v| v.to_string()),
        }
    }
}

impl From<&Relation> for GraphEdge {
    fn from(rel: &Relation) -> Self {
        GraphEdge {
            source_doc_id: rel.source_doc_id.unwrap_or(0),
            target_doc_id: rel.target_doc_id.unwrap_or(0),
            relation_type: rel.relation_type.as_str().to_string(),
            source_chunk_id: rel.source_chunk_id,
            target_chunk_id: rel.target_chunk_id,
            properties: rel.properties.as_ref().map(|v| v.to_string()),
        }
    }
}

/// A projected subgraph with nodes and edges.
#[derive(Debug, Clone, Default)]
pub struct GraphProjection {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl GraphProjection {
    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
    pub fn get_node(&self, doc_id: i64) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.doc_id == doc_id)
    }
}

/// An evidence item extracted from a document chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub chunk_id: i64,
    pub chunk_hash: String,
    pub section_name: Option<String>,
    pub content_text: String,
    pub source_path: String,
}

/// An evidence bundle for a graph path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub path: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub evidence: Vec<EvidenceItem>,
    pub total_chunks: usize,
}

impl EvidenceBundle {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn summary(&self) -> String {
        format!(
            "EvidenceBundle {{ nodes: {}, edges: {}, evidence_chunks: {} }}",
            self.path.len(),
            self.edges.len(),
            self.total_chunks,
        )
    }
}

/// Convert a RelationType to a human-readable node type string.
pub fn relation_type_to_node_type(rt: &RelationType) -> &'static str {
    match rt {
        RelationType::Sop => "SOP",
        RelationType::Clause => "CLAUSE",
        RelationType::Capa => "CAPA",
        RelationType::Deviation => "DEVIATION",
        RelationType::Role => "ROLE",
        RelationType::Equipment => "EQUIPMENT",
        RelationType::AuditFinding => "AUDIT_FINDING",
    }
}

/// Get a document's latest version number.
pub fn get_latest_version(storage: &dyn StorageEngine, doc_id: i64) -> SqlResult<i32> {
    let versions = get_document_versions(storage, doc_id)?;
    Ok(versions.iter().map(|v| v.version_number).max().unwrap_or(1))
}

/// Get all document types in the graph.
pub fn get_all_node_types(storage: &dyn StorageEngine) -> SqlResult<Vec<String>> {
    let rows = storage.scan(TABLE_DOCUMENTS)?;
    let mut types: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in rows {
        if let Some(Value::Text(t)) = row.get(2) {
            types.insert(t.clone());
        }
    }
    Ok(types.into_iter().collect())
}

/// Project a subgraph centered on a document up to given depth.
pub fn project_subgraph(
    storage: &dyn StorageEngine,
    center_doc_id: i64,
    depth: usize,
    relation_types: Option<&[RelationType]>,
) -> SqlResult<GraphProjection> {
    let doc_rows = storage.scan(TABLE_DOCUMENTS)?;
    let docs: HashMap<i64, Document> = doc_rows
        .into_iter()
        .filter_map(|row| Document::from_row(&row).map(|d| (d.id, d)))
        .collect();

    let rel_rows = storage.scan("gmp_relations")?;
    let relations: Vec<Relation> = rel_rows
        .into_iter()
        .filter_map(|r| Relation::from_row(&r))
        .collect();

    let mut visited: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut queue = vec![(center_doc_id, 0)];
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    visited.insert(center_doc_id);

    while let Some((current_id, current_depth)) = queue.pop() {
        if current_depth > depth {
            continue;
        }

        if let Some(doc) = docs.get(&current_id) {
            let version = get_latest_version(storage, current_id).unwrap_or(1);
            nodes.push(GraphNode {
                doc_id: current_id,
                node_type: doc.doc_type.clone(),
                title: doc.title.clone(),
                status: doc.status.as_str().to_string(),
                effective_date: doc.effective_date,
                version,
            });
        }

        let neighbors: Vec<Relation> = get_neighbors(storage, current_id, None)
            .unwrap_or_default()
            .into_iter()
            .filter(|r| {
                relation_types.map_or(true, |types| types.contains(&r.relation_type))
            })
            .collect();

        for rel in neighbors {
            edges.push(GraphEdge::from(&rel));

            let neighbor_id = if rel.source_doc_id == Some(current_id) {
                rel.target_doc_id
            } else {
                rel.source_doc_id
            };

            if let Some(nid) = neighbor_id {
                if !visited.contains(&nid) {
                    visited.insert(nid);
                    queue.push((nid, current_depth + 1));
                }
            }
        }
    }

    Ok(GraphProjection { nodes, edges })
}

/// Generate an evidence bundle from a graph path.
pub fn generate_evidence_bundle(
    storage: &dyn StorageEngine,
    graph_path: &GraphPath,
) -> SqlResult<EvidenceBundle> {
    let doc_rows = storage.scan(TABLE_DOCUMENTS)?;
    let docs: HashMap<i64, Document> = doc_rows
        .into_iter()
        .filter_map(|row| Document::from_row(&row).map(|d| (d.id, d)))
        .collect();

    let mut path_nodes_out = Vec::new();
    let mut all_edges = Vec::new();
    let mut all_evidence = Vec::new();
    let mut total_chunks = 0;

    for path_node in &graph_path.nodes {
        let doc_id = path_node.doc_id;
        if let Some(doc) = docs.get(&doc_id) {
            let version = get_latest_version(storage, doc_id).unwrap_or(1);
            path_nodes_out.push(GraphNode {
                doc_id,
                node_type: doc.doc_type.clone(),
                title: doc.title.clone(),
                status: doc.status.as_str().to_string(),
                effective_date: doc.effective_date,
                version,
            });

            if let Ok(chunks) = get_chunks_for_version(storage, doc_id, version) {
                for chunk in chunks {
                    all_evidence.push(EvidenceItem {
                        chunk_id: chunk.id,
                        chunk_hash: chunk.content_hash.clone(),
                        section_name: chunk.section_name.clone(),
                        content_text: chunk.content_text.clone(),
                        source_path: doc.doc_type.clone(),
                    });
                    total_chunks += 1;
                }
            }
        }
    }

    for path_edge in &graph_path.edges {
        all_edges.push(GraphEdge::from(path_edge));
    }

    Ok(EvidenceBundle {
        path: path_nodes_out,
        edges: all_edges,
        evidence: all_evidence,
        total_chunks,
    })
}

/// Graph statistics.
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub node_types: Vec<(String, usize)>,
    pub edge_types: Vec<(String, usize)>,
    pub avg_degree: f32,
}

/// Compute graph statistics.
pub fn get_graph_stats(storage: &dyn StorageEngine) -> SqlResult<GraphStats> {
    let doc_rows = storage.scan(TABLE_DOCUMENTS)?;
    let rel_rows = storage.scan("gmp_relations")?;

    let docs: Vec<Document> = doc_rows
        .into_iter()
        .filter_map(|r| Document::from_row(&r))
        .collect();

    let relations: Vec<Relation> = rel_rows
        .into_iter()
        .filter_map(|r| Relation::from_row(&r))
        .collect();

    let mut node_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut edge_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for doc in &docs {
        *node_type_counts.entry(doc.doc_type.clone()).or_insert(0) += 1;
    }

    for rel in &relations {
        let t = rel.relation_type.as_str().to_string();
        *edge_type_counts.entry(t).or_insert(0) += 1;
    }

    let total_edges = relations.len();
    let avg_degree = if docs.is_empty() {
        0.0
    } else {
        (total_edges * 2) as f32 / docs.len() as f32
    };

    let mut node_types: Vec<_> = node_type_counts.into_iter().collect();
    node_types.sort_by(|a, b| b.1.cmp(&a.1));
    let mut edge_types: Vec<_> = edge_type_counts.into_iter().collect();
    edge_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(GraphStats {
        total_nodes: docs.len(),
        total_edges,
        node_types,
        edge_types,
        avg_degree,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{create_gmp_tables, DocStatus};
    use crate::relation::insert_relation;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_graph_node_creation() {
        let node = GraphNode {
            doc_id: 1,
            node_type: "SOP".to_string(),
            title: "Test SOP".to_string(),
            status: "ACTIVE".to_string(),
            effective_date: 19000,
            version: 1,
        };
        assert_eq!(node.doc_id, 1);
        assert_eq!(node.node_type, "SOP");
    }

    #[test]
    fn test_graph_edge_from_path_edge() {
        let edge = PathEdge {
            source_doc_id: 1,
            source_chunk_id: Some(10),
            relation_type: RelationType::Clause,
            target_doc_id: 2,
            target_chunk_id: Some(20),
            properties: None,
        };
        let ge = GraphEdge::from(&edge);
        assert_eq!(ge.source_doc_id, 1);
        assert_eq!(ge.relation_type, "CLAUSE");
    }

    #[test]
    fn test_graph_edge_from_relation() {
        let rel = Relation {
            id: 1,
            source_doc_id: Some(1),
            source_chunk_id: Some(10),
            target_doc_id: Some(2),
            target_chunk_id: Some(20),
            relation_type: RelationType::Sop,
            properties: Some(serde_json::json!({"strength": "MUST"})),
            created_at: 0,
        };
        let edge = GraphEdge::from(&rel);
        assert_eq!(edge.source_doc_id, 1);
        assert_eq!(edge.target_doc_id, 2);
        assert!(edge.properties.is_some());
    }

    #[test]
    fn test_graph_projection_counts() {
        let proj = GraphProjection {
            nodes: vec![
                GraphNode { doc_id: 1, node_type: "SOP".to_string(), title: "".to_string(), status: "".to_string(), effective_date: 0, version: 1 },
                GraphNode { doc_id: 2, node_type: "CLAUSE".to_string(), title: "".to_string(), status: "".to_string(), effective_date: 0, version: 1 },
            ],
            edges: vec![],
        };
        assert_eq!(proj.node_count(), 2);
        assert_eq!(proj.edge_count(), 0);
    }

    #[test]
    fn test_evidence_bundle_summary() {
        let bundle = EvidenceBundle {
            path: vec![],
            edges: vec![],
            evidence: vec![],
            total_chunks: 5,
        };
        let s = bundle.summary();
        assert!(s.contains("nodes: 0"));
        assert!(s.contains("evidence_chunks: 5"));
    }

    #[test]
    fn test_evidence_bundle_json() {
        let bundle = EvidenceBundle::default();
        let json = bundle.to_json();
        assert!(!json.is_empty()); // serializes to full JSON
    }

    #[test]
    fn test_relation_type_to_node_type() {
        assert_eq!(relation_type_to_node_type(&RelationType::Sop), "SOP");
        assert_eq!(relation_type_to_node_type(&RelationType::Clause), "CLAUSE");
        assert_eq!(relation_type_to_node_type(&RelationType::Capa), "CAPA");
        assert_eq!(relation_type_to_node_type(&RelationType::Deviation), "DEVIATION");
        assert_eq!(relation_type_to_node_type(&RelationType::Role), "ROLE");
        assert_eq!(relation_type_to_node_type(&RelationType::Equipment), "EQUIPMENT");
        assert_eq!(relation_type_to_node_type(&RelationType::AuditFinding), "AUDIT_FINDING");
    }

    #[test]
    fn test_project_subgraph_empty() {
        let storage = MemoryStorage::new();
        let proj = project_subgraph(&storage, 999, 3, None).unwrap();
        assert_eq!(proj.node_count(), 0);
        assert_eq!(proj.edge_count(), 0);
    }

    #[test]
    fn test_get_graph_stats_empty() {
        let storage = MemoryStorage::new();
        let stats = get_graph_stats(&storage).unwrap();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_edges, 0);
        assert_eq!(stats.avg_degree, 0.0);
    }
}
