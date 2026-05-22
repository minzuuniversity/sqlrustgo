//! Document Classifier for GMP documents

use crate::taxonomy::Taxonomy;
use crate::types::{ClassificationResult, DocTaxonomy};
use std::time::Instant;

/// Document Classifier for GMP domain
pub struct DocClassifier {
    taxonomy: Taxonomy,
}

impl DocClassifier {
    pub fn new() -> Self {
        Self {
            taxonomy: Taxonomy::new(),
        }
    }

    /// Classify a document based on its title and content
    pub fn classify(&self, doc_id: &str, title: &str, content: &str) -> ClassificationResult {
        let start = Instant::now();
        
        // Combine title (with higher weight) and content
        let text = format!("{} {}", title, content);
        
        // Get classification results
        let results = self.taxonomy.classify_top(&text);
        let matched_rules: Vec<String> = self.taxonomy.classify(&text)
            .into_iter()
            .flat_map(|(_, rules)| rules.into_iter().map(|(r, _)| r.id.to_string()))
            .collect();
        
        // Build DocTaxonomy
        let mut taxonomy = DocTaxonomy::new();
        
        if let Some(topic) = results.get("topic") {
            taxonomy.topic = Some(topic.clone());
        }
        if let Some(sub_topic) = results.get("sub_topic") {
            taxonomy.sub_topic = Some(sub_topic.clone());
        }
        if let Some(doc_type) = results.get("doc_type") {
            taxonomy.doc_type = Some(doc_type.clone());
        }
        if let Some(regulation) = results.get("regulation") {
            taxonomy.regulation = Some(regulation.clone());
        }
        if let Some(risk_level) = results.get("risk_level") {
            taxonomy.risk_level = Some(risk_level.clone());
        }
        if let Some(department) = results.get("department") {
            taxonomy.department = Some(department.clone());
        }
        
        // Set confidence based on how many dimensions were matched
        let dim_count = taxonomy.topic.is_some() as u8
            + taxonomy.doc_type.is_some() as u8
            + taxonomy.risk_level.is_some() as u8
            + taxonomy.department.is_some() as u8;
        taxonomy.confidence = (dim_count as f32) * 0.25; // 0.25 per matched dimension
        
        let elapsed = start.elapsed().as_millis() as u64;
        
        ClassificationResult::new(doc_id.to_string(), taxonomy, elapsed)
            .with_matched_rules(matched_rules)
    }

    /// Classify based on title only (fast path)
    pub fn classify_title(&self, doc_id: &str, title: &str) -> ClassificationResult {
        self.classify(doc_id, title, "")
    }
}

impl Default for DocClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_deviation_sop() {
        let classifier = DocClassifier::new();
        let result = classifier.classify(
            "DEV-001",
            "重大偏差调查处理程序 SOP-001",
            "本程序规定了偏差的调查、分析和处理流程...",
        );
        
        assert!(result.taxonomy.topic.is_some());
        assert_eq!(result.taxonomy.doc_type.as_deref(), Some("SOP"));
        println!("Taxonomy: {:?}", result.taxonomy);
    }

    #[test]
    fn test_classify_validation_sop() {
        let classifier = DocClassifier::new();
        let result = classifier.classify(
            "VAL-001",
            "清洁验证操作规程 SOP-101",
            "本验证方案规定了设备清洁验证的具体要求...",
        );
        
        assert_eq!(result.taxonomy.topic.as_deref(), Some("验证"));
        assert_eq!(result.taxonomy.doc_type.as_deref(), Some("SOP"));
    }

    #[test]
    fn test_classify_equipment_sop() {
        let classifier = DocClassifier::new();
        let result = classifier.classify(
            "EQ-001",
            "设备维护保养规程",
            "设备定期维护保养的具体要求和操作步骤...",
        );
        
        assert_eq!(result.taxonomy.topic.as_deref(), Some("设备"));
    }

    #[test]
    fn test_classify_high_risk() {
        let classifier = DocClassifier::new();
        let result = classifier.classify(
            "RISK-001",
            "高风险偏差调查报告",
            "关于关键设备故障导致的重大偏差调查...",
        );
        
        assert_eq!(result.taxonomy.risk_level.as_deref(), Some("高"));
    }

    #[test]
    fn test_classify_department() {
        let classifier = DocClassifier::new();
        let result = classifier.classify(
            "DOC-001",
            "质量部文件管理规程",
            "质量部负责文件的编制、审核和批准...",
        );
        
        assert_eq!(result.taxonomy.department.as_deref(), Some("质量部"));
    }

    #[test]
    fn test_confidence_scoring() {
        let classifier = DocClassifier::new();
        
        // Only title
        let result1 = classifier.classify_title("DOC-001", "偏差调查报告");
        println!("Title-only confidence: {}", result1.taxonomy.confidence);
        
        // Full content
        let result2 = classifier.classify(
            "DOC-002",
            "偏差调查报告",
            "重大偏差调查 - 质量部负责偏差调查和处理...",
        );
        println!("Full content confidence: {}", result2.taxonomy.confidence);
        
        assert!(result2.taxonomy.confidence >= result1.taxonomy.confidence);
    }
}
