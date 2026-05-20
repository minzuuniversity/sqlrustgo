use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub id: String,
    pub product_name: String,
    pub batch_number: String,
    pub status: BatchState,
    pub steps: Vec<BatchStep>,
    pub deviations: Vec<Deviation>,
    pub signatures: Vec<BatchSignature>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BatchState {
    Created,
    InProgress,
    PendingQA,
    Released,
    Archived,
    OnHold,
    Rejected,
}

impl BatchState {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: StepStatus,
    pub required_signatures: u32,
    pub completed_signatures: u32,
    pub parameters: Vec<StepParameter>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Locked,
    Unlocked,
    InProgress,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepParameter {
    pub name: String,
    pub value: ParameterValue,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deviation {
    pub id: String,
    pub description: String,
    pub severity: DeviationSeverity,
    pub status: DeviationStatus,
    pub reported_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub capa_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeviationSeverity {
    Critical,
    Major,
    Minor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeviationStatus {
    Open,
    Investigation,
    CAPAInProgress,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSignature {
    pub id: String,
    pub step_id: Option<String>,
    pub signer_id: String,
    pub signer_name: String,
    pub signature_type: SignatureType,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SignatureType {
    Operator,
    QA,
    Manager,
    Witness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBatchRequest {
    pub product_name: String,
    pub batch_number: String,
    pub steps: Vec<CreateStepRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStepRequest {
    pub name: String,
    pub description: String,
    pub required_signatures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRequest {
    pub operator_id: String,
    pub operator_name: String,
    pub signature_type: SignatureType,
    pub reason: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseBatchRequest {
    pub qa_id: String,
    pub qa_name: String,
    pub reason: String,
}

pub struct BatchService;

impl BatchService {
    pub fn create_batch(request: CreateBatchRequest) -> Batch {
        let now = Utc::now();
        let steps = request
            .steps
            .into_iter()
            .map(|s| BatchStep {
                id: Uuid::new_v4().to_string(),
                name: s.name,
                description: s.description,
                status: StepStatus::Locked,
                required_signatures: s.required_signatures,
                completed_signatures: 0,
                parameters: Vec::new(),
                started_at: None,
                completed_at: None,
            })
            .collect();

        Batch {
            id: Uuid::new_v4().to_string(),
            product_name: request.product_name,
            batch_number: request.batch_number,
            status: BatchState::Created,
            steps,
            deviations: Vec::new(),
            signatures: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn start_batch(batch: &mut Batch) -> Result<(), BatchError> {
        if batch.status != BatchState::Created {
            return Err(BatchError::InvalidStateTransition);
        }
        if batch.steps.is_empty() {
            return Err(BatchError::NoSteps);
        }
        batch.status = BatchState::InProgress;
        batch.steps[0].status = StepStatus::Unlocked;
        batch.updated_at = Utc::now();
        Ok(())
    }

    pub fn unlock_step(batch: &mut Batch, step_id: &str) -> Result<(), BatchError> {
        let step = batch
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or(BatchError::StepNotFound)?;

        if step.status != StepStatus::Locked {
            return Err(BatchError::InvalidStepState);
        }

        step.status = StepStatus::Unlocked;
        batch.updated_at = Utc::now();
        Ok(())
    }

    pub fn start_step(batch: &mut Batch, step_id: &str) -> Result<(), BatchError> {
        let step = batch
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or(BatchError::StepNotFound)?;

        if step.status != StepStatus::Unlocked {
            return Err(BatchError::InvalidStepState);
        }

        step.status = StepStatus::InProgress;
        step.started_at = Some(Utc::now());
        batch.updated_at = Utc::now();
        Ok(())
    }

    pub fn complete_step(
        batch: &mut Batch,
        step_id: &str,
        signature: SignatureRequest,
    ) -> Result<(), BatchError> {
        let step = batch
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or(BatchError::StepNotFound)?;

        if step.status != StepStatus::InProgress {
            return Err(BatchError::InvalidStepState);
        }

        step.status = StepStatus::Completed;
        step.completed_signatures = 1;
        step.completed_at = Some(Utc::now());

        batch.signatures.push(BatchSignature {
            id: Uuid::new_v4().to_string(),
            step_id: Some(step_id.to_string()),
            signer_id: signature.operator_id,
            signer_name: signature.operator_name,
            signature_type: signature.signature_type,
            reason: signature.reason,
            timestamp: Utc::now(),
        });

        if let Some(next_step) = batch.steps.iter_mut().find(|s| s.status == StepStatus::Locked) {
            next_step.status = StepStatus::Unlocked;
        }

        batch.updated_at = Utc::now();
        Ok(())
    }

    pub fn submit_for_qa(batch: &mut Batch) -> Result<(), BatchError> {
        if batch.status != BatchState::InProgress {
            return Err(BatchError::InvalidStateTransition);
        }

        let all_completed = batch.steps.iter().all(|s| {
            s.status == StepStatus::Completed || s.status == StepStatus::Skipped
        });
        if !all_completed {
            return Err(BatchError::IncompleteSteps);
        }

        batch.status = BatchState::PendingQA;
        batch.updated_at = Utc::now();
        Ok(())
    }

    pub fn release_batch(
        batch: &mut Batch,
        request: ReleaseBatchRequest,
    ) -> Result<(), BatchError> {
        if batch.status != BatchState::PendingQA {
            return Err(BatchError::InvalidStateTransition);
        }

        batch.status = BatchState::Released;
        batch.signatures.push(BatchSignature {
            id: Uuid::new_v4().to_string(),
            step_id: None,
            signer_id: request.qa_id,
            signer_name: request.qa_name,
            signature_type: SignatureType::QA,
            reason: request.reason,
            timestamp: Utc::now(),
        });
        batch.updated_at = Utc::now();
        Ok(())
    }

    pub fn report_deviation(
        batch: &mut Batch,
        description: String,
        severity: DeviationSeverity,
    ) -> Result<Deviation, BatchError> {
        if batch.status == BatchState::Released || batch.status == BatchState::Archived {
            return Err(BatchError::InvalidStateTransition);
        }

        let deviation = Deviation {
            id: Uuid::new_v4().to_string(),
            description,
            severity,
            status: DeviationStatus::Open,
            reported_at: Utc::now(),
            resolved_at: None,
            capa_id: None,
        };

        batch.deviations.push(deviation.clone());
        batch.updated_at = Utc::now();
        Ok(deviation)
    }

    pub fn get_progress(batch: &Batch) -> f64 {
        if batch.steps.is_empty() {
            return 0.0;
        }
        let completed = batch
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed || s.status == StepStatus::Skipped)
            .count();
        (completed as f64 / batch.steps.len() as f64) * 100.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BatchError {
    #[error("Batch not found")]
    NotFound,
    #[error("Step not found")]
    StepNotFound,
    #[error("Invalid state transition")]
    InvalidStateTransition,
    #[error("Invalid step state")]
    InvalidStepState,
    #[error("No steps defined")]
    NoSteps,
    #[error("Not all steps completed")]
    IncompleteSteps,
    #[error("Invalid signature")]
    InvalidSignature,
}
