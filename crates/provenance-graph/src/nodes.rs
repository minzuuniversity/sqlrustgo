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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_type_as_str() {
        assert_eq!(NodeType::Equipment.as_str(), "equipment");
        assert_eq!(NodeType::Operator.as_str(), "operator");
        assert_eq!(NodeType::SOP.as_str(), "sop");
        assert_eq!(NodeType::Batch.as_str(), "batch");
        assert_eq!(NodeType::Workflow.as_str(), "workflow");
        assert_eq!(NodeType::Deviation.as_str(), "deviation");
        assert_eq!(NodeType::CAPA.as_str(), "capa");
        assert_eq!(NodeType::Signature.as_str(), "signature");
        assert_eq!(NodeType::SignatureChain.as_str(), "signature_chain");
        assert_eq!(NodeType::AuditRecord.as_str(), "audit_record");
        assert_eq!(NodeType::Instrument.as_str(), "instrument");
        assert_eq!(NodeType::Material.as_str(), "material");
        assert_eq!(NodeType::ProcessStep.as_str(), "process_step");
    }

    #[test]
    fn test_node_type_equality() {
        assert_eq!(NodeType::Batch, NodeType::Batch);
        assert_eq!(NodeType::Operator, NodeType::Operator);
        assert_ne!(NodeType::Batch, NodeType::Operator);
    }

    #[test]
    fn test_node_type_copy() {
        let nt = NodeType::Equipment;
        let nt2 = nt;
        assert_eq!(nt, nt2);
    }
}
