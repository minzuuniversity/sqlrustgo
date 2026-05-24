//! GMP Knowledge Graph Storage using SQLRustGo FileStorage
//!
//! This module provides storage for Layer 0 knowledge extraction data
//! using SQLRustGo FileStorage as the underlying engine.

use crate::types::*;
use anyhow::{Context, Result};
use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, Record, StorageEngine, TableInfo,
};
use sqlrustgo_types::Value;
use std::path::PathBuf;

/// GMP Knowledge Graph Storage
pub struct KgStorage {
    storage: FileStorage,
}

impl KgStorage {
    /// Create a new KgStorage with the given data directory
    /// Create a new KgStorage with the given data directory
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        let storage = FileStorage::new_with_buffer_config(data_dir.clone(), 0, false)
            .context("Failed to create FileStorage")?;

        let mut kg = Self { storage };
        kg.init_schema()?;
        Ok(kg)
    }

    /// Initialize the schema for Layer 0 tables
    fn init_schema(&mut self) -> Result<()> {
        // gmp_terms table
        let terms_table = TableInfo {
            name: "gmp_terms".to_string(),
            columns: vec![
                ColumnDefinition::new("term_id", "integer"),
                ColumnDefinition::new("canonical_term", "text"),
                ColumnDefinition::new("term_type", "text"),
                ColumnDefinition::new("synonyms", "text"),
                ColumnDefinition::new("hypernyms", "text"),
                ColumnDefinition::new("hyponyms", "text"),
                ColumnDefinition::new("gmp_chapter", "text"),
                ColumnDefinition::new("entity_type", "text"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            has_hidden_rowid: false,
            next_rowid: 0,
        };

        if !self.storage.has_table("gmp_terms") {
            self.storage
                .create_table(&terms_table)
                .map_err(|e| anyhow::anyhow!("Failed to create gmp_terms: {}", e))?;
        }

        // doc_entities table
        let entities_table = TableInfo {
            name: "doc_entities".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "integer"),
                ColumnDefinition::new("doc_id", "text"),
                ColumnDefinition::new("entity_type", "text"),
                ColumnDefinition::new("surface_form", "text"),
                ColumnDefinition::new("canonical_term", "text"),
                ColumnDefinition::new("start_pos", "integer"),
                ColumnDefinition::new("end_pos", "integer"),
                ColumnDefinition::new("confidence", "float"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            has_hidden_rowid: false,
            next_rowid: 0,
        };

        if !self.storage.has_table("doc_entities") {
            self.storage
                .create_table(&entities_table)
                .map_err(|e| anyhow::anyhow!("Failed to create doc_entities: {}", e))?;
        }

        // doc_relations table
        let relations_table = TableInfo {
            name: "doc_relations".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "integer"),
                ColumnDefinition::new("doc_id", "text"),
                ColumnDefinition::new("relation_type", "text"),
                ColumnDefinition::new("src_entity", "text"),
                ColumnDefinition::new("dst_entity", "text"),
                ColumnDefinition::new("confidence", "float"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            has_hidden_rowid: false,
            next_rowid: 0,
        };

        if !self.storage.has_table("doc_relations") {
            self.storage
                .create_table(&relations_table)
                .map_err(|e| anyhow::anyhow!("Failed to create doc_relations: {}", e))?;
        }

        // doc_taxonomy table
        let taxonomy_table = TableInfo {
            name: "doc_taxonomy".to_string(),
            columns: vec![
                ColumnDefinition::new("doc_id", "text"),
                ColumnDefinition::new("topic", "text"),
                ColumnDefinition::new("sub_topic", "text"),
                ColumnDefinition::new("doc_type", "text"),
                ColumnDefinition::new("regulation", "text"),
                ColumnDefinition::new("risk_level", "text"),
                ColumnDefinition::new("department", "text"),
                ColumnDefinition::new("confidence", "float"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            has_hidden_rowid: false,
            next_rowid: 0,
        };

        if !self.storage.has_table("doc_taxonomy") {
            self.storage
                .create_table(&taxonomy_table)
                .map_err(|e| anyhow::anyhow!("Failed to create doc_taxonomy: {}", e))?;
        }

        // doc_relevance table
        let relevance_table = TableInfo {
            name: "doc_relevance".to_string(),
            columns: vec![
                ColumnDefinition::new("doc_id", "text"),
                ColumnDefinition::new("overall_score", "float"),
                ColumnDefinition::new("term_density", "float"),
                ColumnDefinition::new("reg_citation", "float"),
                ColumnDefinition::new("process_completeness", "float"),
                ColumnDefinition::new("entity_density", "float"),
                ColumnDefinition::new("dept_coverage", "float"),
                ColumnDefinition::new("confidence", "float"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            has_hidden_rowid: false,
            next_rowid: 0,
        };

        if !self.storage.has_table("doc_relevance") {
            self.storage
                .create_table(&relevance_table)
                .map_err(|e| anyhow::anyhow!("Failed to create doc_relevance: {}", e))?;
        }

        // term_synonyms table (replaces JSON in gmp_terms.synonyms)
        let synonyms_table = TableInfo {
            name: "term_synonyms".to_string(),
            columns: vec![
                ColumnDefinition::new("term_id", "integer"),
                ColumnDefinition::new("synonym", "text"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            has_hidden_rowid: false,
            next_rowid: 0,
        };

        if !self.storage.has_table("term_synonyms") {
            self.storage
                .create_table(&synonyms_table)
                .map_err(|e| anyhow::anyhow!("Failed to create term_synonyms: {}", e))?;
        }

        // term_hierarchy table (replaces hypernyms/hyponyms in gmp_terms)
        let hierarchy_table = TableInfo {
            name: "term_hierarchy".to_string(),
            columns: vec![
                ColumnDefinition::new("term_id", "integer"),
                ColumnDefinition::new("parent_term", "text"),
                ColumnDefinition::new("hierarchy_type", "text"), // "hypernym" or "hyponym"
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            has_hidden_rowid: false,
            next_rowid: 0,
        };

        if !self.storage.has_table("term_hierarchy") {
            self.storage
                .create_table(&hierarchy_table)
                .map_err(|e| anyhow::anyhow!("Failed to create term_hierarchy: {}", e))?;
        }

        Ok(())
    }

    /// Insert a GMP term (synonyms/hierarchy go to separate tables)
    pub fn insert_term(&mut self, term: &GmpTerm) -> Result<()> {
        let record: Record = vec![
            Value::Integer(term.term_id),
            Value::Text(term.canonical_term.clone()),
            Value::Text(term.term_type.clone()),
            Value::Null, // synonyms now in term_synonyms table
            Value::Null, // hypernyms now in term_hierarchy table
            Value::Null, // hyponyms now in term_hierarchy table
            term.gmp_chapter
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            term.entity_type
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
        ];

        self.storage
            .insert("gmp_terms", vec![record])
            .map_err(|e| anyhow::anyhow!("Failed to insert term: {}", e))?;

        // Insert synonyms into term_synonyms table
        for synonym in &term.synonyms {
            let syn_record: Record = vec![
                Value::Integer(term.term_id),
                Value::Text(synonym.clone()),
            ];
            self.storage
                .insert("term_synonyms", vec![syn_record])
                .map_err(|e| anyhow::anyhow!("Failed to insert synonym: {}", e))?;
        }

        // Insert hypernyms into term_hierarchy table
        if let Some(ref hypernym) = term.hypernyms {
            let hier_record: Record = vec![
                Value::Integer(term.term_id),
                Value::Text(hypernym.clone()),
                Value::Text("hypernym".to_string()),
            ];
            self.storage
                .insert("term_hierarchy", vec![hier_record])
                .map_err(|e| anyhow::anyhow!("Failed to insert hypernym: {}", e))?;
        }

        // Insert hyponyms into term_hierarchy table
        if let Some(ref hyponym) = term.hyponyms {
            let hier_record: Record = vec![
                Value::Integer(term.term_id),
                Value::Text(hyponym.clone()),
                Value::Text("hyponym".to_string()),
            ];
            self.storage
                .insert("term_hierarchy", vec![hier_record])
                .map_err(|e| anyhow::anyhow!("Failed to insert hyponym: {}", e))?;
        }

        Ok(())
    }

    /// Scan all GMP terms (reconstructs synonyms/hierarchy from relation tables)
    pub fn scan_terms(&self) -> Result<Vec<GmpTerm>> {
        let records = self
            .storage
            .scan("gmp_terms")
            .map_err(|e| anyhow::anyhow!("Failed to scan terms: {}", e))?;

        let terms = records
            .into_iter()
            .map(|r| {
                let get_text = |v: &Value| match v {
                    Value::Text(s) => s.clone(),
                    _ => String::new(),
                };
                let get_opt_text = |v: &Value| {
                    if let Value::Text(s) = v {
                        if !s.is_empty() {
                            return Some(s.clone());
                        }
                    }
                    None
                };
                let term_id = r[0].as_integer().unwrap_or(0);

                // Reconstruct synonyms from term_synonyms table
                let synonyms = self.get_synonyms_for_term(term_id).unwrap_or_default();

                // Reconstruct hypernyms/hyponyms from term_hierarchy table
                let (hypernyms, hyponyms) = self.get_hierarchy_for_term(term_id);

                GmpTerm {
                    term_id,
                    canonical_term: get_text(&r[1]),
                    term_type: get_text(&r[2]),
                    synonyms,
                    hypernyms,
                    hyponyms,
                    gmp_chapter: get_opt_text(&r[6]),
                    entity_type: get_opt_text(&r[7]),
                }
            })
            .collect();

        Ok(terms)
    }

    /// Get synonyms for a specific term_id from term_synonyms table
    fn get_synonyms_for_term(&self, term_id: i64) -> Result<Vec<String>> {
        let all_records = self
            .storage
            .scan("term_synonyms")
            .map_err(|e| anyhow::anyhow!("Failed to scan term_synonyms: {}", e))?;

        let synonyms: Vec<String> = all_records
            .into_iter()
            .filter(|r| r[0].as_integer() == Some(term_id))
            .map(|r| {
                if let Value::Text(s) = &r[1] {
                    s.clone()
                } else {
                    String::new()
                }
            })
            .filter(|s| !s.is_empty())
            .collect();

        Ok(synonyms)
    }

    /// Get hypernyms and hyponyms for a specific term_id from term_hierarchy table
    fn get_hierarchy_for_term(&self, term_id: i64) -> (Option<String>, Option<String>) {
        let all_records = match self.storage.scan("term_hierarchy") {
            Ok(r) => r,
            Err(_) => return (None, None),
        };

        let mut hypernyms: Option<String> = None;
        let mut hyponyms: Option<String> = None;

        for r in all_records {
            if r[0].as_integer() != Some(term_id) {
                continue;
            }
            let hier_type = if let Value::Text(s) = &r[2] {
                s.clone()
            } else {
                continue;
            };
            let parent = if let Value::Text(s) = &r[1] {
                s.clone()
            } else {
                continue;
            };

            match hier_type.as_str() {
                "hypernym" if hypernyms.is_none() => hypernyms = Some(parent),
                "hyponym" if hyponyms.is_none() => hyponyms = Some(parent),
                _ => {}
            }
        }

        (hypernyms, hyponyms)
    }

    /// Insert a document entity
    pub fn insert_entity(&mut self, entity: &DocEntity) -> Result<()> {
        let record: Record = vec![
            Value::Integer(entity.id),
            Value::Text(entity.doc_id.clone()),
            Value::Text(entity.entity_type.clone()),
            Value::Text(entity.surface_form.clone()),
            Value::Text(entity.canonical_term.clone()),
            entity.start_pos.map(Value::Integer).unwrap_or(Value::Null),
            entity.end_pos.map(Value::Integer).unwrap_or(Value::Null),
            Value::Float(entity.confidence),
        ];

        self.storage
            .insert("doc_entities", vec![record])
            .map_err(|e| anyhow::anyhow!("Failed to insert entity: {}", e))?;
        Ok(())
    }

    /// Insert a document relation
    pub fn insert_relation(&mut self, relation: &DocRelation) -> Result<()> {
        let record: Record = vec![
            Value::Integer(relation.id),
            Value::Text(relation.doc_id.clone()),
            Value::Text(relation.relation_type.clone()),
            Value::Text(relation.src_entity.clone()),
            Value::Text(relation.dst_entity.clone()),
            Value::Float(relation.confidence),
        ];

        self.storage
            .insert("doc_relations", vec![record])
            .map_err(|e| anyhow::anyhow!("Failed to insert relation: {}", e))?;
        Ok(())
    }

    /// Insert or update a document taxonomy
    pub fn upsert_taxonomy(&mut self, taxonomy: &DocTaxonomy) -> Result<()> {
        let record: Record = vec![
            Value::Text(taxonomy.doc_id.clone()),
            taxonomy
                .topic
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            taxonomy
                .sub_topic
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            taxonomy
                .doc_type
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            taxonomy
                .regulation
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            taxonomy
                .risk_level
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            taxonomy
                .department
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            Value::Float(taxonomy.confidence),
        ];

        self.storage
            .insert("doc_taxonomy", vec![record])
            .map_err(|e| anyhow::anyhow!("Failed to upsert taxonomy: {}", e))?;
        Ok(())
    }

    /// Insert or update document relevance
    pub fn upsert_relevance(&mut self, relevance: &DocRelevance) -> Result<()> {
        let record: Record = vec![
            Value::Text(relevance.doc_id.clone()),
            Value::Float(relevance.overall_score),
            relevance.term_density.map(Value::Float).unwrap_or(Value::Null),
            relevance.reg_citation.map(Value::Float).unwrap_or(Value::Null),
            relevance
                .process_completeness
                .map(Value::Float)
                .unwrap_or(Value::Null),
            relevance.entity_density.map(Value::Float).unwrap_or(Value::Null),
            relevance.dept_coverage.map(Value::Float).unwrap_or(Value::Null),
            Value::Float(relevance.confidence),
        ];

        self.storage
            .insert("doc_relevance", vec![record])
            .map_err(|e| anyhow::anyhow!("Failed to upsert relevance: {}", e))?;
        Ok(())
    }

    /// Extract GMP entities from query text using seed terms and extraction rules
    pub fn get_stats(&self) -> Result<KgStats> {
        let terms_count = self.storage.scan("gmp_terms").map(|r| r.len()).unwrap_or(0);
        let synonyms_count = self.storage.scan("term_synonyms").map(|r| r.len()).unwrap_or(0);
        let hierarchy_count = self.storage.scan("term_hierarchy").map(|r| r.len()).unwrap_or(0);
        let entities_count = self
            .storage
            .scan("doc_entities")
            .map(|r| r.len())
            .unwrap_or(0);
        let relations_count = self
            .storage
            .scan("doc_relations")
            .map(|r| r.len())
            .unwrap_or(0);
        let taxonomy_count = self
            .storage
            .scan("doc_taxonomy")
            .map(|r| r.len())
            .unwrap_or(0);
        let relevance_count = self
            .storage
            .scan("doc_relevance")
            .map(|r| r.len())
            .unwrap_or(0);

        Ok(KgStats {
            terms_count,
            synonyms_count,
            hierarchy_count,
            entities_count,
            relations_count,
            taxonomy_count,
            relevance_count,
        })
    }

    /// Extract GMP entities from query text using seed terms and extraction rules
    /// Returns entities sorted by match length (longest match first)
    pub fn extract_entities(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        let terms = self.scan_terms()?;
        let mut entities = Vec::new();

        for term in &terms {
            if text.contains(&term.canonical_term) {
                entities.push(ExtractedEntity {
                    term: term.canonical_term.clone(),
                    term_type: term.term_type.clone(),
                    entity_type: term.entity_type.clone(),
                    confidence: 1.0,
                });
            }
            for synonym in &term.synonyms {
                if text.contains(synonym) {
                    entities.push(ExtractedEntity {
                        term: synonym.clone(),
                        term_type: term.term_type.clone(),
                        entity_type: term.entity_type.clone(),
                        confidence: 0.9, // slightly lower for synonyms
                    });
                }
            }
        }

        // Sort by longest match first (better specificity)
        entities.sort_by(|a, b| b.term.len().cmp(&a.term.len()));
        // Remove duplicates by term name
        entities.dedup_by(|a, b| a.term == b.term);

        Ok(entities)
    }
}

/// Statistics about stored knowledge graph data
#[derive(Debug, Clone)]
pub struct KgStats {
    pub terms_count: usize,
    pub synonyms_count: usize,
    pub hierarchy_count: usize,
    pub entities_count: usize,
    pub relations_count: usize,
    pub taxonomy_count: usize,
    pub relevance_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_creation() {
        let dir = tempdir().unwrap();
        let storage = KgStorage::new(dir.path().to_path_buf()).unwrap();

        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.terms_count, 0);
        assert_eq!(stats.entities_count, 0);
    }

    #[test]
    fn test_insert_and_scan_terms() {
        let dir = tempdir().unwrap();
        let mut storage = KgStorage::new(dir.path().to_path_buf()).unwrap();

        // Check stats before insert
        let stats_before = storage.get_stats().unwrap();
        eprintln!("Stats before insert: {:?}", stats_before);

        let term = GmpTerm {
            term_id: 1,
            canonical_term: "偏差".to_string(),
            term_type: "entity".to_string(),
            synonyms: vec!["偏离".to_string(), "异常".to_string()],
            hypernyms: Some("质量事件".to_string()),
            hyponyms: None,
            gmp_chapter: Some("第七章".to_string()),
            entity_type: Some("Deviation".to_string()),
        };

        let insert_result = storage.insert_term(&term);
        eprintln!("Insert result: {:?}", insert_result);

        let stats_after = storage.get_stats().unwrap();
        eprintln!("Stats after insert: {:?}", stats_after);

        let terms = storage.scan_terms().unwrap();
        eprintln!("Scanned terms: {:?}", terms);
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].canonical_term, "偏差");
    }
}

