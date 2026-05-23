//! Relation types for GMP domain

use serde::{Deserialize, Serialize};

/// GMP relation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// Belongs to (e.g., SOP belongs to Chapter)
    BelongsTo,
    /// Managed by (e.g., SOP managed by Department)
    ManagedBy,
    /// Triggers (e.g., Deviation triggers CAPA)
    Triggers,
    /// Requires (e.g., Device requires Validation)
    Requires,
    /// Precedes (e.g., SOP A precedes SOP B)
    Precedes,
    /// Mandates (e.g., Regulation mandates SOP)
    Mandates,
    /// Generates (e.g., AuditFinding generates CAPA)
    Generates,
    /// Verified by (e.g., CAPA verified by Validation)
    VerifiedBy,
    /// Follows SOP (e.g., Operator follows SOP)
    FollowsSop,
    /// Authorized for (e.g., Operator authorized for Device)
    AuthorizedFor,
    /// Trained on (e.g., Operator trained on SOP)
    TrainedOn,
    /// Part of (e.g., Material part of Product)
    PartOf,
    /// Contains (e.g., Chapter contains SOPs)
    Contains,
    /// Related to (generic relation)
    RelatedTo,
    /// Unknown
    Unknown,
}

impl RelationType {
    pub fn display_name(&self) -> &'static str {
        match self {
            RelationType::BelongsTo => "属于",
            RelationType::ManagedBy => "由...管理",
            RelationType::Triggers => "触发",
            RelationType::Requires => "需要",
            RelationType::Precedes => "先于",
            RelationType::Mandates => "规定",
            RelationType::Generates => "产生",
            RelationType::VerifiedBy => "由...验证",
            RelationType::FollowsSop => "遵循",
            RelationType::AuthorizedFor => "授权操作",
            RelationType::TrainedOn => "培训合格",
            RelationType::PartOf => "组成",
            RelationType::Contains => "包含",
            RelationType::RelatedTo => "关联",
            RelationType::Unknown => "未知",
        }
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A relation between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmpRelation {
    /// Source entity surface form
    pub src: String,
    /// Destination entity surface form
    pub dst: String,
    /// Relation type
    pub relation_type: RelationType,
    /// Start position in text
    pub start_pos: usize,
    /// End position in text
    pub end_pos: usize,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

impl GmpRelation {
    pub fn new(
        src: String,
        dst: String,
        relation_type: RelationType,
        start_pos: usize,
        end_pos: usize,
    ) -> Self {
        Self {
            src,
            dst,
            relation_type,
            start_pos,
            end_pos,
            confidence: 1.0,
        }
    }

    pub fn with_confidence(
        src: String,
        dst: String,
        relation_type: RelationType,
        start_pos: usize,
        end_pos: usize,
        confidence: f32,
    ) -> Self {
        Self {
            src,
            dst,
            relation_type,
            start_pos,
            end_pos,
            confidence,
        }
    }
}

/// Result of relation extraction from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationResult {
    /// Document ID
    pub doc_id: String,
    /// Extracted relations
    pub relations: Vec<GmpRelation>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

impl RelationResult {
    pub fn new(doc_id: String, relations: Vec<GmpRelation>, processing_time_ms: u64) -> Self {
        Self {
            doc_id,
            relations,
            processing_time_ms,
        }
    }

    pub fn by_type(&self, relation_type: RelationType) -> Vec<&GmpRelation> {
        self.relations.iter().filter(|r| r.relation_type == relation_type).collect()
    }

    pub fn count_by_type(&self) -> std::collections::HashMap<RelationType, usize> {
        let mut counts = std::collections::HashMap::new();
        for relation in &self.relations {
            *counts.entry(relation.relation_type).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_type_display() {
        assert_eq!(RelationType::Triggers.display_name(), "触发");
        assert_eq!(RelationType::Requires.display_name(), "需要");
    }

    #[test]
    fn test_relation_creation() {
        let relation = GmpRelation::new(
            "偏差2024-001".to_string(),
            "CAPA-001".to_string(),
            RelationType::Triggers,
            0,
            20,
        );
        assert_eq!(relation.src, "偏差2024-001");
        assert_eq!(relation.relation_type, RelationType::Triggers);
        assert_eq!(relation.confidence, 1.0);
    }
}
