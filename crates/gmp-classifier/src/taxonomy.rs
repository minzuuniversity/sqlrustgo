//! GMP Document Taxonomy definitions

use regex::Regex;
use std::collections::HashMap;

/// A classification rule
#[derive(Debug, Clone)]
pub struct ClassificationRule {
    /// Rule ID
    pub id: &'static str,
    /// Dimension (topic, doc_type, risk_level, department)
    pub dimension: &'static str,
    /// Pattern to match
    pub pattern: Regex,
    /// Value to assign if matched
    pub value: &'static str,
    /// Priority (higher = more specific)
    pub priority: u32,
}

impl ClassificationRule {
    pub fn new(id: &'static str, dimension: &'static str, pattern: &str, value: &'static str, priority: u32) -> Self {
        Self {
            id,
            dimension,
            pattern: Regex::new(pattern).unwrap(),
            value,
            priority,
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        self.pattern.is_match(text)
    }
}

/// GMP Document Taxonomy
pub struct Taxonomy {
    rules: Vec<ClassificationRule>,
}

impl Taxonomy {
    pub fn new() -> Self {
        Self {
            rules: Self::build_rules(),
        }
    }

    fn build_rules() -> Vec<ClassificationRule> {
        vec![
            // ============ Topic Rules ============
            // 偏差
            ClassificationRule::new("topic_deviation_1", "topic", 
                r"偏差", "偏差", 10),
            ClassificationRule::new("topic_deviation_2", "topic", 
                r"偏离|异常", "偏差", 5),
            
            // 变更
            ClassificationRule::new("topic_change_1", "topic", 
                r"变更控制|变更申请|变更管理", "变更控制", 10),
            ClassificationRule::new("topic_change_2", "topic", 
                r"变更", "变更控制", 5),
            
            // 验证
            ClassificationRule::new("topic_validation_1", "topic", 
                r"验证方案|验证报告|确认方案", "验证", 10),
            ClassificationRule::new("topic_validation_2", "topic", 
                r"IQ|OQ|PQ|DQ|工艺验证|清洁验证|设备确认", "验证", 8),
            ClassificationRule::new("topic_validation_3", "topic", 
                r"验证|确认", "验证", 3),
            
            // 设备
            ClassificationRule::new("topic_equipment_1", "topic", 
                r"设备管理|设备维护|设备校准|设备确认", "设备", 10),
            ClassificationRule::new("topic_equipment_2", "topic", 
                r"设备|机器", "设备", 3),
            
            // 物料
            ClassificationRule::new("topic_material_1", "topic", 
                r"物料管理|物料控制|仓储管理", "物料", 10),
            ClassificationRule::new("topic_material_2", "topic", 
                r"物料|原料|辅料|成品", "物料", 3),
            
            // 文件
            ClassificationRule::new("topic_document_1", "topic", 
                r"文件管理|文档管理|SOP管理", "文件", 10),
            ClassificationRule::new("topic_document_2", "topic", 
                r"批记录|记录表格|表单", "文件", 5),
            
            // 人员
            ClassificationRule::new("topic_personnel_1", "topic", 
                r"人员培训|培训管理|资质确认", "人员", 10),
            ClassificationRule::new("topic_personnel_2", "topic", 
                r"人员|培训|授权", "人员", 3),
            
            // 厂房
            ClassificationRule::new("topic_facility_1", "topic", 
                r"厂房设施|洁净区|环境监控", "厂房", 10),
            ClassificationRule::new("topic_facility_2", "topic", 
                r"洁净区|压差|温湿度|HVAC", "厂房", 5),
            
            // ============ Document Type Rules ============
            ClassificationRule::new("doctype_sop", "doc_type", 
                r"SOP[-_]?\d+|标准操作规程|操作规程[编号]?\d*", "SOP", 20),
            ClassificationRule::new("doctype_sop_2", "doc_type", 
                r"管理规程|管理文件", "管理规程", 15),
            ClassificationRule::new("doctype_record", "doc_type", 
                r"批生产记录|批记录|检验记录|培训记录|维护记录", "记录", 15),
            ClassificationRule::new("doctype_report", "doc_type", 
                r"验证报告|确认报告|调查报告|审计报告|总结报告", "报告", 15),
            ClassificationRule::new("doctype_plan", "doc_type", 
                r"验证方案|确认方案|培训计划|审计计划", "方案", 15),
            ClassificationRule::new("doctypeRegulation", "doc_type", 
                r"GMP[第]?\d*[条章]?|药典|法规", "法规", 10),
            
            // ============ Risk Level Rules ============
            ClassificationRule::new("risk_high", "risk_level", 
                r"重大偏差|关键设备|关键工艺参数|无菌|高风险", "高", 20),
            ClassificationRule::new("risk_medium", "risk_level", 
                r"一般偏差|重要|中等", "中", 15),
            ClassificationRule::new("risk_low", "risk_level", 
                r"微小偏差|低风险|次要", "低", 15),
            
            // ============ Department Rules ============
            ClassificationRule::new("dept_qa", "department", 
                r"质量部|质量管理部|QA|质量保证", "质量部", 20),
            ClassificationRule::new("dept_production", "department", 
                r"生产部|生产管理部|生产车间", "生产部", 20),
            ClassificationRule::new("dept_engineering", "department", 
                r"工程部|设备工程部|工程维修", "工程部", 20),
            ClassificationRule::new("dept_warehouse", "department", 
                r"仓储部|仓库|物料管理", "仓储部", 20),
            ClassificationRule::new("dept_purchase", "department", 
                r"采购部|供应部", "采购部", 20),
            
            // ============ Sub-topic Rules ============
            ClassificationRule::new("sub_deviation_investigation", "sub_topic", 
                r"偏差调查|偏差处理|偏差分析", "偏差调查", 15),
            ClassificationRule::new("sub_deviation_capa", "sub_topic", 
                r"CAPA|纠正预防", "CAPA管理", 15),
            ClassificationRule::new("sub_deviation_classification", "sub_topic", 
                r"偏差分级|偏差分类", "偏差分级", 15),
            ClassificationRule::new("sub_change_request", "sub_topic", 
                r"变更申请|变更请求", "变更申请", 15),
            ClassificationRule::new("sub_change_assessment", "sub_topic", 
                r"变更评估|风险评估", "变更评估", 15),
            ClassificationRule::new("sub_validation_iq", "sub_topic", 
                r"IQ|安装确认", "安装确认", 15),
            ClassificationRule::new("sub_validation_oq", "sub_topic", 
                r"OQ|运行确认", "运行确认", 15),
            ClassificationRule::new("sub_validation_pq", "sub_topic", 
                r"PQ|性能确认", "性能确认", 15),
        ]
    }

    /// Classify text and return all matched rules
    pub fn classify(&self, text: &str) -> HashMap<&'static str, Vec<(&ClassificationRule, &str)>> {
        let mut results: HashMap<&'static str, Vec<(&ClassificationRule, &str)>> = HashMap::new();
        
        for rule in &self.rules {
            if rule.matches(text) {
                results
                    .entry(rule.dimension)
                    .or_default()
                    .push((rule, rule.value));
            }
        }
        
        results
    }

    /// Get the highest priority match for each dimension
    pub fn classify_top(&self, text: &str) -> HashMap<String, String> {
        let matches = self.classify(text);
        let mut result = HashMap::new();
        
        for (dimension, rules) in matches {
            // Sort by priority descending
            let mut sorted: Vec<_> = rules;
            sorted.sort_by(|a, b| b.0.priority.cmp(&a.0.priority));
            
            if let Some((_rule, value)) = sorted.first() {
                // Only use if we don't already have a higher priority match
                result.entry(dimension.to_string()).or_insert_with(|| value.to_string());
            }
        }
        
        result
    }

    pub fn rules(&self) -> &[ClassificationRule] {
        &self.rules
    }
}

impl Default for Taxonomy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_topic() {
        let taxonomy = Taxonomy::new();
        
        let result = taxonomy.classify_top("偏差调查报告 - 重大偏差调查处理程序");
        
        assert_eq!(result.get("topic").map(|s| s.as_str()), Some("偏差"));
        assert!(result.contains_key("doc_type") || result.get("doc_type").is_some());
    }

    #[test]
    fn test_classify_sop() {
        let taxonomy = Taxonomy::new();
        
        let result = taxonomy.classify_top("SOP-MFG-101 清洁验证操作规程");
        
        assert_eq!(result.get("doc_type").map(|s| s.as_str()), Some("SOP"));
    }

    #[test]
    fn test_classify_risk() {
        let taxonomy = Taxonomy::new();
        
        let result = taxonomy.classify_top("重大偏差处理 - 关键设备故障");
        
        assert_eq!(result.get("risk_level").map(|s| s.as_str()), Some("高"));
    }

    #[test]
    fn test_classify_department() {
        let taxonomy = Taxonomy::new();
        
        let result = taxonomy.classify_top("质量部偏差调查报告");
        
        assert_eq!(result.get("department").map(|s| s.as_str()), Some("质量部"));
    }

    #[test]
    fn test_multiple_rules() {
        let taxonomy = Taxonomy::new();
        
        let text = "质量部偏差调查报告 - 重大偏差调查";
        let result = taxonomy.classify(text);
        
        println!("Matched {} dimensions", result.len());
        for (dim, rules) in &result {
            println!("  {}: {:?}", dim, rules);
        }
        
        assert!(!result.is_empty());
    }
}
