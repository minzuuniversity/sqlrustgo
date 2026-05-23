//! Relation Extractor for GMP documents

use crate::relations::default_templates;
use crate::types::{GmpRelation, RelationResult, RelationType};
use std::time::Instant;

/// Relation Extractor for GMP domain
pub struct RelationExtractor {
    templates: Vec<crate::relations::RelationTemplate>,
}

impl RelationExtractor {
    pub fn new() -> Self {
        Self {
            templates: default_templates(),
        }
    }

    /// Extract relations from text
    pub fn extract(&self, doc_id: &str, text: &str) -> RelationResult {
        let start = Instant::now();
        let mut relations = Vec::new();

        for tmpl in &self.templates {
            // Find all matches
            for caps in tmpl.pattern.find_iter(text) {
                if let Some(groups) = tmpl.pattern.captures(caps.as_str()) {
                    let src = groups.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let dst = groups.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
                    
                    if !src.is_empty() && !dst.is_empty() {
                        relations.push(GmpRelation::new(
                            src,
                            dst,
                            tmpl.relation_type,
                            caps.start(),
                            caps.end(),
                        ));
                    }
                }
            }
        }

        // Sort by start position
        relations.sort_by_key(|r| r.start_pos);

        let elapsed = start.elapsed().as_millis() as u64;
        RelationResult::new(doc_id.to_string(), relations, elapsed)
    }

    /// Add a custom template
    pub fn add_template(&mut self, template: crate::relations::RelationTemplate) {
        self.templates.push(template);
    }

    /// Get all available templates
    pub fn templates(&self) -> &[crate::relations::RelationTemplate] {
        &self.templates
    }

    /// Count relations by type
    pub fn count_by_type(&self, doc_id: &str, text: &str) -> std::collections::HashMap<RelationType, usize> {
        let result = self.extract(doc_id, text);
        result.count_by_type()
    }
}

impl Default for RelationExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_deviation_capa() {
        let extractor = RelationExtractor::new();
        let text = "偏差2024-001触发CAPA-003，该CAPA需在30天内完成。";
        let result = extractor.extract("test-doc", text);
        
        let triggers: Vec<_> = result.by_type(RelationType::Triggers);
        assert!(!triggers.is_empty(), "Should find TRIGGERS relation");
    }

    #[test]
    fn test_extract_sop_department() {
        let extractor = RelationExtractor::new();
        let text = "偏差调查程序由质量部管理。";
        let result = extractor.extract("test-doc", text);
        
        let managed: Vec<_> = result.by_type(RelationType::ManagedBy);
        assert!(!managed.is_empty(), "Should find ManagedBy relation");
    }

    #[test]
    fn test_extract_iq_oq_pq() {
        let extractor = RelationExtractor::new();
        let text = "混合机需进行IQ和OQ验证。";
        let result = extractor.extract("test-doc", text);
        
        let requires: Vec<_> = result.by_type(RelationType::Requires);
        assert!(!requires.is_empty(), "Should find Requires relation");
    }

    #[test]
    fn test_no_relations() {
        let extractor = RelationExtractor::new();
        let text = "这是一段普通文本，没有关系。";
        let result = extractor.extract("test-doc", text);
        
        assert!(result.relations.is_empty(), "Should find no relations");
    }

    #[test]
    fn test_multiple_relations() {
        let extractor = RelationExtractor::new();
        let text = "重大偏差触发CAPA-001，偏差调查程序由质量部管理，混合机需进行IQ验证。";
        let result = extractor.extract("test-doc", text);
        
        println!("Found {} relations", result.relations.len());
        for rel in &result.relations {
            println!("  {:?}: {} -> {}", rel.relation_type, rel.src, rel.dst);
        }
        
        assert!(result.relations.len() >= 3, "Should find at least 3 relations");
    }
}
