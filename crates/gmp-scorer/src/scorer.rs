use crate::types::*;

/// GMP Document Relevance Scorer
pub struct RelevanceScorer {
    weights: ScoringWeights,
    /// Content length for normalization
    min_content_len: usize,
    /// Maximum expected term count for normalization
    max_term_count: usize,
    /// Maximum expected entity count for normalization
    max_entity_count: usize,
}

impl Default for RelevanceScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl RelevanceScorer {
    pub fn new() -> Self {
        Self {
            weights: ScoringWeights::default(),
            min_content_len: 100,
            max_term_count: 50,
            max_entity_count: 30,
        }
    }

    /// Score a document based on its metadata
    pub fn score(&self, meta: &DocMetadata) -> RelevanceScore {
        // Calculate individual dimension scores
        let term_density = self.calc_term_density(meta);
        let reg_citation = self.calc_reg_citation(meta);
        let process_completeness = self.calc_process_completeness(meta);
        let entity_density = self.calc_entity_density(meta);
        let dept_coverage = self.calc_dept_coverage(meta);

        // Calculate weighted overall score
        let overall_score = 
            term_density * self.weights.term_density +
            reg_citation * self.weights.reg_citation +
            process_completeness * self.weights.process_completeness +
            entity_density * self.weights.entity_density +
            dept_coverage * self.weights.dept_coverage;

        // Confidence is based on content length and available metadata
        let confidence = self.calc_confidence(meta);

        RelevanceScore {
            overall_score,
            term_density,
            reg_citation,
            process_completeness,
            entity_density,
            dept_coverage,
            confidence,
        }
    }

    /// Calculate term density score (0.0 ~ 1.0)
    fn calc_term_density(&self, meta: &DocMetadata) -> f32 {
        let content_len = meta.content.len();
        if content_len < self.min_content_len {
            return 0.0;
        }

        let total_terms = meta.term_stats.total_terms as f32;
        let unique_terms = meta.term_stats.unique_terms as f32;
        
        // Term density = unique_terms / total_terms (vocabulary richness)
        // Combined with raw term count normalized
        let raw_density = (total_terms / content_len as f32) * 1000.0; // terms per 1000 chars
        let richness = if total_terms > 0.0 {
            unique_terms / total_terms
        } else {
            0.0
        };

        // Normalize raw density (expected max ~20 terms per 1000 chars)
        let normalized = (raw_density / 20.0).min(1.0);
        
        // Combine with richness
        (normalized * 0.6 + richness * 0.4).min(1.0)
    }

    /// Calculate regulatory citation score (0.0 ~ 1.0)
    fn calc_reg_citation(&self, meta: &DocMetadata) -> f32 {
        let gmp = meta.reg_stats.gmp_articles as f32;
        let chapter = meta.reg_stats.chapter_refs as f32;
        let other = meta.reg_stats.other_regs as f32;

        // Weighted regulatory score
        let score = (gmp * 2.0 + chapter * 1.5 + other * 1.0) / 10.0;
        score.min(1.0)
    }

    /// Calculate process completeness score (0.0 ~ 1.0)
    fn calc_process_completeness(&self, meta: &DocMetadata) -> f32 {
        let p = &meta.process_stats;
        
        // Each indicator contributes to completeness
        let indicators = [
            (p.has_procedure, 0.25),
            (p.has_responsibility, 0.20),
            (p.has_acceptance_criteria, 0.25),
            (p.has_records, 0.20),
            (p.has_attachment, 0.10),
        ];

        let mut score = 0.0f32;
        for (present, weight) in indicators {
            if present {
                score += weight;
            }
        }
        score
    }

    /// Calculate entity density score (0.0 ~ 1.0)
    fn calc_entity_density(&self, meta: &DocMetadata) -> f32 {
        let content_len = meta.content.len();
        if content_len < self.min_content_len {
            return 0.0;
        }

        let entity_count = meta.entity_count as f32;
        let normalized = (entity_count / self.max_entity_count as f32).min(1.0);
        normalized
    }

    /// Calculate department coverage score (0.0 ~ 1.0)
    fn calc_dept_coverage(&self, meta: &DocMetadata) -> f32 {
        match &meta.department {
            Some(_) => 1.0,  // Has department = full score
            None => 0.0,     // No department = zero
        }
    }

    /// Calculate confidence in the scoring (0.0 ~ 1.0)
    fn calc_confidence(&self, meta: &DocMetadata) -> f32 {
        let content_len = meta.content.len();
        
        // Content length factor
        let len_factor = if content_len < self.min_content_len {
            content_len as f32 / self.min_content_len as f32
        } else {
            1.0
        };

        // Metadata availability factor
        let metadata_factor = match &meta.doc_type {
            Some(_) => 1.0,
            None => 0.5,
        };

        (len_factor * 0.6 + metadata_factor * 0.4).min(1.0)
    }

    /// Batch score multiple documents
    pub fn score_batch(&self, docs: &[DocMetadata]) -> Vec<RelevanceScore> {
        docs.iter().map(|doc| self.score(doc)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_density_score() {
        let scorer = RelevanceScorer::new();
        
        // High term density
        let meta = DocMetadata {
            doc_id: "TEST-001".into(),
            title: "偏差调查程序".into(),
            content: "本偏差处理程序规定了偏差的调查、分析和处理流程。偏差分为重大偏差和一般偏差，需要进行偏差分级和CAPA处理。".into(),
            doc_type: Some("SOP".into()),
            department: Some("质量部".into()),
            term_stats: TermStats {
                total_terms: 15,
                unique_terms: 12,
                entity_terms: 5,
                process_terms: 8,
                regulation_terms: 2,
                document_terms: 0,
            },
            reg_stats: RegStats::default(),
            process_stats: ProcessStats {
                has_procedure: true,
                has_responsibility: true,
                has_acceptance_criteria: true,
                has_records: true,
                has_attachment: false,
            },
            entity_count: 8,
        };

        let score = scorer.score(&meta);
        assert!(score.overall_score > 0.5, "High quality doc should score > 0.5");
        assert_eq!(score.term_density > 0.0, true);
    }

    #[test]
    fn test_low_content_score() {
        let scorer = RelevanceScorer::new();
        
        // Low content
        let meta = DocMetadata {
            doc_id: "TEST-002".into(),
            title: "Test".into(),
            content: "Short".into(),
            doc_type: None,
            department: None,
            term_stats: TermStats::default(),
            reg_stats: RegStats::default(),
            process_stats: ProcessStats::default(),
            entity_count: 0,
        };

        let score = scorer.score(&meta);
        assert!(score.overall_score < 0.3, "Low quality doc should score < 0.3");
    }

    #[test]
    fn test_dept_coverage() {
        let scorer = RelevanceScorer::new();
        
        let meta_with_dept = DocMetadata {
            doc_id: "TEST-003".into(),
            title: "Test".into(),
            content: "Test content for department coverage".into(),
            doc_type: None,
            department: Some("质量部".into()),
            term_stats: TermStats::default(),
            reg_stats: RegStats::default(),
            process_stats: ProcessStats::default(),
            entity_count: 0,
        };

        let score_with = scorer.score(&meta_with_dept);
        assert_eq!(score_with.dept_coverage, 1.0);

        let meta_without = DocMetadata {
            department: None,
            ..meta_with_dept.clone()
        };
        let score_without = scorer.score(&meta_without);
        assert_eq!(score_without.dept_coverage, 0.0);
    }

    #[test]
    fn test_reg_citation() {
        let scorer = RelevanceScorer::new();
        
        let meta = DocMetadata {
            doc_id: "TEST-004".into(),
            title: "Test".into(),
            content: "Test content".into(),
            doc_type: None,
            department: None,
            term_stats: TermStats::default(),
            reg_stats: RegStats {
                gmp_articles: 5,
                chapter_refs: 3,
                other_regs: 2,
            },
            process_stats: ProcessStats::default(),
            entity_count: 0,
        };

        let score = scorer.score(&meta);
        // (5*2 + 3*1.5 + 2*1) / 10 = (10 + 4.5 + 2) / 10 = 1.65, capped at 1.0
        assert_eq!(score.reg_citation, 1.0);
    }
}
