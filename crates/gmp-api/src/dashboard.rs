use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub trust_score: f64,
    pub compliance_rate: f64,
    pub active_batches: u32,
    pub pending_signatures: u32,
    pub open_deviations: u32,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStatus {
    pub batch_id: String,
    pub status: BatchLifecycleState,
    pub progress_percent: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BatchLifecycleState {
    Created,
    InProgress,
    PendingQA,
    Released,
    Archived,
    OnHold,
    Rejected,
}

impl BatchLifecycleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::InProgress => "in_progress",
            Self::PendingQA => "pending_qa",
            Self::Released => "released",
            Self::Archived => "archived",
            Self::OnHold => "on_hold",
            Self::Rejected => "rejected",
        }
    }
}

pub struct DashboardService;

impl DashboardService {
    pub fn get_summary() -> DashboardSummary {
        DashboardSummary {
            trust_score: 0.0,
            compliance_rate: 0.0,
            active_batches: 0,
            pending_signatures: 0,
            open_deviations: 0,
            alerts: Vec::new(),
        }
    }

    pub fn get_batch_status_list(
        page: u32,
        page_size: u32,
    ) -> super::PaginatedResponse<BatchStatus> {
        super::PaginatedResponse::new(Vec::new(), 0, &super::PaginationParams { page, page_size })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_summary_fields() {
        let summary = DashboardSummary {
            trust_score: 95.5,
            compliance_rate: 98.2,
            active_batches: 42,
            pending_signatures: 3,
            open_deviations: 1,
            alerts: Vec::new(),
        };
        assert_eq!(summary.active_batches, 42);
        assert_eq!(summary.pending_signatures, 3);
        assert_eq!(summary.open_deviations, 1);
        assert!((summary.compliance_rate - 98.2).abs() < 0.01);
    }

    #[test]
    fn test_alert_severity_as_str() {
        assert_eq!(AlertSeverity::Critical.as_str(), "critical");
        assert_eq!(AlertSeverity::Warning.as_str(), "warning");
        assert_eq!(AlertSeverity::Info.as_str(), "info");
    }

    #[test]
    fn test_batch_lifecycle_state_as_str() {
        assert_eq!(BatchLifecycleState::Created.as_str(), "created");
        assert_eq!(BatchLifecycleState::InProgress.as_str(), "in_progress");
        assert_eq!(BatchLifecycleState::PendingQA.as_str(), "pending_qa");
        assert_eq!(BatchLifecycleState::Released.as_str(), "released");
        assert_eq!(BatchLifecycleState::Archived.as_str(), "archived");
        assert_eq!(BatchLifecycleState::OnHold.as_str(), "on_hold");
        assert_eq!(BatchLifecycleState::Rejected.as_str(), "rejected");
    }

    #[test]
    fn test_batch_status() {
        let status = BatchStatus {
            batch_id: "BATCH-001".to_string(),
            status: BatchLifecycleState::InProgress,
            progress_percent: 75.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(status.batch_id, "BATCH-001");
        assert!((status.progress_percent - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_dashboard_service_get_summary() {
        let summary = DashboardService::get_summary();
        // smoke test - fields should be initialized
        assert!(summary.trust_score >= 0.0);
        assert!(summary.compliance_rate >= 0.0);
        assert!(summary.active_batches >= 0);
    }

    #[test]
    fn test_dashboard_service_get_batch_status_list() {
        let list = DashboardService::get_batch_status_list(1, 10);
        assert!(list.items.is_empty());
        assert_eq!(list.total, 0);
        assert_eq!(list.page, 1);
        assert_eq!(list.page_size, 10);
    }

    #[test]
    fn test_paginated_response_new() {
        let params = super::super::PaginationParams {
            page: 2,
            page_size: 20,
        };
        let resp = super::super::PaginatedResponse::new(
            vec!["a".to_string(), "b".to_string()],
            50,
            &params,
        );
        assert_eq!(resp.items.len(), 2);
        assert_eq!(resp.total, 50);
        assert_eq!(resp.page, 2);
        assert_eq!(resp.page_size, 20);
        assert_eq!(resp.total_pages, 3);
    }

    #[test]
    fn test_pagination_params_default() {
        let params = super::super::PaginationParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }

    #[test]
    fn test_api_response_success() {
        let resp = super::super::ApiResponse::<i32>::success(42);
        assert!(resp.success);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let resp: super::super::ApiResponse<i32> = super::super::ApiResponse::error("test error");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
    }
}
