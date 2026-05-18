//! State transition results

pub enum TransitionResult {
    Advanced(String),       // Moved to next step with ID
    Completed,              // Workflow completed
    Rejected(String),       // Rejected at step ID
    InvalidAction(String),  // Unknown action
    Timeout(String),        // Timed out at step ID
}

impl TransitionResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Advanced(_) | Self::Completed)
    }
}
