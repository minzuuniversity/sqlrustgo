//! Node types for provenance graph

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Equipment,
    Operator,
    SOP,
    Batch,
    Workflow,
    Deviation,
    CAPA,
    Signature,
    SignatureChain,
    AuditRecord,
    Instrument,
    Material,
    ProcessStep,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equipment => "equipment",
            Self::Operator => "operator",
            Self::SOP => "sop",
            Self::Batch => "batch",
            Self::Workflow => "workflow",
            Self::Deviation => "deviation",
            Self::CAPA => "capa",
            Self::Signature => "signature",
            Self::SignatureChain => "signature_chain",
            Self::AuditRecord => "audit_record",
            Self::Instrument => "instrument",
            Self::Material => "material",
            Self::ProcessStep => "process_step",
        }
    }
}

pub trait GMPNode {
    fn node_id(&self) -> &str;
    fn node_type(&self) -> NodeType;
}
