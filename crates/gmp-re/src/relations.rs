//! Predefined relation templates for GMP domain

use crate::types::RelationType;
use regex::Regex;

/// A relation extraction template
#[derive(Debug, Clone)]
pub struct RelationTemplate {
    /// Regex pattern to match the text containing the relation
    pub pattern: Regex,
    /// Relation type
    pub relation_type: RelationType,
    /// Source entity type hint
    pub src_type: &'static str,
    /// Destination entity type hint
    pub dst_type: &'static str,
    /// Description of what this template captures
    pub description: &'static str,
}

impl RelationTemplate {
    pub fn new(pattern: &str, relation_type: RelationType, src_type: &'static str, dst_type: &'static str, description: &'static str) -> Self {
        Self {
            pattern: Regex::new(pattern).unwrap(),
            relation_type,
            src_type,
            dst_type,
            description,
        }
    }

    /// Check if text matches this template and return captures
    pub fn try_match<'a>(&'a self, text: &'a str) -> Option<RelationCaptures<'a>> {
        self.pattern.captures(text).map(|caps| RelationCaptures {
            src: caps.get(1).map(|m| m.as_str()).unwrap_or(""),
            dst: caps.get(2).map(|m| m.as_str()).unwrap_or(""),
            full_match: caps.get(0).map(|m| (m.start(), m.end())),
        })
    }
}

/// Captured groups from a relation template match
#[derive(Debug, Clone)]
pub struct RelationCaptures<'a> {
    pub src: &'a str,
    pub dst: &'a str,
    pub full_match: Option<(usize, usize)>,
}

/// Default GMP relation templates
pub fn default_templates() -> Vec<RelationTemplate> {
    vec![
        // Deviation -> CAPA (触发关系)
        RelationTemplate::new(
            r"([^\s]{2,20})触发[到个]?([A-Z]{2,}-?\d+)",
            RelationType::Triggers,
            "Deviation",
            "CAPA",
            "偏差触发CAPA",
        ),
        RelationTemplate::new(
            r"([^\s]{2,20})引发[起]?([A-Z]{2,}-?\d+)",
            RelationType::Triggers,
            "Deviation",
            "CAPA",
            "偏差引发CAPA",
        ),
        RelationTemplate::new(
            r"([^\s]{2,20})导致([A-Z]{2,}-?\d+)",
            RelationType::Triggers,
            "Deviation",
            "CAPA",
            "偏差导致CAPA",
        ),
        
        // SOP -> Department (属于/管理关系)
        RelationTemplate::new(
            r"([^\s]{2,10})由(生产部|质量部|工程部|仓储部|采购部|人事部)管理",
            RelationType::ManagedBy,
            "SOP",
            "Department",
            "SOP由部门管理",
        ),
        RelationTemplate::new(
            r"([^\s]{2,10})属于(第[一二三四五六七八九十\d]+章)",
            RelationType::BelongsTo,
            "SOP",
            "Chapter",
            "SOP属于某章节",
        ),
        
        // Device -> Validation (需要验证关系)
        RelationTemplate::new(
            r"([^\s]{2,10})(?:设备|机器)需要(IQ|OQ|PQ|确认|验证)",
            RelationType::Requires,
            "Equipment",
            "Validation",
            "设备需要验证",
        ),
        RelationTemplate::new(
            r"([^\s]{2,10})需进行(IQ|OQ|PQ|DQ|验证|确认)",
            RelationType::Requires,
            "Equipment",
            "Validation",
            "需要进行验证",
        ),
        
        // Regulation -> SOP (规定关系)
        RelationTemplate::new(
            r"(GMP[第]?\d*[条章节]?)[规]?定[了]?([^\s]{2,10})",
            RelationType::Mandates,
            "Regulation",
            "SOP",
            "GMP规定某SOP",
        ),
        RelationTemplate::new(
            r"([^\s]{2,10})符合(GMP[第]?\d*[条章节]?)",
            RelationType::Mandates,
            "SOP",
            "Regulation",
            "符合GMP规定",
        ),
        
        // Audit Finding -> CAPA
        RelationTemplate::new(
            r"审计发现[的]?([^\s]{2,10})启动([A-Z]{2,}-?\d+)",
            RelationType::Generates,
            "AuditFinding",
            "CAPA",
            "审计发现启动CAPA",
        ),
        RelationTemplate::new(
            r"([^\s]{2,10})需采取([A-Z]{2,}-?\d+)",
            RelationType::Generates,
            "AuditFinding",
            "CAPA",
            "需采取CAPA措施",
        ),
        
        // CAPA -> Validation (验证关系)
        RelationTemplate::new(
            r"([A-Z]{2,}-?\d+)效果[的]?通过(验证|确认|检查)",
            RelationType::VerifiedBy,
            "CAPA",
            "Validation",
            "CAPA效果验证",
        ),
        RelationTemplate::new(
            r"([A-Z]{2,}-?\d+)经(验证|确认)有效",
            RelationType::VerifiedBy,
            "CAPA",
            "Validation",
            "CAPA经验证有效",
        ),
        
        // Operator -> SOP (培训/遵循关系)
        RelationTemplate::new(
            r"([^\s]{2,6})已培训[的]?([^\s]{2,10})",
            RelationType::TrainedOn,
            "Operator",
            "SOP",
            "人员已培训某SOP",
        ),
        RelationTemplate::new(
            r"([^\s]{2,6})授权[的]?([^\s]{2,10})",
            RelationType::AuthorizedFor,
            "Operator",
            "SOP",
            "人员授权某SOP",
        ),
        
        // SOP -> SOP (流程顺序关系)
        RelationTemplate::new(
            r"([^\s]{2,10})应先于([^\s]{2,10})",
            RelationType::Precedes,
            "SOP",
            "SOP",
            "某SOP应先于另一SOP",
        ),
        RelationTemplate::new(
            r"进行([^\s]{2,10})后[方]?再执行([^\s]{2,10})",
            RelationType::Precedes,
            "SOP",
            "SOP",
            "先执行A再执行B",
        ),
        
        // Change -> Deviation
        RelationTemplate::new(
            r"([^\s]{2,10})导致([^\s]{2,10})偏差",
            RelationType::Triggers,
            "ChangeControl",
            "Deviation",
            "变更导致偏差",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_match() {
        let templates = default_templates();
        
        // Test deviation triggers CAPA
        let text = "重大偏差触发CAPA-001";
        for tmpl in &templates {
            if tmpl.relation_type == RelationType::Triggers {
                if let Some(caps) = tmpl.try_match(text) {
                    assert!(!caps.src.is_empty());
                    println!("Matched: {} -> {}", caps.src, caps.dst);
                }
            }
        }
    }
}
