//! Document taxonomy types for GMP domain

use serde::{Deserialize, Serialize};

/// A document taxonomy (multi-dimensional classification)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocTaxonomy {
    /// Topic classification (偏差/变更/验证/设备/物料/文件/人员/厂房)
    pub topic: Option<String>,
    /// Sub-topic (偏差调查/CAPA/偏差分级/...)
    pub sub_topic: Option<String>,
    /// Document type (SOP/记录/报告/规程/法规/标准)
    pub doc_type: Option<String>,
    /// Regulation system (GMP/EU GMP/FDA/WHO/中国GMP)
    pub regulation: Option<String>,
    /// Risk level (高/中/低)
    pub risk_level: Option<String>,
    /// Department (生产部/质量部/工程部/仓储部)
    pub department: Option<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

impl DocTaxonomy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_topic(mut self, topic: &str) -> Self {
        self.topic = Some(topic.to_string());
        self
    }

    pub fn with_doc_type(mut self, doc_type: &str) -> Self {
        self.doc_type = Some(doc_type.to_string());
        self
    }

    pub fn with_risk_level(mut self, level: &str) -> Self {
        self.risk_level = Some(level.to_string());
        self
    }

    pub fn with_department(mut self, dept: &str) -> Self {
        self.department = Some(dept.to_string());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

/// Result of document classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub doc_id: String,
    pub taxonomy: DocTaxonomy,
    pub processing_time_ms: u64,
    pub matched_rules: Vec<String>,
}

impl ClassificationResult {
    pub fn new(doc_id: String, taxonomy: DocTaxonomy, processing_time_ms: u64) -> Self {
        Self {
            doc_id,
            taxonomy,
            processing_time_ms,
            matched_rules: Vec::new(),
        }
    }

    pub fn with_matched_rules(mut self, rules: Vec<String>) -> Self {
        self.matched_rules = rules;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxonomy_creation() {
        let taxonomy = DocTaxonomy::new()
            .with_topic("偏差")
            .with_doc_type("SOP")
            .with_risk_level("高")
            .with_confidence(0.95);
        
        assert_eq!(taxonomy.topic, Some("偏差".to_string()));
        assert_eq!(taxonomy.doc_type, Some("SOP".to_string()));
        assert_eq!(taxonomy.risk_level, Some("高".to_string()));
        assert_eq!(taxonomy.confidence, 0.95);
    }

    #[test]
    fn test_default_taxonomy() {
        let taxonomy = DocTaxonomy::default();
        assert!(taxonomy.topic.is_none());
        assert!(taxonomy.doc_type.is_none());
        assert_eq!(taxonomy.confidence, 0.0);
    }
}
