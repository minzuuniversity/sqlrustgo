use serde::{Deserialize, Serialize};

/// Relevance scoring dimensions for a GMP document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScore {
    /// Overall relevance score (0.0 ~ 1.0)
    pub overall_score: f32,
    /// Density of GMP terms (0.0 ~ 1.0)
    pub term_density: f32,
    /// Regulatory citation score (0.0 ~ 1.0)
    pub reg_citation: f32,
    /// Process completeness score (0.0 ~ 1.0)
    pub process_completeness: f32,
    /// Named entity density (0.0 ~ 1.0)
    pub entity_density: f32,
    /// Department coverage (0.0 ~ 1.0)
    pub dept_coverage: f32,
    /// Confidence in the scoring (0.0 ~ 1.0)
    pub confidence: f32,
}

impl Default for RelevanceScore {
    fn default() -> Self {
        Self {
            overall_score: 0.0,
            term_density: 0.0,
            reg_citation: 0.0,
            process_completeness: 0.0,
            entity_density: 0.0,
            dept_coverage: 0.0,
            confidence: 0.0,
        }
    }
}

/// Scoring weights for each dimension
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub term_density: f32,
    pub reg_citation: f32,
    pub process_completeness: f32,
    pub entity_density: f32,
    pub dept_coverage: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        // GMP regulatory weights
        Self {
            term_density: 0.25,      // GMP terminology usage
            reg_citation: 0.25,       // Regulatory reference
            process_completeness: 0.20, // Process documentation
            entity_density: 0.15,     // Named entities
            dept_coverage: 0.15,      // Department assignment
        }
    }
}

/// GMP term categories for scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermStats {
    pub total_terms: usize,
    pub unique_terms: usize,
    pub entity_terms: usize,
    pub process_terms: usize,
    pub regulation_terms: usize,
    pub document_terms: usize,
}

/// Regulatory reference stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegStats {
    pub gmp_articles: usize,
    pub chapter_refs: usize,
    pub other_regs: usize,
}

/// Process completeness indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStats {
    pub has_procedure: bool,
    pub has_responsibility: bool,
    pub has_acceptance_criteria: bool,
    pub has_records: bool,
    pub has_attachment: bool,
}

/// Document metadata for scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocMetadata {
    pub doc_id: String,
    pub title: String,
    pub content: String,
    pub doc_type: Option<String>,
    pub department: Option<String>,
    pub term_stats: TermStats,
    pub reg_stats: RegStats,
    pub process_stats: ProcessStats,
    pub entity_count: usize,
}

impl Default for TermStats {
    fn default() -> Self {
        Self {
            total_terms: 0,
            unique_terms: 0,
            entity_terms: 0,
            process_terms: 0,
            regulation_terms: 0,
            document_terms: 0,
        }
    }
}

impl Default for RegStats {
    fn default() -> Self {
        Self {
            gmp_articles: 0,
            chapter_refs: 0,
            other_regs: 0,
        }
    }
}

impl Default for ProcessStats {
    fn default() -> Self {
        Self {
            has_procedure: false,
            has_responsibility: false,
            has_acceptance_criteria: false,
            has_records: false,
            has_attachment: false,
        }
    }
}

impl Default for DocMetadata {
    fn default() -> Self {
        Self {
            doc_id: String::new(),
            title: String::new(),
            content: String::new(),
            doc_type: None,
            department: None,
            term_stats: TermStats::default(),
            reg_stats: RegStats::default(),
            process_stats: ProcessStats::default(),
            entity_count: 0,
        }
    }
}
