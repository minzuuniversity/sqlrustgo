//! GMP Integration Module
//!
//! Bridges LLM-based ComplianceJudge with existing GMP declarative compliance system.
//! Provides event hooks for automatic compliance checking on GMP operations.

use sqlrustgo_compliance_engine::judge::{
    ComplianceJudge as LlvmComplianceJudge, JudgmentRequest as LlvmJudgmentRequest,
    JudgmentResult as LlvmJudgmentResult, JudgmentStatus as LlvmJudgmentStatus, RuleParser,
};
use sqlrustgo_gmp::compliance::{DeclarativeRule, Violation as GmpViolation};
use sqlrustgo_gmp::workflow::ApprovalAction;
use std::sync::{Arc, Mutex};

/// Hybrid compliance result that combines LLM and declarative checking
#[derive(Debug, Clone)]
pub struct HybridComplianceResult {
    /// LLM-based judgment result
    pub llm_result: Option<LlvmJudgmentResult>,
    /// Declarative rule violations
    pub declarative_violations: Vec<GmpViolation>,
    /// Combined status
    pub is_compliant: bool,
    /// Higher confidence score between both methods
    pub confidence_score: f32,
    /// All violations combined
    pub all_violations: Vec<ViolationInfo>,
}

/// Unified violation information
#[derive(Debug, Clone)]
pub struct ViolationInfo {
    /// Source of violation: "llm" or "declarative"
    pub source: String,
    /// Rule ID or name
    pub rule_id: String,
    /// Violation description
    pub description: String,
    /// Severity level
    pub severity: String,
    /// Remediation suggestion
    pub remediation: Option<String>,
}

impl HybridComplianceResult {
    /// Create from LLM judgment result
    pub fn from_llm_result(result: LlvmJudgmentResult) -> Self {
        let violations: Vec<ViolationInfo> = result
            .evidence
            .iter()
            .map(|e| ViolationInfo {
                source: "llm".to_string(),
                rule_id: e.rule_id.clone(),
                description: e.rule_text.clone(),
                severity: e.severity.clone().unwrap_or_else(|| "unknown".to_string()),
                remediation: None,
            })
            .collect();

        let is_compliant = matches!(result.status, LlvmJudgmentStatus::Compliant);

        Self {
            llm_result: Some(result),
            declarative_violations: vec![],
            is_compliant,
            confidence_score: 0.0,
            all_violations: violations,
        }
    }

    /// Create from declarative violations
    pub fn from_declarative_violations(violations: Vec<GmpViolation>) -> Self {
        let violation_infos: Vec<ViolationInfo> = violations
            .iter()
            .map(|v| ViolationInfo {
                source: "declarative".to_string(),
                rule_id: v.rule.clone(),
                description: v.description.clone(),
                severity: v.severity.clone(),
                remediation: Some(v.remediation.clone()),
            })
            .collect();

        let is_compliant = violations.is_empty();
        let confidence_score = if is_compliant { 1.0 } else { 0.9 };

        Self {
            llm_result: None,
            declarative_violations: violations,
            is_compliant,
            confidence_score,
            all_violations: violation_infos,
        }
    }

    /// Combine two results
    pub fn combine(self, other: HybridComplianceResult) -> Self {
        // Merge violations
        let mut all_violations = self.all_violations;
        all_violations.extend(other.all_violations);

        // Use higher confidence score
        let confidence_score = self.confidence_score.max(other.confidence_score);

        // Compliant only if both are compliant
        let is_compliant = self.is_compliant && other.is_compliant;

        Self {
            llm_result: self.llm_result.or(other.llm_result),
            declarative_violations: self.declarative_violations,
            is_compliant,
            confidence_score,
            all_violations,
        }
    }
}

/// GMP event types that can trigger compliance checking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GmpEvent {
    /// Document created
    DocumentCreated(i64),
    /// Document status changed
    DocumentStatusChanged(i64, String),
    /// Document approval requested
    DocumentApprovalRequested(i64),
    /// Workflow approval action
    WorkflowApproval(ApprovalAction),
    /// Signature collected
    SignatureCollected(i64),
}

impl GmpEvent {
    /// Get document ID if this event is document-related
    pub fn doc_id(&self) -> Option<i64> {
        match self {
            GmpEvent::DocumentCreated(id) => Some(*id),
            GmpEvent::DocumentStatusChanged(id, _) => Some(*id),
            GmpEvent::DocumentApprovalRequested(id) => Some(*id),
            GmpEvent::SignatureCollected(id) => Some(*id),
            GmpEvent::WorkflowApproval(_) => None,
        }
    }
}

/// Event hook for GMP compliance checking
pub struct ComplianceEventHook {
    judge: Option<Arc<dyn LlvmComplianceJudgeTrait>>,
    rule_parser: RuleParser,
}

/// Trait object for LLM judge to allow sync usage
pub trait LlvmComplianceJudgeTrait: Send + Sync {
    fn judge_sync(&self, request: &LlvmJudgmentRequest) -> LlvmJudgmentResult;
}

/// Wrapper that provides sync interface
pub struct SyncLlvmComplianceJudge {
    inner: LlvmComplianceJudge,
}

impl SyncLlvmComplianceJudge {
    pub fn new(judge: LlvmComplianceJudge) -> Self {
        Self { inner: judge }
    }
}

impl LlvmComplianceJudgeTrait for SyncLlvmComplianceJudge {
    fn judge_sync(&self, request: &LlvmJudgmentRequest) -> LlvmJudgmentResult {
        // Use block_on to execute the async judge method synchronously
        // In production, this should be called from an async context
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(self.inner.judge(request))
    }
}

impl ComplianceEventHook {
    /// Create a new hook with LLM judge
    pub fn new(judge: LlvmComplianceJudge) -> Self {
        Self {
            judge: Some(Arc::new(SyncLlvmComplianceJudge::new(judge))),
            rule_parser: RuleParser::new(),
        }
    }

    /// Create a hook with a trait object judge (for testing)
    pub fn new_with_trait(judge: Arc<dyn LlvmComplianceJudgeTrait>) -> Self {
        Self {
            judge: Some(judge),
            rule_parser: RuleParser::new(),
        }
    }

    /// Create a hook without LLM judge (declarative only)
    pub fn without_llm() -> Self {
        Self {
            judge: None,
            rule_parser: RuleParser::new(),
        }
    }

    /// Handle a GMP event and perform compliance checking
    pub fn on_event(&self, event: &GmpEvent) -> HybridComplianceResult {
        match event {
            GmpEvent::DocumentCreated(doc_id) => self.check_document_compliance(*doc_id),
            GmpEvent::DocumentStatusChanged(doc_id, _) => self.check_document_compliance(*doc_id),
            GmpEvent::DocumentApprovalRequested(doc_id) => self.check_document_compliance(*doc_id),
            GmpEvent::WorkflowApproval(action) => self.check_workflow_compliance(action),
            GmpEvent::SignatureCollected(doc_id) => self.check_document_compliance(*doc_id),
        }
    }

    /// Check document compliance
    fn check_document_compliance(&self, doc_id: i64) -> HybridComplianceResult {
        // If LLM judge is available, run LLM check
        if let Some(ref judge) = self.judge {
            let request = LlvmJudgmentRequest {
                operation_id: format!("doc-{}", doc_id),
                operation_description: format!("Document {} compliance check", doc_id),
                rule_ids: vec!["GMP-001".to_string()],
                language: Some("en".to_string()),
                streaming: false,
            };

            let result = judge.judge_sync(&request);
            HybridComplianceResult::from_llm_result(result)
        } else {
            HybridComplianceResult::from_declarative_violations(vec![])
        }
    }

    /// Check workflow compliance
    fn check_workflow_compliance(&self, action: &ApprovalAction) -> HybridComplianceResult {
        let description = format!("Workflow approval action: {:?}", action);
        let rule_ids = vec!["GMP-APPROVAL-001".to_string()];

        if let Some(ref judge) = self.judge {
            let request = LlvmJudgmentRequest {
                operation_id: format!("workflow-{:?}", action),
                operation_description: description,
                rule_ids,
                language: Some("en".to_string()),
                streaming: false,
            };

            let result = judge.judge_sync(&request);
            HybridComplianceResult::from_llm_result(result)
        } else {
            HybridComplianceResult::from_declarative_violations(vec![])
        }
    }
}

/// Storage for compliance event history
pub struct ComplianceEventStore {
    events: Arc<Mutex<Vec<ComplianceEvent>>>,
}

#[derive(Debug, Clone)]
struct ComplianceEvent {
    event: GmpEvent,
    result: HybridComplianceResult,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl ComplianceEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record(&self, event: GmpEvent, result: HybridComplianceResult) {
        let mut events = self.events.lock().unwrap();
        events.push(ComplianceEvent {
            event,
            result,
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn get_events(&self) -> Vec<(GmpEvent, HybridComplianceResult)> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .map(|e| (e.event.clone(), e.result.clone()))
            .collect()
    }

    pub fn get_events_for_document(&self, doc_id: i64) -> Vec<(GmpEvent, HybridComplianceResult)> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .filter(|e| e.event.doc_id() == Some(doc_id))
            .map(|e| (e.event.clone(), e.result.clone()))
            .collect()
    }
}

impl Default for ComplianceEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_result_from_llm() {
        use sqlrustgo_compliance_engine::judge::JudgmentStatus;

        let result = LlvmJudgmentResult {
            operation_id: "op-1".to_string(),
            status: JudgmentStatus::Compliant,
            confidence_score: 0.95,
            explanation: "All rules satisfied".to_string(),
            evidence: vec![],
            latency_ms: 100,
            model: Some("test-model".to_string()),
            timestamp: chrono::Utc::now(),
        };

        let hybrid = HybridComplianceResult::from_llm_result(result);
        assert!(hybrid.is_compliant);
        assert!(hybrid.llm_result.is_some());
    }

    #[test]
    fn test_hybrid_result_from_declarative() {
        let violations = vec![GmpViolation {
            doc_id: 1,
            rule: "TestRule".to_string(),
            severity: "High".to_string(),
            description: "Test violation".to_string(),
            detected_at: 0,
            remediation: "Fix it".to_string(),
        }];

        let hybrid = HybridComplianceResult::from_declarative_violations(violations);
        assert!(!hybrid.is_compliant);
        assert_eq!(hybrid.declarative_violations.len(), 1);
    }

    #[test]
    fn test_combine_hybrid_results() {
        use sqlrustgo_compliance_engine::judge::JudgmentStatus;

        let llm_result = LlvmJudgmentResult {
            operation_id: "op-1".to_string(),
            status: JudgmentStatus::Compliant,
            confidence_score: 0.9,
            explanation: "OK".to_string(),
            evidence: vec![],
            latency_ms: 100,
            model: Some("test".to_string()),
            timestamp: chrono::Utc::now(),
        };

        let result1 = HybridComplianceResult::from_llm_result(llm_result);

        let violations = vec![GmpViolation {
            doc_id: 1,
            rule: "Rule1".to_string(),
            severity: "Medium".to_string(),
            description: "Violation".to_string(),
            detected_at: 0,
            remediation: "Fix".to_string(),
        }];
        let result2 = HybridComplianceResult::from_declarative_violations(violations);

        let combined = result1.combine(result2);
        assert!(!combined.is_compliant); // One is non-compliant
        assert_eq!(combined.all_violations.len(), 1);
    }

    #[test]
    fn test_compliance_event_store() {
        let store = ComplianceEventStore::new();
        let event = GmpEvent::DocumentCreated(123);
        let result = HybridComplianceResult::from_declarative_violations(vec![]);

        store.record(event.clone(), result);

        let events = store.get_events();
        assert_eq!(events.len(), 1);

        let doc_events = store.get_events_for_document(123);
        assert_eq!(doc_events.len(), 1);

        let other_events = store.get_events_for_document(999);
        assert_eq!(other_events.len(), 0);
    }

    #[test]
    fn test_gmp_event_doc_id() {
        assert_eq!(GmpEvent::DocumentCreated(1).doc_id(), Some(1));
        assert_eq!(
            GmpEvent::DocumentStatusChanged(2, "Active".to_string()).doc_id(),
            Some(2)
        );
        assert_eq!(
            GmpEvent::WorkflowApproval(ApprovalAction::Approve).doc_id(),
            None
        );
    }

    #[test]
    fn test_gmp_event_equality() {
        assert_eq!(GmpEvent::DocumentCreated(1), GmpEvent::DocumentCreated(1));
        assert_ne!(GmpEvent::DocumentCreated(1), GmpEvent::DocumentCreated(2));
    }

    /// Mock judge for testing
    struct MockJudge {
        result: LlvmJudgmentResult,
    }

    impl MockJudge {
        fn new_compliant() -> Self {
            use sqlrustgo_compliance_engine::judge::JudgmentStatus;
            Self {
                result: LlvmJudgmentResult {
                    operation_id: String::new(),
                    status: JudgmentStatus::Compliant,
                    confidence_score: 0.95,
                    explanation: "Compliant".to_string(),
                    evidence: vec![],
                    latency_ms: 10,
                    model: Some("mock".to_string()),
                    timestamp: chrono::Utc::now(),
                },
            }
        }

        fn new_non_compliant(rule_id: &str, description: &str) -> Self {
            use sqlrustgo_compliance_engine::judge::JudgmentStatus;
            Self {
                result: LlvmJudgmentResult {
                    operation_id: String::new(),
                    status: JudgmentStatus::NonCompliant,
                    confidence_score: 0.9,
                    explanation: "Non-compliant".to_string(),
                    evidence: vec![sqlrustgo_compliance_engine::judge::types::RuleEvidence {
                        rule_id: rule_id.to_string(),
                        rule_text: description.to_string(),
                        severity: Some("high".to_string()),
                    }],
                    latency_ms: 10,
                    model: Some("mock".to_string()),
                    timestamp: chrono::Utc::now(),
                },
            }
        }
    }

    impl LlvmComplianceJudgeTrait for MockJudge {
        fn judge_sync(&self, _request: &LlvmJudgmentRequest) -> LlvmJudgmentResult {
            self.result.clone()
        }
    }

    #[test]
    fn test_compliance_event_hook_with_mock_judge() {
        let mock_judge = MockJudge::new_compliant();
        let hook = ComplianceEventHook::new_with_trait(Arc::new(mock_judge));

        let event = GmpEvent::DocumentCreated(100);
        let result = hook.on_event(&event);

        assert!(result.is_compliant);
        assert!(result.llm_result.is_some());
    }

    #[test]
    fn test_compliance_event_hook_non_compliant() {
        let mock_judge = MockJudge::new_non_compliant("GMP-001", "Temperature out of range");
        let hook = ComplianceEventHook::new_with_trait(Arc::new(mock_judge));

        let event = GmpEvent::DocumentCreated(100);
        let result = hook.on_event(&event);

        assert!(!result.is_compliant);
        assert_eq!(result.all_violations.len(), 1);
        assert_eq!(result.all_violations[0].rule_id, "GMP-001");
    }

    #[test]
    fn test_compliance_event_hook_workflow_event() {
        let mock_judge = MockJudge::new_compliant();
        let hook = ComplianceEventHook::new_with_trait(Arc::new(mock_judge));

        let event = GmpEvent::WorkflowApproval(ApprovalAction::Approve);
        let result = hook.on_event(&event);

        assert!(result.is_compliant);
    }

    #[test]
    fn test_event_store_integration() {
        let mock_judge = MockJudge::new_compliant();
        let hook = ComplianceEventHook::new_with_trait(Arc::new(mock_judge));
        let store = ComplianceEventStore::new();

        // Process events
        let event1 = GmpEvent::DocumentCreated(1);
        let result1 = hook.on_event(&event1);
        store.record(event1, result1);

        let event2 = GmpEvent::DocumentStatusChanged(2, "Active".to_string());
        let result2 = hook.on_event(&event2);
        store.record(event2, result2);

        // Verify store
        let all_events = store.get_events();
        assert_eq!(all_events.len(), 2);

        let doc1_events = store.get_events_for_document(1);
        assert_eq!(doc1_events.len(), 1);

        let doc2_events = store.get_events_for_document(2);
        assert_eq!(doc2_events.len(), 1);

        let doc99_events = store.get_events_for_document(99);
        assert!(doc99_events.is_empty());
    }

    #[test]
    fn test_hybrid_result_combines_llm_and_declarative() {
        use sqlrustgo_compliance_engine::judge::JudgmentStatus;

        // LLM says compliant
        let llm_result = LlvmJudgmentResult {
            operation_id: "op-1".to_string(),
            status: JudgmentStatus::Compliant,
            confidence_score: 0.9,
            explanation: "OK".to_string(),
            evidence: vec![],
            latency_ms: 100,
            model: Some("test".to_string()),
            timestamp: chrono::Utc::now(),
        };

        // Declarative says non-compliant
        let violations = vec![GmpViolation {
            doc_id: 1,
            rule: "GMP-001".to_string(),
            severity: "High".to_string(),
            description: "Temperature violation".to_string(),
            detected_at: 0,
            remediation: "Fix it".to_string(),
        }];

        let llm_hybrid = HybridComplianceResult::from_llm_result(llm_result);
        let decl_hybrid = HybridComplianceResult::from_declarative_violations(violations);

        let combined = llm_hybrid.combine(decl_hybrid);

        // Combined should be non-compliant (both must be compliant)
        assert!(!combined.is_compliant);
        assert_eq!(combined.all_violations.len(), 1);
        assert_eq!(combined.all_violations[0].source, "declarative");
    }
}
