use super::{
    DriftSeverity, DriftViolation, DriftViolationType, RecoveryConfidence, RecoveryPlan,
    RecoveryType,
};

pub struct RecoveryPlanner {
    trace_id: String,
}

impl RecoveryPlanner {
    pub fn new(trace_id: String) -> Self {
        Self { trace_id }
    }

    pub fn create_plan(&self, violation: &DriftViolation) -> Option<RecoveryPlan> {
        match violation.violation_type {
            DriftViolationType::WalDrift => Some(self.plan_for_wal_drift(violation)),
            DriftViolationType::TxnDrift => Some(self.plan_for_txn_drift(violation)),
            DriftViolationType::GraphDrift => Some(self.plan_for_graph_drift(violation)),
        }
    }

    fn plan_for_wal_drift(&self, violation: &DriftViolation) -> RecoveryPlan {
        let (recovery_type, confidence, steps) = match violation.severity {
            DriftSeverity::Critical => (
                RecoveryType::Rollback,
                RecoveryConfidence::High,
                vec![
                    "Abort current transaction".to_string(),
                    "Revert storage mutation".to_string(),
                    "Log rollback completion".to_string(),
                ],
            ),
            DriftSeverity::Medium => (
                RecoveryType::Patch,
                RecoveryConfidence::Medium,
                vec![
                    "Insert missing WalBegin record".to_string(),
                    "Append WalCommit after mutation".to_string(),
                    "Verify WAL chain integrity".to_string(),
                ],
            ),
            DriftSeverity::Low => (
                RecoveryType::Ignore,
                RecoveryConfidence::Low,
                vec!["Annotate graph with WAL gap warning".to_string()],
            ),
        };

        RecoveryPlan::new(
            self.trace_id.clone(),
            violation.violation_id.clone(),
            recovery_type,
            confidence,
            steps,
        )
    }

    fn plan_for_txn_drift(&self, violation: &DriftViolation) -> RecoveryPlan {
        let (recovery_type, confidence, steps) = match violation.severity {
            DriftSeverity::Critical => (
                RecoveryType::Rollback,
                RecoveryConfidence::High,
                vec![
                    "Force TxnRollback".to_string(),
                    "Clear pending locks".to_string(),
                    "Reset transaction state".to_string(),
                ],
            ),
            DriftSeverity::Medium => (
                RecoveryType::Patch,
                RecoveryConfidence::Medium,
                vec![
                    "Inject missing TxnBegin".to_string(),
                    "Align txn_id fields".to_string(),
                    "Re-verify transaction boundaries".to_string(),
                ],
            ),
            DriftSeverity::Low => (
                RecoveryType::Ignore,
                RecoveryConfidence::Low,
                vec!["Mark transaction boundary as advisory".to_string()],
            ),
        };

        RecoveryPlan::new(
            self.trace_id.clone(),
            violation.violation_id.clone(),
            recovery_type,
            confidence,
            steps,
        )
    }

    fn plan_for_graph_drift(&self, violation: &DriftViolation) -> RecoveryPlan {
        let (recovery_type, confidence, steps) = match violation.severity {
            DriftSeverity::Critical => (
                RecoveryType::Rewire,
                RecoveryConfidence::High,
                vec![
                    "Rebuild NEXT chain".to_string(),
                    "Verify CAUSES relationships".to_string(),
                    "Validate graph structural integrity".to_string(),
                ],
            ),
            DriftSeverity::Medium => (
                RecoveryType::Rewire,
                RecoveryConfidence::Medium,
                vec![
                    "Repair broken link".to_string(),
                    "Re-establish event sequence".to_string(),
                ],
            ),
            DriftSeverity::Low => (
                RecoveryType::Ignore,
                RecoveryConfidence::Low,
                vec!["Flag graph drift for manual review".to_string()],
            ),
        };

        RecoveryPlan::new(
            self.trace_id.clone(),
            violation.violation_id.clone(),
            recovery_type,
            confidence,
            steps,
        )
    }
}

pub struct ExecutionReplayEngine {
    trace_id: String,
    events: Vec<String>,
}

impl ExecutionReplayEngine {
    pub fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            events: Vec::new(),
        }
    }

    pub fn replay(&self) -> ReplayResult {
        let mut divergences = Vec::new();
        let mut last_wal = None;

        for (idx, event) in self.events.iter().enumerate() {
            match event.as_str() {
                "WalBegin" => last_wal = Some(idx),
                "WalCommit" => last_wal = None,
                "StorageMutation" => {
                    if let Some(wal_idx) = last_wal {
                        if wal_idx > idx {
                            divergences.push(format!(
                                "StorageMutation at {} preceded WalBegin at {}",
                                idx, wal_idx
                            ));
                        }
                    } else {
                        divergences.push(format!(
                            "StorageMutation at {} without open WAL segment",
                            idx
                        ));
                    }
                }
                _ => {}
            }
        }

        let valid = divergences.is_empty();
        ReplayResult {
            trace_id: self.trace_id.clone(),
            divergences,
            valid,
        }
    }

    pub fn add_event(&mut self, event_type: String) {
        self.events.push(event_type);
    }
}

#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub trace_id: String,
    pub divergences: Vec<String>,
    pub valid: bool,
}

pub struct SafeExecutionController {
    block_on_critical: bool,
    suggest_on_medium: bool,
    auto_repair_confidence_threshold: RecoveryConfidence,
}

impl SafeExecutionController {
    pub fn new() -> Self {
        Self {
            block_on_critical: true,
            suggest_on_medium: true,
            auto_repair_confidence_threshold: RecoveryConfidence::High,
        }
    }

    pub fn should_block(&self, plan: &RecoveryPlan) -> bool {
        self.block_on_critical
            && plan.recovery_type == RecoveryType::Rollback
            && plan.confidence == RecoveryConfidence::High
    }

    pub fn should_suggest(&self, plan: &RecoveryPlan) -> bool {
        self.suggest_on_medium
            && plan.recovery_type == RecoveryType::Patch
            && plan.confidence == RecoveryConfidence::Medium
    }

    pub fn should_auto_repair(&self, plan: &RecoveryPlan) -> bool {
        plan.confidence.ge(&self.auto_repair_confidence_threshold)
            && matches!(
                plan.recovery_type,
                RecoveryType::Patch | RecoveryType::Rewire
            )
    }

    pub fn execute_plan(&self, plan: &RecoveryPlan) -> ExecutionResult {
        if self.should_block(plan) {
            return ExecutionResult::Blocked {
                plan_id: plan.plan_id.clone(),
                reason: "Auto-repair blocked: Critical severity + High confidence rollback"
                    .to_string(),
            };
        }

        if self.should_auto_repair(plan) {
            return ExecutionResult::AutoRepaired {
                plan_id: plan.plan_id.clone(),
                steps_executed: plan.steps.len(),
            };
        }

        ExecutionResult::Suggested {
            plan_id: plan.plan_id.clone(),
            steps: plan.steps.clone(),
        }
    }
}

impl Default for SafeExecutionController {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Blocked {
        plan_id: String,
        reason: String,
    },
    AutoRepaired {
        plan_id: String,
        steps_executed: usize,
    },
    Suggested {
        plan_id: String,
        steps: Vec<String>,
    },
}
