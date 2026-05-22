//! Entity types for GMP domain

use serde::{Deserialize, Serialize};

/// GMP entity types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Sop,
    Deviation,
    Capa,
    ChangeControl,
    Equipment,
    Material,
    Validation,
    Regulation,
    PersonRole,
    Document,
    RiskLevel,
    Facility,
    Department,
    Unknown,
}

impl EntityType {
    pub fn display_name(&self) -> &'static str {
        match self {
            EntityType::Sop => "SOP/规程",
            EntityType::Deviation => "偏差",
            EntityType::Capa => "CAPA",
            EntityType::ChangeControl => "变更控制",
            EntityType::Equipment => "设备",
            EntityType::Material => "物料",
            EntityType::Validation => "验证",
            EntityType::Regulation => "法规条款",
            EntityType::PersonRole => "人员角色",
            EntityType::Document => "文档",
            EntityType::RiskLevel => "风险等级",
            EntityType::Facility => "厂房设施",
            EntityType::Department => "部门",
            EntityType::Unknown => "未知",
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A recognized entity from text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub surface_form: String,
    pub canonical_term: String,
    pub entity_type: EntityType,
    pub start_pos: usize,
    pub end_pos: usize,
    pub confidence: f32,
}

impl Entity {
    pub fn new(
        surface_form: String,
        canonical_term: String,
        entity_type: EntityType,
        start_pos: usize,
        end_pos: usize,
    ) -> Self {
        Self {
            surface_form,
            canonical_term,
            entity_type,
            start_pos,
            end_pos,
            confidence: 1.0,
        }
    }

    pub fn with_confidence(
        surface_form: String,
        canonical_term: String,
        entity_type: EntityType,
        start_pos: usize,
        end_pos: usize,
        confidence: f32,
    ) -> Self {
        Self {
            surface_form,
            canonical_term,
            entity_type,
            start_pos,
            end_pos,
            confidence,
        }
    }
}

/// Result of entity extraction from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub doc_id: String,
    pub entities: Vec<Entity>,
    pub processing_time_ms: u64,
}

impl ExtractionResult {
    pub fn new(doc_id: String, entities: Vec<Entity>, processing_time_ms: u64) -> Self {
        Self {
            doc_id,
            entities,
            processing_time_ms,
        }
    }

    pub fn by_type(&self, entity_type: EntityType) -> Vec<&Entity> {
        self.entities.iter().filter(|e| e.entity_type == entity_type).collect()
    }

    pub fn count_by_type(&self) -> std::collections::HashMap<EntityType, usize> {
        let mut counts = std::collections::HashMap::new();
        for entity in &self.entities {
            *counts.entry(entity.entity_type).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_display() {
        assert_eq!(EntityType::Deviation.display_name(), "偏差");
        assert_eq!(EntityType::Sop.display_name(), "SOP/规程");
    }

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(
            "重大偏差".to_string(),
            "重大偏差".to_string(),
            EntityType::Deviation,
            0,
            4,
        );
        assert_eq!(entity.surface_form, "重大偏差");
        assert_eq!(entity.entity_type, EntityType::Deviation);
        assert_eq!(entity.confidence, 1.0);
    }
}
