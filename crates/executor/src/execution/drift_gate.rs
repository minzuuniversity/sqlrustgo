//! DriftGate — enforcement layer for WAL/TX contract
//!
//! Enforces invariants BEFORE operations execute.
//! Replaces DriftDetector (observability-only) with事前阻断.

use super::transaction_context::TransactionContext;
use super::write_op::WriteOp;
use chrono::{DateTime, Utc};

/// Drift violation types
#[derive(Debug, Clone)]
pub enum DriftViolationType {
    WalDrift,
    TxnDrift,
    GraphDrift,
}

/// Violation severity
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriftSeverity {
    Critical,
    Medium,
    Low,
}

/// A detected drift violation
#[derive(Debug, Clone)]
pub struct DriftViolation {
    pub violation_id: String,
    pub trace_id: String,
    pub violation_type: DriftViolationType,
    pub severity: DriftSeverity,
    pub description: String,
    pub detected_at: i64, // Unix timestamp
}

impl DriftViolation {
    pub fn new(
        trace_id: String,
        violation_type: DriftViolationType,
        severity: DriftSeverity,
        description: String,
    ) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let violation_id = format!(
            "D-{}-{:x}",
            match &violation_type {
                DriftViolationType::WalDrift => "WAL",
                DriftViolationType::TxnDrift => "TXN",
                DriftViolationType::GraphDrift => "GRAPH",
            },
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let detected_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            violation_id,
            trace_id,
            violation_type,
            severity,
            description,
            detected_at,
        }
    }
}

/// DriftGate — enforces WAL/TX contract before mutations
pub struct DriftGate {
    trace_id: String,
    policy: GuardPolicy,
}

pub struct GuardPolicy {
    block_on_critical: bool,
    mark_degraded_on_medium: bool,
}

impl GuardPolicy {
    pub fn new() -> Self {
        Self {
            block_on_critical: true,
            mark_degraded_on_medium: true,
        }
    }

    pub fn should_block(&self, violations: &[DriftViolation]) -> bool {
        self.block_on_critical
            && violations
                .iter()
                .any(|v| v.severity == DriftSeverity::Critical)
    }
}

impl Default for GuardPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl DriftGate {
    pub fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            policy: GuardPolicy::new(),
        }
    }

    /// Validate a write operation BEFORE execution
    /// Returns Err if the operation violates WAL/TX contract
    pub fn validate(&self, op: &WriteOp, ctx: &TransactionContext) -> Result<(), DriftViolation> {
        self.check_wal_before_mutation(op, ctx)?;
        self.check_txn_boundary(op, ctx)?;
        Ok(())
    }

    /// Validate before COMMIT
    pub fn validate_pre_commit(&self, ctx: &TransactionContext) -> Result<(), DriftViolation> {
        // WAL segment must be open (Begin logged)
        if !ctx.wal_segment_open {
            return Err(DriftViolation::new(
                self.trace_id.clone(),
                DriftViolationType::WalDrift,
                DriftSeverity::Critical,
                "Cannot commit without open WAL segment".to_string(),
            ));
        }
        Ok(())
    }

    fn check_wal_before_mutation(
        &self,
        _op: &WriteOp,
        ctx: &TransactionContext,
    ) -> Result<(), DriftViolation> {
        // WAL begin must be logged before any mutation
        if !ctx.wal_segment_open {
            return Err(DriftViolation::new(
                self.trace_id.clone(),
                DriftViolationType::WalDrift,
                DriftSeverity::Critical,
                format!(
                    "{} without preceding WAL begin",
                    match _op {
                        WriteOp::Insert { .. } => "INSERT",
                        WriteOp::Update { .. } => "UPDATE",
                        WriteOp::Delete { .. } => "DELETE",
                    }
                ),
            ));
        }
        Ok(())
    }

    fn check_txn_boundary(
        &self,
        _op: &WriteOp,
        ctx: &TransactionContext,
    ) -> Result<(), DriftViolation> {
        // Mutation must be within active transaction
        if !ctx.is_active {
            return Err(DriftViolation::new(
                self.trace_id.clone(),
                DriftViolationType::TxnDrift,
                DriftSeverity::Critical,
                format!(
                    "{} outside active transaction",
                    match _op {
                        WriteOp::Insert { .. } => "INSERT",
                        WriteOp::Update { .. } => "UPDATE",
                        WriteOp::Delete { .. } => "DELETE",
                    }
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_drift_detected() {
        let gate = DriftGate::new("trace-1".to_string());
        let mut ctx = TransactionContext::new(1);
        // ctx.wal_segment_open = false → violation
        let op = WriteOp::Insert {
            table: "t".to_string(),
            columns: vec!["c".to_string()],
            values: vec![vec![]],
        };
        let result = gate.validate(&op, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_txn_boundary_violation() {
        let gate = DriftGate::new("trace-1".to_string());
        let mut ctx = TransactionContext::new(1);
        ctx.mark_wal_open();
        ctx.mark_committed(); // is_active = false
        let op = WriteOp::Update {
            table: "t".to_string(),
            set: vec![],
            filter: "1=1".to_string(),
        };
        let result = gate.validate(&op, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_mutation() {
        let gate = DriftGate::new("trace-1".to_string());
        let mut ctx = TransactionContext::new(1);
        ctx.mark_wal_open();
        let op = WriteOp::Delete {
            table: "t".to_string(),
            filter: "id=1".to_string(),
        };
        let result = gate.validate(&op, &ctx);
        assert!(result.is_ok());
    }
}
