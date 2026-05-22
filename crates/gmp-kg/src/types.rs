//! Types for GMP Knowledge Graph Layer

use serde::{Deserialize, Serialize};

/// GMP Term from the dictionary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmpTerm {
    pub term_id: i64,
    pub canonical_term: String,
    pub term_type: String,
    pub synonyms: Vec<String>,
    pub hypernyms: Option<String>,
    pub hyponyms: Option<String>,
    pub gmp_chapter: Option<String>,
    pub entity_type: Option<String>,
}

/// Entity extracted from a document via NER
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntity {
    pub id: i64,
    pub doc_id: String,
    pub entity_type: String,
    pub surface_form: String,
    pub canonical_term: String,
    pub start_pos: Option<i64>,
    pub end_pos: Option<i64>,
    pub confidence: f64,
}

/// Relation extracted from a document via RE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocRelation {
    pub id: i64,
    pub doc_id: String,
    pub relation_type: String,
    pub src_entity: String,
    pub dst_entity: String,
    pub confidence: f64,
}

/// Document taxonomy classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTaxonomy {
    pub doc_id: String,
    pub topic: Option<String>,
    pub sub_topic: Option<String>,
    pub doc_type: Option<String>,
    pub regulation: Option<String>,
    pub risk_level: Option<String>,
    pub department: Option<String>,
    pub confidence: f64,
}

/// Relevance score for a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocRelevance {
    pub doc_id: String,
    pub overall_score: f64,
    pub term_density: Option<f64>,
    pub reg_citation: Option<f64>,
    pub process_completeness: Option<f64>,
    pub entity_density: Option<f64>,
    pub dept_coverage: Option<f64>,
    pub confidence: f64,
}
