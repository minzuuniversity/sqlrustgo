//! Compliance dashboard

#[derive(Debug, Clone)]
pub struct ComplianceDashboard {
    pub summary: ComplianceSummary,
    pub recent_violations: Vec<ViolationItem>,
    pub evidence_chains: Vec<EvidenceChainSummary>,
}

#[derive(Debug, Clone)]
pub struct ComplianceSummary {
    pub total_batches: u32,
    pub compliant_batches: u32,
    pub violations: u32,
    pub compliance_rate: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl ComplianceSummary {
    pub fn compliance_percentage(&self) -> f64 {
        if self.total_batches == 0 {
            100.0
        } else {
            (self.compliant_batches as f64 / self.total_batches as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViolationItem {
    pub batch_id: String,
    pub rule_name: String,
    pub severity: String,
    pub description: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct EvidenceChainSummary {
    pub batch_id: String,
    pub chain_length: u32,
    pub is_complete: bool,
    pub last_signature: Option<String>,
}
