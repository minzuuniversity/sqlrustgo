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
