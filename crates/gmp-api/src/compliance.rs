//! Compliance Judge API
//!
//! HTTP endpoints for LLM-powered compliance judgment using Ollama.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlrustgo_compliance_engine::judge::{
    AuditLogEntry, ComplianceJudge as EngineJudge, JudgmentRequest as EngineRequest,
    JudgmentResult as EngineResult, RuleParser,
};
use std::sync::Arc;

/// Application state for compliance judge service
pub struct ComplianceServiceState {
    pub judge: EngineJudge,
    pub rule_parser: RuleParser,
    /// Audit logs storage
    pub audit_logs: Arc<std::sync::Mutex<Vec<AuditLogEntry>>>,
}

/// HTTP request for compliance judgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeRequest {
    /// Unique operation identifier
    pub operation_id: String,
    /// Operation description to evaluate
    pub operation_description: String,
    /// Applicable rule IDs to evaluate against
    pub rule_ids: Vec<String>,
    /// Language code (e.g., "zh", "en", "ja")
    pub language: Option<String>,
}

/// HTTP response for compliance judgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResponse {
    /// Operation ID from request
    pub operation_id: String,
    /// Judgment status: COMPLIANT, NON_COMPLIANT, NEEDS_REVIEW, TIMEOUT, ERROR
    pub status: String,
    /// Confidence score (0.0-1.0)
    pub confidence_score: f32,
    /// Human-readable explanation of judgment
    pub explanation: String,
    /// Relevant rule sections that influenced decision
    pub evidence: Vec<EvidenceItem>,
    /// Time taken for judgment in milliseconds
    pub latency_ms: u64,
    /// LLM model used
    pub model: Option<String>,
    /// Timestamp of judgment
    pub timestamp: DateTime<Utc>,
}

/// Evidence citation from a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Rule ID
    pub rule_id: String,
    /// Rule text that was applied
    pub rule_text: String,
    /// Severity level if violated
    pub severity: Option<String>,
}

/// Audit log query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogResponse {
    /// Unique entry ID
    pub entry_id: String,
    /// Unique operation ID
    pub operation_id: String,
    /// Rule ID used for judgment
    pub rule_id: String,
    /// Judgment status
    pub status: String,
    /// Confidence score
    pub confidence_score: f32,
    /// LLM model used
    pub llm_model: Option<String>,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Hash of this entry
    pub hash: String,
    /// Hash of previous entry (chain)
    pub prev_hash: String,
}

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

/// Audit query result - either success with entries or error with message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuditResult {
    Success(Vec<AuditLogResponse>),
    Error { error: String, message: String },
}

impl From<Vec<AuditLogResponse>> for AuditResult {
    fn from(entries: Vec<AuditLogResponse>) -> Self {
        AuditResult::Success(entries)
    }
}

impl From<String> for AuditResult {
    fn from(msg: String) -> Self {
        AuditResult::Error {
            error: "NOT_FOUND".to_string(),
            message: msg,
        }
    }
}

/// Convert from EngineJudgmentResult to JudgeResponse
impl From<EngineResult> for JudgeResponse {
    fn from(result: EngineResult) -> Self {
        Self {
            operation_id: result.operation_id,
            status: format!("{:?}", result.status),
            confidence_score: result.confidence_score,
            explanation: result.explanation,
            evidence: result
                .evidence
                .into_iter()
                .map(|e| EvidenceItem {
                    rule_id: e.rule_id,
                    rule_text: e.rule_text,
                    severity: e.severity,
                })
                .collect(),
            latency_ms: result.latency_ms,
            model: result.model,
            timestamp: result.timestamp,
        }
    }
}

/// Convert from AuditLogEntry to AuditLogResponse
impl From<AuditLogEntry> for AuditLogResponse {
    fn from(entry: AuditLogEntry) -> Self {
        Self {
            entry_id: entry.entry_id,
            operation_id: entry.operation_id,
            rule_id: entry.rule_id,
            status: entry.status,
            confidence_score: entry.confidence_score,
            llm_model: entry.llm_model,
            latency_ms: entry.latency_ms,
            timestamp: entry.timestamp,
            hash: entry.hash,
            prev_hash: entry.prev_hash,
        }
    }
}

/// POST /api/v1/compliance/judge - Make a compliance judgment
pub async fn judge_handler(
    State(state): State<Arc<ComplianceServiceState>>,
    Json(request): Json<JudgeRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let engine_request = EngineRequest {
        operation_id: request.operation_id.clone(),
        operation_description: request.operation_description.clone(),
        rule_ids: request.rule_ids.clone(),
        language: request.language,
        streaming: false,
    };

    let result: EngineResult = state.judge.judge(&engine_request).await;

    // Record to audit log
    let audit_entry = AuditLogEntry {
        entry_id: uuid::Uuid::new_v4().to_string(),
        operation_id: request.operation_id,
        rule_id: request
            .rule_ids
            .first()
            .cloned()
            .unwrap_or_default(),
        status: format!("{:?}", result.status),
        confidence_score: result.confidence_score,
        llm_model: result.model.clone(),
        latency_ms: result.latency_ms,
        timestamp: result.timestamp,
        hash: String::new(),
        prev_hash: String::new(),
    };

    // Calculate hash for audit entry
    let prev_hash = {
        let logs = state.audit_logs.lock().unwrap();
        logs.last()
            .map(|e| e.hash.clone())
            .unwrap_or_default()
    };

    let mut entry_with_hash = audit_entry;
    entry_with_hash.prev_hash = prev_hash;
    entry_with_hash.hash = entry_with_hash.calculate_hash();

    {
        let mut logs = state.audit_logs.lock().unwrap();
        logs.push(entry_with_hash.clone());
    }

    let response: JudgeResponse = result.into();
    Ok(Json(crate::ApiResponse::success(response)))
}

/// GET /api/v1/compliance/audit/{operation_id} - Get audit log entry by operation ID
pub async fn audit_handler(
    State(state): State<Arc<ComplianceServiceState>>,
    Path(operation_id): Path<String>,
) -> impl IntoResponse {
    let logs = state.audit_logs.lock().unwrap();
    let entries: Vec<AuditLogResponse> = logs
        .iter()
        .filter(|e| e.operation_id == operation_id)
        .cloned()
        .map(|e| e.into())
        .collect();

    let result: AuditResult = if entries.is_empty() {
        AuditResult::Error {
            error: "NOT_FOUND".to_string(),
            message: format!("No audit log found for operation '{}'", operation_id),
        }
    } else {
        AuditResult::Success(entries)
    };

    (StatusCode::OK, Json(crate::ApiResponse::success(result)))
}

/// Create compliance judge service state
pub fn create_service_state(judge: EngineJudge) -> ComplianceServiceState {
    ComplianceServiceState {
        judge,
        rule_parser: RuleParser::new(),
        audit_logs: Arc::new(std::sync::Mutex::new(Vec::new())),
    }
}

/// Create the compliance API router
pub fn create_router(state: Arc<ComplianceServiceState>) -> Router {
    Router::new()
        .route("/api/v1/compliance/judge", post(judge_handler))
        .route("/api/v1/compliance/audit/{operation_id}", get(audit_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_judge_request_serialization() {
        let request = JudgeRequest {
            operation_id: "op-001".to_string(),
            operation_description: "Store vaccine at 5°C".to_string(),
            rule_ids: vec!["GMP-001".to_string()],
            language: Some("zh".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("op-001"));
        assert!(json.contains("Store vaccine"));
    }

    #[test]
    fn test_judge_response_deserialization() {
        let response = JudgeResponse {
            operation_id: "op-001".to_string(),
            status: "Compliant".to_string(),
            confidence_score: 0.95,
            explanation: "All rules satisfied".to_string(),
            evidence: vec![],
            latency_ms: 150,
            model: Some("qwen3:8b-gmp".to_string()),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Compliant"));
        assert!(json.contains("0.95"));
    }

    #[test]
    fn test_evidence_item_serialization() {
        let evidence = EvidenceItem {
            rule_id: "GMP-001".to_string(),
            rule_text: "Temperature must be 2-8°C".to_string(),
            severity: Some("critical".to_string()),
        };

        let json = serde_json::to_string(&evidence).unwrap();
        assert!(json.contains("GMP-001"));
        assert!(json.contains("Temperature must be 2-8°C"));
    }

    #[test]
    fn test_audit_log_response_serialization() {
        let entry = AuditLogResponse {
            entry_id: "entry-001".to_string(),
            operation_id: "op-001".to_string(),
            rule_id: "GMP-001".to_string(),
            status: "Compliant".to_string(),
            confidence_score: 0.95,
            llm_model: Some("qwen3:8b".to_string()),
            latency_ms: 150,
            timestamp: Utc::now(),
            hash: "abc123".to_string(),
            prev_hash: "genesis".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("op-001"));
        assert!(json.contains("Compliant"));
    }

    #[test]
    fn test_api_error_response() {
        let error = ApiError {
            error: "NOT_FOUND".to_string(),
            message: "Operation not found".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("Operation not found"));
    }

    #[test]
    fn test_audit_result_success_serialization() {
        let entries = vec![
            AuditLogResponse {
                entry_id: "entry-001".to_string(),
                operation_id: "op-001".to_string(),
                rule_id: "GMP-001".to_string(),
                status: "Compliant".to_string(),
                confidence_score: 0.95,
                llm_model: Some("qwen3:8b".to_string()),
                latency_ms: 150,
                timestamp: Utc::now(),
                hash: "abc123".to_string(),
                prev_hash: "genesis".to_string(),
            },
            AuditLogResponse {
                entry_id: "entry-002".to_string(),
                operation_id: "op-001".to_string(),
                rule_id: "GMP-002".to_string(),
                status: "NonCompliant".to_string(),
                confidence_score: 0.85,
                llm_model: Some("qwen3:8b".to_string()),
                latency_ms: 120,
                timestamp: Utc::now(),
                hash: "def456".to_string(),
                prev_hash: "abc123".to_string(),
            },
        ];

        let result = AuditResult::Success(entries);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("entry-001"));
        assert!(json.contains("entry-002"));
        assert!(json.contains("Compliant"));
        assert!(json.contains("NonCompliant"));
    }

    #[test]
    fn test_audit_result_error_serialization() {
        let result = AuditResult::Error {
            error: "NOT_FOUND".to_string(),
            message: "No audit log found for operation 'op-999'".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("No audit log found"));
    }

    #[tokio::test]
    async fn test_judge_request_to_engine_request_conversion() {
        let request = JudgeRequest {
            operation_id: "op-test-001".to_string(),
            operation_description: "Store vaccine at 5°C".to_string(),
            rule_ids: vec!["GMP-001".to_string(), "GMP-002".to_string()],
            language: Some("zh".to_string()),
        };

        let engine_request = EngineRequest {
            operation_id: request.operation_id.clone(),
            operation_description: request.operation_description.clone(),
            rule_ids: request.rule_ids.clone(),
            language: request.language,
            streaming: false,
        };

        assert_eq!(engine_request.operation_id, "op-test-001");
        assert_eq!(engine_request.operation_description, "Store vaccine at 5°C");
        assert_eq!(engine_request.rule_ids.len(), 2);
        assert_eq!(engine_request.language, Some("zh".to_string()));
        assert!(!engine_request.streaming);
    }

    #[tokio::test]
    async fn test_judge_response_from_engine_result() {
        use sqlrustgo_compliance_engine::judge::JudgmentStatus;

        let engine_result = EngineResult {
            operation_id: "op-001".to_string(),
            status: JudgmentStatus::Compliant,
            confidence_score: 0.92,
            explanation: "All GMP rules satisfied".to_string(),
            evidence: vec![sqlrustgo_compliance_engine::RuleEvidence {
                rule_id: "GMP-001".to_string(),
                rule_text: "Temperature must be 2-8°C".to_string(),
                severity: None,
            }],
            latency_ms: 150,
            model: Some("qwen3:8b-gmp".to_string()),
            timestamp: Utc::now(),
        };

        let response: JudgeResponse = engine_result.into();

        assert_eq!(response.operation_id, "op-001");
        assert!(response.explanation.contains("All GMP rules satisfied"));
        assert_eq!(response.evidence.len(), 1);
        assert_eq!(response.evidence[0].rule_id, "GMP-001");
    }

    #[test]
    fn test_create_service_state_initializes_correctly() {
        use sqlrustgo_compliance_engine::ComplianceJudge as EngineJudge;

        // Create a judge with a mock client
        let judge = EngineJudge::new(
            Box::new(crate::ollama_client::OllamaClientImpl::new("http://localhost:11434")),
            "test-model",
        );
        let state = create_service_state(judge);

        // Verify audit logs are initialized as empty
        assert!(state.audit_logs.lock().unwrap().is_empty());
    }
}
