//! Workflow instance state

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    Pending,
    Running,
    Waiting,
    Completed,
    Rejected,
    Cancelled,
    TimedOut,
}

impl InstanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    NotStarted,
    InProgress,
    Approved,
    Rejected,
    Skipped,
    TimedOut,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Skipped => "skipped",
            Self::TimedOut => "timed_out",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepInstance {
    pub step_id: String,
    pub status: StepStatus,
    pub assignee: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub evidence_ids: Vec<String>,
}

impl StepInstance {
    pub fn new(step_id: &str) -> Self {
        Self {
            step_id: step_id.to_string(),
            status: StepStatus::NotStarted,
            assignee: None,
            started_at: None,
            completed_at: None,
            evidence_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowInstance {
    pub id: String,
    pub workflow_name: String,
    pub version: String,
    pub status: InstanceStatus,
    pub current_step: Option<String>,
    pub steps: Vec<StepInstance>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl WorkflowInstance {
    pub fn new(id: &str, workflow_name: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            workflow_name: workflow_name.to_string(),
            version: version.to_string(),
            status: InstanceStatus::Pending,
            current_step: None,
            steps: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    pub fn advance_to_step(&mut self, step_id: &str) {
        self.current_step = Some(step_id.to_string());
        self.status = InstanceStatus::Running;
        self.updated_at = chrono::Utc::now();
    }
}
