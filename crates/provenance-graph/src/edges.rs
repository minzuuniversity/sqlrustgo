//! Edge types for provenance graph

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    OperatedBy,
    FollowsSOP,
    Contains,
    ExecutedBy,
    TriggeredBy,
    ApprovedBy,
    Documents,
    SignedBy,
    CorrectedBy,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OperatedBy => "operated_by",
            Self::FollowsSOP => "follows_sop",
            Self::Contains => "contains",
            Self::ExecutedBy => "executed_by",
            Self::TriggeredBy => "triggered_by",
            Self::ApprovedBy => "approved_by",
            Self::Documents => "documents",
            Self::SignedBy => "signed_by",
            Self::CorrectedBy => "corrected_by",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_type_as_str() {
        assert_eq!(EdgeType::OperatedBy.as_str(), "operated_by");
        assert_eq!(EdgeType::FollowsSOP.as_str(), "follows_sop");
        assert_eq!(EdgeType::Contains.as_str(), "contains");
        assert_eq!(EdgeType::ExecutedBy.as_str(), "executed_by");
        assert_eq!(EdgeType::TriggeredBy.as_str(), "triggered_by");
        assert_eq!(EdgeType::ApprovedBy.as_str(), "approved_by");
        assert_eq!(EdgeType::Documents.as_str(), "documents");
        assert_eq!(EdgeType::SignedBy.as_str(), "signed_by");
        assert_eq!(EdgeType::CorrectedBy.as_str(), "corrected_by");
    }

    #[test]
    fn test_edge_type_equality() {
        assert_eq!(EdgeType::OperatedBy, EdgeType::OperatedBy);
        assert_eq!(EdgeType::SignedBy, EdgeType::SignedBy);
        assert_ne!(EdgeType::OperatedBy, EdgeType::SignedBy);
    }

    #[test]
    fn test_edge_creation() {
        let edge = Edge {
            from: "node1".to_string(),
            to: "node2".to_string(),
            edge_type: EdgeType::Contains,
        };
        assert_eq!(edge.from, "node1");
        assert_eq!(edge.to, "node2");
        assert_eq!(edge.edge_type, EdgeType::Contains);
    }
}
