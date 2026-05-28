//! Evidence Trace Module
//!
//! Provides the EvidenceTrace struct for tracking verification traces.

use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{Evidence, Verdict};

/// A complete evidence trace record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceTrace {
    /// Unique trace identifier
    pub trace_id: String,
    /// Evidence steps in the trace
    pub steps: Vec<Evidence>,
    /// Final verdict of the trace
    pub verdict: Verdict,
    /// Timestamp when trace was created
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
}

impl EvidenceTrace {
    /// Creates a new evidence trace with a generated trace_id
    pub fn new(steps: Vec<Evidence>, verdict: Verdict) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            steps,
            verdict,
            created_at: Utc::now(),
        }
    }

    /// Creates a new evidence trace with a custom trace_id
    pub fn with_id(trace_id: String, steps: Vec<Evidence>, verdict: Verdict) -> Self {
        Self {
            trace_id,
            steps,
            verdict,
            created_at: Utc::now(),
        }
    }

    /// Returns the number of steps in the trace
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns true if the trace has no steps
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Adds a step to the trace
    pub fn add_step(&mut self, evidence: Evidence) {
        self.steps.push(evidence);
    }

    /// Returns true if the trace is verified (Proven verdict)
    pub fn is_verified(&self) -> bool {
        self.verdict == Verdict::Proven
    }
}

impl std::fmt::Display for EvidenceTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EvidenceTrace(id={}, steps={}, verdict={})",
            self.trace_id,
            self.steps.len(),
            self.verdict
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_trace_new() {
        let sql = Evidence::SQLQuery {
            query: "SELECT 1".to_string(),
            params: None,
        };
        let trace = EvidenceTrace::new(vec![sql], Verdict::Proven);
        assert!(!trace.trace_id.is_empty());
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.verdict, Verdict::Proven);
    }

    #[test]
    fn test_evidence_trace_with_id() {
        let trace = EvidenceTrace::with_id(
            "test-trace-001".to_string(),
            vec![],
            Verdict::Partial,
        );
        assert_eq!(trace.trace_id, "test-trace-001");
    }

    #[test]
    fn test_evidence_trace_len() {
        let trace = EvidenceTrace::new(vec![], Verdict::Unverifiable);
        assert_eq!(trace.len(), 0);
        assert!(trace.is_empty());
    }

    #[test]
    fn test_evidence_trace_add_step() {
        let mut trace = EvidenceTrace::new(vec![], Verdict::Unverifiable);
        let sql = Evidence::SQLQuery {
            query: "SELECT 1".to_string(),
            params: None,
        };
        trace.add_step(sql);
        assert_eq!(trace.len(), 1);
        assert!(!trace.is_empty());
    }

    #[test]
    fn test_evidence_trace_is_verified() {
        let trace_proven = EvidenceTrace::new(vec![], Verdict::Proven);
        assert!(trace_proven.is_verified());

        let trace_partial = EvidenceTrace::new(vec![], Verdict::Partial);
        assert!(!trace_partial.is_verified());
    }

    #[test]
    fn test_evidence_trace_display() {
        let trace = EvidenceTrace::with_id(
            "display-test".to_string(),
            vec![],
            Verdict::Contradicted,
        );
        let display = format!("{}", trace);
        assert!(display.contains("display-test"));
        assert!(display.contains("Contradicted"));
    }
}
