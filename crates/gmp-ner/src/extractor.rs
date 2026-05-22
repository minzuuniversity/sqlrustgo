//! NER Extractor for GMP documents

use crate::terms::{TermDictionary, DEFAULT_DICTIONARY};
use crate::types::{Entity, EntityType, ExtractionResult};
use regex::Regex;
use std::time::Instant;

/// NER Extractor for GMP domain
pub struct NerExtractor {
    dictionary: TermDictionary,
    /// Regex patterns for structured entities (SOP codes, deviation IDs, etc.)
    structured_patterns: Vec<(Regex, EntityType)>,
}

impl NerExtractor {
    /// Create a new extractor with default dictionary
    pub fn new() -> Self {
        Self {
            dictionary: DEFAULT_DICTIONARY.clone(),
            structured_patterns: Self::build_structured_patterns(),
        }
    }

    /// Create with custom dictionary
    pub fn with_dictionary(dictionary: TermDictionary) -> Self {
        Self {
            dictionary,
            structured_patterns: Self::build_structured_patterns(),
        }
    }

    /// Build regex patterns for structured entities
    fn build_structured_patterns() -> Vec<(Regex, EntityType)> {
        vec![
            // SOP codes: SOP-MFG-101, SOP-001, etc.
            (Regex::new(r"SOP-[A-Z]+-\d+").unwrap(), EntityType::Sop),
            (Regex::new(r"SOP-\d+").unwrap(), EntityType::Sop),
            // Deviation IDs: DEV-001, 偏差2024-001
            (Regex::new(r"DEV-[A-Z]+-\d+").unwrap(), EntityType::Deviation),
            (Regex::new(r"偏差\d{4}-\d+").unwrap(), EntityType::Deviation),
            // Validation IDs: VAL-001
            (Regex::new(r"VAL-[A-Z]+-\d+").unwrap(), EntityType::Validation),
            // Change control: CC-2024-001
            (Regex::new(r"CC-\d{4}-\d+").unwrap(), EntityType::ChangeControl),
            // CAPA IDs: CAPA-001
            (Regex::new(r"CAPA-\d+").unwrap(), EntityType::Capa),
            // Equipment: DEV-EQ-001
            (Regex::new(r"DEV-[A-Z]+-\d+").unwrap(), EntityType::Equipment),
            // IQ/OQ/PQ/DQ mentions
            (Regex::new(r"\b(IQ|OQ|PQ|DQ)\b").unwrap(), EntityType::Validation),
        ]
    }

    /// Extract entities from text
    pub fn extract(&self, doc_id: &str, text: &str) -> ExtractionResult {
        let start = Instant::now();
        let mut entities = Vec::new();

        // 1. Extract structured entities (codes, IDs)
        for (pattern, entity_type) in &self.structured_patterns {
            for m in pattern.find_iter(text) {
                entities.push(Entity::new(
                    m.as_str().to_string(),
                    m.as_str().to_string(),
                    *entity_type,
                    m.start(),
                    m.end(),
                ));
            }
        }

        // 2. Dictionary-based extraction
        for entry in self.dictionary.canonical_terms() {
            let term = &entry.canonical_term;
            let entity_type = entry.entity_type;
            
            // Case-insensitive search
            let re = Regex::new(&format!("(?i){}", regex::escape(term))).unwrap();
            for m in re.find_iter(text) {
                // Avoid overlapping with already found entities
                let overlaps = entities.iter().any(|e: &Entity| {
                    (m.start() >= e.start_pos && m.start() < e.end_pos) ||
                    (m.end() > e.start_pos && m.end() <= e.end_pos)
                });
                if !overlaps {
                    entities.push(Entity::new(
                        m.as_str().to_string(),
                        entry.canonical_term.clone(),
                        entity_type,
                        m.start(),
                        m.end(),
                    ));
                }
            }

            // Also search synonyms
            for syn in &entry.synonyms {
                let re = Regex::new(&format!("(?i){}", regex::escape(syn))).unwrap();
                for m in re.find_iter(text) {
                    let overlaps = entities.iter().any(|e| {
                        (m.start() >= e.start_pos && m.start() < e.end_pos) ||
                        (m.end() > e.start_pos && m.end() <= e.end_pos)
                    });
                    if !overlaps {
                        entities.push(Entity::new(
                            m.as_str().to_string(),
                            entry.canonical_term.clone(),
                            entity_type,
                            m.start(),
                            m.end(),
                        ));
                    }
                }
            }
        }

        // Sort by start position
        entities.sort_by_key(|e| e.start_pos);

        let elapsed = start.elapsed().as_millis() as u64;
        ExtractionResult::new(doc_id.to_string(), entities, elapsed)
    }

    /// Extract entities from multiple chunks of a document
    pub fn extract_chunks(&self, doc_id: &str, chunks: &[&str]) -> ExtractionResult {
        let start = Instant::now();
        let mut all_entities = Vec::new();
        let mut offset = 0;

        for chunk in chunks {
            let result = self.extract(doc_id, chunk);
            // Adjust positions
            for mut entity in result.entities {
                entity.start_pos += offset;
                entity.end_pos += offset;
                all_entities.push(entity);
            }
            offset += chunk.len();
        }

        let elapsed = start.elapsed().as_millis() as u64;
        ExtractionResult::new(doc_id.to_string(), all_entities, elapsed)
    }
}

impl Default for NerExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_sop_code() {
        let extractor = NerExtractor::new();
        let text = "根据 SOP-MFG-101 进行清洁验证";
        let result = extractor.extract("test-doc", text);
        
        let sop_entities: Vec<_> = result.by_type(EntityType::Sop);
        assert!(!sop_entities.is_empty(), "Should find SOP-MFG-101");
        assert_eq!(sop_entities[0].surface_form, "SOP-MFG-101");
    }

    #[test]
    fn test_extract_deviation() {
        let extractor = NerExtractor::new();
        let text = "偏差2024-001 已启动调查";
        let result = extractor.extract("test-doc", text);
        
        let dev_entities: Vec<_> = result.by_type(EntityType::Deviation);
        assert!(!dev_entities.is_empty(), "Should find deviation");
    }

    #[test]
    fn test_extract_from_dictionary() {
        let extractor = NerExtractor::new();
        let text = "该偏差属于重大偏差，需要启动CAPA";
        let result = extractor.extract("test-doc", text);
        
        assert!(!result.by_type(EntityType::Deviation).is_empty());
        assert!(!result.by_type(EntityType::Capa).is_empty());
    }

    #[test]
    fn test_extract_iq_oq_pq() {
        let extractor = NerExtractor::new();
        let text = "设备 IQ 已完成，OQ 正在进行中";
        let result = extractor.extract("test-doc", text);
        
        let val_entities: Vec<_> = result.by_type(EntityType::Validation);
        assert_eq!(val_entities.len(), 2);
    }

    #[test]
    fn test_no_overlapping_entities() {
        let extractor = NerExtractor::new();
        let text = "重大偏差 - 偏差2024-001";
        let result = extractor.extract("test-doc", text);
        
        // Should not have overlapping entities
        for (i, e1) in result.entities.iter().enumerate() {
            for e2 in result.entities.iter().skip(i + 1) {
                assert!(
                    e1.end_pos <= e2.start_pos || e2.end_pos <= e1.start_pos,
                    "Entities should not overlap: {:?} and {:?}", e1, e2
                );
            }
        }
    }
}
