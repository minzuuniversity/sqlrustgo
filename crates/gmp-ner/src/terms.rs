//! Term dictionary for GMP NER

use crate::types::EntityType;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

/// A term entry in the dictionary
#[derive(Debug, Clone)]
pub struct TermEntry {
    pub canonical_term: String,
    pub entity_type: EntityType,
    pub synonyms: Vec<String>,
    pub hypernym: Option<String>,
}

impl TermEntry {
    pub fn new(canonical_term: &str, entity_type: EntityType) -> Self {
        Self {
            canonical_term: canonical_term.to_string(),
            entity_type,
            synonyms: Vec::new(),
            hypernym: None,
        }
    }

    pub fn with_synonyms(mut self, synonyms: Vec<&str>) -> Self {
        self.synonyms = synonyms.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_hypernym(mut self, hypernym: &str) -> Self {
        self.hypernym = Some(hypernym.to_string());
        self
    }
}

/// Term dictionary for GMP domain
#[derive(Clone)]
pub struct TermDictionary {
    /// Map from term -> entry
    terms: HashMap<String, TermEntry>,
    /// Regex patterns for entity types
    patterns: HashMap<EntityType, Vec<Regex>>,
}

impl TermDictionary {
    /// Create a new term dictionary with default GMP terms
    pub fn new() -> Self {
        let mut dict = Self {
            terms: HashMap::new(),
            patterns: HashMap::new(),
        };
        dict.load_default_terms();
        dict.compile_patterns();
        dict
    }

    /// Load default GMP terms
    fn load_default_terms(&mut self) {
        // Deviation types
        let deviation_terms = vec![
            ("偏差", vec!["偏离", "异常", "偏差事件", "偏差情况"]),
            ("重大偏差", vec!["严重偏差", "重要偏差"]),
            ("一般偏差", vec!["轻微偏差", "微小偏差", "次要偏差"]),
            ("微小偏差", vec![]),
        ];
        for (term, synonyms) in deviation_terms {
            self.add_term(TermEntry::new(term, EntityType::Deviation).with_synonyms(synonyms));
        }

        // CAPA
        let capa_terms = vec![
            ("CAPA", vec!["纠正预防措施", "纠正和预防措施"]),
            ("纠正措施", vec!["纠正"]),
            ("预防措施", vec!["预防"]),
            ("CAPA计划", vec!["纠正预防计划"]),
        ];
        for (term, synonyms) in capa_terms {
            self.add_term(TermEntry::new(term, EntityType::Capa).with_synonyms(synonyms));
        }

        // Change control
        let change_terms = vec![
            ("变更控制", vec!["变更管理", "变更申请"]),
            ("工艺变更", vec!["生产工艺变更"]),
            ("设备变更", vec![]),
            ("物料变更", vec![]),
            ("重大变更", vec!["重要变更", "实质性变更"]),
            ("一般变更", vec!["行政变更", "小型变更"]),
        ];
        for (term, synonyms) in change_terms {
            self.add_term(TermEntry::new(term, EntityType::ChangeControl).with_synonyms(synonyms));
        }

        // Validation
        let validation_terms = vec![
            ("验证", vec!["确认", "qualification"]),
            ("工艺验证", vec!["过程验证"]),
            ("清洁验证", vec!["清洗验证"]),
            ("设备确认", vec!["设备验证"]),
            ("IQ", vec!["安装确认"]),
            ("OQ", vec!["运行确认"]),
            ("PQ", vec!["性能确认"]),
            ("DQ", vec!["设计确认", "设计鉴定"]),
            ("再确认", vec!["再验证"]),
            ("验证方案", vec!["确认方案", "验证计划"]),
            ("验证报告", vec!["确认报告"]),
        ];
        for (term, synonyms) in validation_terms {
            self.add_term(TermEntry::new(term, EntityType::Validation).with_synonyms(synonyms));
        }

        // Equipment
        let equipment_terms = vec![
            ("生产设备", vec![]),
            ("检验设备", vec!["实验室设备"]),
            ("公用设备", vec!["公共设备"]),
            ("关键设备", vec!["重要设备"]),
            ("校准", vec!["计量"]),
            ("维护", vec!["保养"]),
        ];
        for (term, synonyms) in equipment_terms {
            self.add_term(TermEntry::new(term, EntityType::Equipment).with_synonyms(synonyms));
        }

        // Material
        let material_terms = vec![
            ("原料药", vec!["API", "活性药物成分"]),
            ("辅料", vec!["添加剂"]),
            ("中间体", vec![]),
            ("物料管理", vec!["物料控制"]),
        ];
        for (term, synonyms) in material_terms {
            self.add_term(TermEntry::new(term, EntityType::Material).with_synonyms(synonyms));
        }

        // SOP/Document
        let doc_terms = vec![
            ("SOP", vec!["标准操作规程", "操作规程"]),
            ("管理规程", vec!["管理文件"]),
            ("批记录", vec!["批生产记录", "EBR"]),
            ("批号", vec!["生产批号", "批次号"]),
            ("文件管理", vec![]),
            ("文件编码", vec!["文件编号"]),
        ];
        for (term, synonyms) in doc_terms {
            self.add_term(TermEntry::new(term, EntityType::Document).with_synonyms(synonyms));
        }

        // Risk
        let risk_terms = vec![
            ("风险评估", vec!["风险分析"]),
            ("FMEA", vec!["失效模式与效应分析"]),
            ("HAZOP", vec!["危害与可操作性分析"]),
            ("RPN", vec!["风险优先数"]),
            ("高风险", vec![]),
            ("中风险", vec![]),
            ("低风险", vec![]),
        ];
        for (term, synonyms) in risk_terms {
            self.add_term(TermEntry::new(term, EntityType::RiskLevel).with_synonyms(synonyms));
        }

        // Person roles
        let person_terms = vec![
            ("质量授权人", vec!["QA负责人", "质量负责人"]),
            ("生产负责人", vec!["生产经理"]),
            ("受权人", vec!["授权人"]),
        ];
        for (term, synonyms) in person_terms {
            self.add_term(TermEntry::new(term, EntityType::PersonRole).with_synonyms(synonyms));
        }

        // Audit
        let audit_terms = vec![
            ("内部审计", vec!["内审"]),
            ("外部审计", vec!["外审"]),
            ("审计计划", vec!["审核计划"]),
            ("审计报告", vec!["审核报告"]),
            ("审计发现", vec!["审核发现"]),
            ("供应商审计", vec!["供货商审核"]),
            ("自查", vec!["自我检查"]),
        ];
        for (term, synonyms) in audit_terms {
            self.add_term(TermEntry::new(term, EntityType::Regulation).with_synonyms(synonyms));
        }

        // Facility
        let facility_terms = vec![
            ("洁净区", vec!["洁净室", "清洁区域"]),
            ("洁净厂房", vec!["洁净工厂"]),
            ("D级区", vec!["D级洁净区"]),
            ("C级区", vec!["C级洁净区"]),
            ("B级区", vec!["B级洁净区"]),
            ("A级区", vec!["A级洁净区"]),
        ];
        for (term, synonyms) in facility_terms {
            self.add_term(TermEntry::new(term, EntityType::Facility).with_synonyms(synonyms));
        }

        // Department
        let dept_terms = vec![
            ("生产部", vec![]),
            ("质量部", vec!["质量管理部"]),
            ("工程部", vec!["设备工程部"]),
            ("仓储部", vec!["仓库"]),
            ("采购部", vec![]),
        ];
        for (term, synonyms) in dept_terms {
            self.add_term(TermEntry::new(term, EntityType::Department).with_synonyms(synonyms));
        }
    }

    /// Add a term to the dictionary
    pub fn add_term(&mut self, entry: TermEntry) {
        // Add canonical term
        self.terms.insert(entry.canonical_term.clone(), entry.clone());
        // Add synonyms
        for syn in &entry.synonyms {
            self.terms.insert(syn.clone(), entry.clone());
        }
    }

    /// Compile regex patterns for entity types
    fn compile_patterns(&mut self) {
        // Pattern for SOP/规程 numbers
        let sop_pattern = Regex::new(r"SOP-[A-Z]+-\d+|SOP-\d+|操作规程[编号]?\d+|[管理]规程\d+").unwrap();
        self.patterns.entry(EntityType::Sop).or_default().push(sop_pattern);

        // Pattern for deviation numbers
        let dev_pattern = Regex::new(r"偏差[编号]?\d*|[偏离异常]告?\d*").unwrap();
        self.patterns.entry(EntityType::Deviation).or_default().push(dev_pattern);

        // Pattern for validation phases
        let val_pattern = Regex::new(r"\b(IQ|OQ|PQ|DQ)\b").unwrap();
        self.patterns.entry(EntityType::Validation).or_default().push(val_pattern);

        // Pattern for risk levels
        let risk_pattern = Regex::new(r"[高、中、低]风险|[高、中、低]优先级").unwrap();
        self.patterns.entry(EntityType::RiskLevel).or_default().push(risk_pattern);

        // Pattern for equipment codes
        let equip_pattern = Regex::new(r"[设备机]器编号?\d+|DEV-[A-Z]+-\d+").unwrap();
        self.patterns.entry(EntityType::Equipment).or_default().push(equip_pattern);

        // Pattern for material codes
        let mat_pattern = Regex::new(r"物料编号?\d+|MAT-\d+|原料编号?\d+").unwrap();
        self.patterns.entry(EntityType::Material).or_default().push(mat_pattern);
    }

    /// Look up a term
    pub fn lookup(&self, term: &str) -> Option<&TermEntry> {
        self.terms.get(term)
    }

    /// Check if a term exists
    pub fn contains(&self, term: &str) -> bool {
        self.terms.contains_key(term)
    }

    /// Get all terms of a specific type
    pub fn get_by_type(&self, entity_type: EntityType) -> Vec<&TermEntry> {
        self.terms.values().filter(|e| e.entity_type == entity_type).collect()
    }

    /// Get all canonical terms (no synonyms)
    pub fn canonical_terms(&self) -> Vec<&TermEntry> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for entry in self.terms.values() {
            if !seen.contains(&entry.canonical_term) {
                seen.insert(entry.canonical_term.clone());
                result.push(entry);
            }
        }
        result
    }
}

impl Default for TermDictionary {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    /// Global default term dictionary
    pub static ref DEFAULT_DICTIONARY: TermDictionary = TermDictionary::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_lookup() {
        let dict = TermDictionary::new();
        
        // Test canonical term
        assert!(dict.contains("偏差"));
        assert!(dict.contains("重大偏差"));
        
        // Test synonym
        assert!(dict.contains("偏离"));
        assert!(dict.contains("异常"));
        
        // Test non-existent
        assert!(!dict.contains("根本不存在的词"));
    }

    #[test]
    fn test_term_entry() {
        let dict = TermDictionary::new();
        let entry = dict.lookup("偏差").unwrap();
        assert_eq!(entry.canonical_term, "偏差");
        assert_eq!(entry.entity_type, EntityType::Deviation);
    }
}
