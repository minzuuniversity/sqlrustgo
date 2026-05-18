use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub id: String,
    pub product_name: String,
    pub batch_number: String,
    pub status: BatchState,
    pub steps: Vec<BatchStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BatchState {
    Created,
    InProgress,
    PendingQA,
    Released,
    Archived,
    OnHold,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStep {
    pub id: String,
    pub name: String,
    pub status: StepStatus,
    pub required_signatures: u32,
    pub completed_signatures: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StepStatus {
    Locked,
    Unlocked,
    InProgress,
    Completed,
    Skipped,
}

pub struct BatchService;

impl BatchService {
    pub fn create_batch(product: &str, batch_number: &str) -> Batch {
        Batch {
            id: uuid::Uuid::new_v4().to_string(),
            product_name: product.to_string(),
            batch_number: batch_number.to_string(),
            status: BatchState::Created,
            steps: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn get_batch(id: &str) -> Option<Batch> {
        None
    }

    pub fn update_step(batch_id: &str, step_id: &str) -> Result<Batch, String> {
        Err("not implemented".to_string())
    }

    pub fn sign_batch(batch_id: &str, signature: &SignatureRequest) -> Result<Batch, String> {
        Err("not implemented".to_string())
    }

    pub fn release_batch(batch_id: &str) -> Result<Batch, String> {
        Err("not implemented".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRequest {
    pub operator_id: String,
    pub reason: String,
    pub signature_data: String,
}
