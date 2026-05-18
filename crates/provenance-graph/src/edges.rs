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
