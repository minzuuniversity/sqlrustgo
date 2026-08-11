//! GMP Relation Graph Management
//!
//! Provides typed relation edges between GMP documents and chunks.
//! Relation types: SOP, CLAUSE, CAPA, DEVIATION, ROLE, EQUIPMENT, AUDIT_FINDING

use crate::schema::{RelationType, TABLE_RELATIONS};
use serde::{Deserialize, Serialize};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};

/// A relation edge between documents/chunks.
#[derive(Debug, Clone)]
pub struct Relation {
    pub id: i64,
    pub source_doc_id: Option<i64>,
    pub source_chunk_id: Option<i64>,
    pub relation_type: RelationType,
    pub target_doc_id: Option<i64>,
    pub target_chunk_id: Option<i64>,
    pub properties: Option<serde_json::Value>,
    pub created_at: i64,
}

impl Relation {
    /// Parse a Relation from a storage engine row.
    /// Expected column order: id, source_doc_id, source_chunk_id, relation_type,
    /// target_doc_id, target_chunk_id, properties, created_at
    pub fn from_row(row: &[Value]) -> Option<Self> {
        let source_doc_id = match &row.get(1)? {
            Value::Integer(n) => Some(*n),
            Value::Null => None,
            _ => return None,
        };
        let source_chunk_id = match &row.get(2)? {
            Value::Integer(n) => Some(*n),
            Value::Null => None,
            _ => return None,
        };
        let relation_type_str = match &row.get(3)? {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        let relation_type = RelationType::from_str(&relation_type_str)?;
        let target_doc_id = match &row.get(4)? {
            Value::Integer(n) => Some(*n),
            Value::Null => None,
            _ => return None,
        };
        let target_chunk_id = match &row.get(5)? {
            Value::Integer(n) => Some(*n),
            Value::Null => None,
            _ => return None,
        };
        let properties = match &row.get(6)? {
            Value::Text(s) => serde_json::from_str(s).ok(),
            Value::Null => None,
            _ => return None,
        };

        Some(Relation {
            id: match &row.get(0)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
            source_doc_id,
            source_chunk_id,
            relation_type,
            target_doc_id,
            target_chunk_id,
            properties,
            created_at: match &row.get(7)? {
                Value::Integer(n) => *n,
                _ => return None,
            },
        })
    }

    /// Convert to a database row.
    pub fn to_row(&self) -> Vec<Value> {
        vec![
            Value::Integer(self.id),
            self.source_doc_id
                .map(Value::Integer)
                .unwrap_or(Value::Null),
            self.source_chunk_id
                .map(Value::Integer)
                .unwrap_or(Value::Null),
            Value::Text(self.relation_type.as_str().to_string()),
            self.target_doc_id
                .map(Value::Integer)
                .unwrap_or(Value::Null),
            self.target_chunk_id
                .map(Value::Integer)
                .unwrap_or(Value::Null),
            self.properties
                .as_ref()
                .map(|p| Value::Text(p.to_string()))
                .unwrap_or(Value::Null),
            Value::Integer(self.created_at),
        ]
    }
}

/// Insert a relation edge (upsert by source + target + type).
pub fn insert_relation(
    storage: &mut dyn StorageEngine,
    source_doc_id: Option<i64>,
    source_chunk_id: Option<i64>,
    relation_type: &RelationType,
    target_doc_id: Option<i64>,
    target_chunk_id: Option<i64>,
    properties: Option<&serde_json::Value>,
) -> SqlResult<i64> {
    // Check for existing
    let existing = find_relation(
        storage,
        source_doc_id,
        source_chunk_id,
        relation_type,
        target_doc_id,
        target_chunk_id,
    )?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    if let Some(existing) = existing {
        return Ok(existing.id);
    }

    let rows = storage.scan(TABLE_RELATIONS)?;
    let next_id = rows
        .iter()
        .filter_map(|r| match r.get(0)? {
            Value::Integer(n) => Some(*n),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;

    let row = vec![
        Value::Integer(next_id),
        source_doc_id.map(Value::Integer).unwrap_or(Value::Null),
        source_chunk_id.map(Value::Integer).unwrap_or(Value::Null),
        Value::Text(relation_type.as_str().to_string()),
        target_doc_id.map(Value::Integer).unwrap_or(Value::Null),
        target_chunk_id.map(Value::Integer).unwrap_or(Value::Null),
        properties
            .map(|p| Value::Text(p.to_string()))
            .unwrap_or(Value::Null),
        Value::Integer(timestamp),
    ];

    storage.insert(TABLE_RELATIONS, vec![row])?;
    Ok(next_id)
}

/// Find a specific relation by its full key.
pub fn find_relation(
    storage: &dyn StorageEngine,
    source_doc_id: Option<i64>,
    source_chunk_id: Option<i64>,
    relation_type: &RelationType,
    target_doc_id: Option<i64>,
    target_chunk_id: Option<i64>,
) -> SqlResult<Option<Relation>> {
    let rows = storage.scan(TABLE_RELATIONS)?;
    let relation = rows
        .into_iter()
        .filter_map(|r| Relation::from_row(&r))
        .find(|rel| {
            rel.source_doc_id == source_doc_id
                && rel.source_chunk_id == source_chunk_id
                && rel.relation_type == *relation_type
                && rel.target_doc_id == target_doc_id
                && rel.target_chunk_id == target_chunk_id
        });
    Ok(relation)
}

/// Get all neighbors of a document, optionally filtered by relation type.
pub fn get_neighbors(
    storage: &dyn StorageEngine,
    doc_id: i64,
    relation_type: Option<&RelationType>,
) -> SqlResult<Vec<Relation>> {
    let rows = storage.scan(TABLE_RELATIONS)?;
    let relations: Vec<Relation> = rows
        .into_iter()
        .filter_map(|r| Relation::from_row(&r))
        .filter(|rel| {
            let matches_doc =
                rel.source_doc_id == Some(doc_id) || rel.target_doc_id == Some(doc_id);
            let matches_type = relation_type.map_or(true, |t| &rel.relation_type == t);
            matches_doc && matches_type
        })
        .collect();
    Ok(relations)
}

/// Get all neighbors of a chunk.
pub fn get_neighbors_chunk(storage: &dyn StorageEngine, chunk_id: i64) -> SqlResult<Vec<Relation>> {
    let rows = storage.scan(TABLE_RELATIONS)?;
    let relations: Vec<Relation> = rows
        .into_iter()
        .filter_map(|r| Relation::from_row(&r))
        .filter(|rel| {
            rel.source_chunk_id == Some(chunk_id) || rel.target_chunk_id == Some(chunk_id)
        })
        .collect();
    Ok(relations)
}

/// A path result from a graph traversal.
#[derive(Debug, Clone)]
pub struct GraphPath {
    pub nodes: Vec<PathNode>,
    pub edges: Vec<PathEdge>,
}

/// A node in a graph path result.
#[derive(Debug, Clone)]
pub struct PathNode {
    pub doc_id: i64,
    pub chunk_id: Option<i64>,
    pub relation_type: Option<RelationType>,
}

/// An edge in a graph path result.
#[derive(Debug, Clone)]
pub struct PathEdge {
    pub source_doc_id: i64,
    pub source_chunk_id: Option<i64>,
    pub relation_type: RelationType,
    pub target_doc_id: i64,
    pub target_chunk_id: Option<i64>,
    pub properties: Option<serde_json::Value>,
}

/// Path query: find paths from source to target with depth <= max_depth.
/// Returns all paths up to the specified depth.
pub fn path_query(
    storage: &dyn StorageEngine,
    source_doc_id: i64,
    target_doc_id: i64,
    max_depth: usize,
) -> SqlResult<Vec<GraphPath>> {
    if max_depth == 0 || max_depth > 3 {
        return Ok(Vec::new());
    }

    let all_relations: Vec<Relation> = {
        let rows = storage.scan(TABLE_RELATIONS)?;
        rows.into_iter()
            .filter_map(|r| Relation::from_row(&r))
            .collect()
    };

    let mut results = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut queue: Vec<(i64, Vec<(i64, Option<i64>)>, Vec<Relation>)> =
        vec![(source_doc_id, vec![(source_doc_id, None)], vec![])];

    while let Some((current_doc, path_nodes, path_edges)) = queue.pop() {
        if path_nodes.len() > max_depth {
            continue;
        }

        if current_doc == target_doc_id && !path_nodes.is_empty() {
            // Found a path
            let nodes: Vec<PathNode> = path_nodes
                .iter()
                .map(|(doc_id, chunk_id)| PathNode {
                    doc_id: *doc_id,
                    chunk_id: *chunk_id,
                    relation_type: None,
                })
                .collect();

            let edges: Vec<PathEdge> = path_edges
                .iter()
                .map(|r| PathEdge {
                    source_doc_id: r.source_doc_id.unwrap_or(0),
                    source_chunk_id: r.source_chunk_id,
                    relation_type: r.relation_type.clone(),
                    target_doc_id: r.target_doc_id.unwrap_or(0),
                    target_chunk_id: r.target_chunk_id,
                    properties: r.properties.clone(),
                })
                .collect();

            results.push(GraphPath { nodes, edges });
            continue;
        }

        // Explore neighbors
        for rel in &all_relations {
            let neighbor_doc = if rel.source_doc_id == Some(current_doc) {
                rel.target_doc_id
            } else if rel.target_doc_id == Some(current_doc) {
                rel.source_doc_id
            } else {
                continue;
            };

            let Some(neighbor_doc) = neighbor_doc else {
                continue;
            };

            let visit_key = (neighbor_doc, rel.relation_type.clone());
            if !visited.contains(&visit_key) && path_nodes.len() < max_depth {
                visited.insert(visit_key);
                let mut new_nodes = path_nodes.clone();
                new_nodes.push((neighbor_doc, rel.target_chunk_id.or(rel.source_chunk_id)));
                let mut new_edges = path_edges.clone();
                new_edges.push(rel.clone());
                queue.push((neighbor_doc, new_nodes, new_edges));
            }
        }
    }

    Ok(results)
}

/// Get all relations for a document.
pub fn get_relations_for_doc(storage: &dyn StorageEngine, doc_id: i64) -> SqlResult<Vec<Relation>> {
    let rows = storage.scan(TABLE_RELATIONS)?;
    let relations: Vec<Relation> = rows
        .into_iter()
        .filter_map(|r| Relation::from_row(&r))
        .filter(|rel| rel.source_doc_id == Some(doc_id) || rel.target_doc_id == Some(doc_id))
        .collect();
    Ok(relations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    #[test]
    fn test_insert_and_get_relations() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        let id = insert_relation(
            &mut storage,
            Some(1),
            None,
            &RelationType::Clause,
            Some(2),
            None,
            Some(&serde_json::json!({"strength": "MUST"})),
        )
        .unwrap();

        assert!(id > 0);

        let neighbors = get_neighbors(&storage, 1, Some(&RelationType::Clause)).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].relation_type, RelationType::Clause);
    }

    #[test]
    fn test_upsert_relation() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        let id1 = insert_relation(
            &mut storage,
            Some(1),
            None,
            &RelationType::Sop,
            Some(2),
            None,
            None,
        )
        .unwrap();

        let id2 = insert_relation(
            &mut storage,
            Some(1),
            None,
            &RelationType::Sop,
            Some(2),
            None,
            None,
        )
        .unwrap();

        assert_eq!(id1, id2); // Should be upsert, same ID

        let neighbors = get_neighbors(&storage, 1, None).unwrap();
        assert_eq!(neighbors.len(), 1); // Still only one relation
    }

    #[test]
    fn test_get_neighbors_filtered() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        insert_relation(
            &mut storage,
            Some(1),
            None,
            &RelationType::Clause,
            Some(2),
            None,
            None,
        )
        .unwrap();
        insert_relation(
            &mut storage,
            Some(1),
            None,
            &RelationType::Capa,
            Some(3),
            None,
            None,
        )
        .unwrap();

        let clause_neighbors = get_neighbors(&storage, 1, Some(&RelationType::Clause)).unwrap();
        assert_eq!(clause_neighbors.len(), 1);

        let all_neighbors = get_neighbors(&storage, 1, None).unwrap();
        assert_eq!(all_neighbors.len(), 2);
    }

    #[test]
    fn test_path_query() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        // A -> B -> C
        insert_relation(
            &mut storage,
            Some(1),
            None,
            &RelationType::Sop,
            Some(2),
            None,
            None,
        )
        .unwrap();
        insert_relation(
            &mut storage,
            Some(2),
            None,
            &RelationType::Capa,
            Some(3),
            None,
            None,
        )
        .unwrap();

        let paths = path_query(&storage, 1, 3, 3).unwrap();
        // Should find path 1 -> 2 -> 3
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_get_neighbors_chunk() {
        let mut storage = MemoryStorage::new();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        insert_relation(
            &mut storage,
            None,
            Some(100),
            &RelationType::AuditFinding,
            Some(2),
            None,
            None,
        )
        .unwrap();

        let neighbors = get_neighbors_chunk(&storage, 100).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].relation_type, RelationType::AuditFinding);
    }
}
